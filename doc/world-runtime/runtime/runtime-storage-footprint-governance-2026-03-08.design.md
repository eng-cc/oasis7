# oasis7 Runtime：执行桥接与运行态存储体积治理（详细技术设计，2026-03-08）

审计轮次: 4

- 上游 PRD: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- 上游项目管理: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md`
- 适用范围: `oasis7_viewer_live` / `oasis7_chain_runtime` 默认运行链路下的 execution bridge、execution world sidecar、node-distfs 复制热数据与 footprint metrics。

## 1. 设计目标
- **Replayability-first**：体积治理不能破坏“可追溯、可回放”；必须明确 canonical replay source、checkpoint 语义与 GC 边界。
- **Latest-state recoverability**：任意时刻成功提交后的 latest head 必须可恢复，且恢复后 `execution_state_root` 与 GC 前一致。
- **Bounded default footprint**：默认 `dev_local` / launcher profile 不能因运行时间增长而无界膨胀。
- **Profile-governed retention**：`dev_local`、`release_default`、`soak_forensics` 通过同一套 storage profile 控制保留密度，而不是散落在脚本中的隐式常量。
- **Derived data is disposable**：快照、sidecar chunk、热提交镜像和体积型索引都是加速/诊断数据；在满足 replay contract 的前提下允许回收。

## 2. 非目标
- 不改变共识协议、区块签名语义或 DistFS challenge 业务规则。
- 不引入远端对象存储、外部数据库或跨节点中心化归档服务。
- 不要求默认开发 profile 下任意历史高度都存在本地全量 `snapshot_ref`；默认 profile 保障的是“保留范围内可回放”和“latest head 可恢复”。
- 不尝试在本轮清理 `target/`、bundle、浏览器缓存等非运行态产物。

## 3. 现状诊断
### 3.1 当前三套主要数据
- `output/chain-runtime/<node_id>/store`
  - execution bridge 的本地 CAS 仓库。
  - 目前每个 committed height 都写入一份 `snapshot_ref` 和一份 `journal_ref`。
- `output/chain-runtime/<node_id>/reward-runtime-execution-world`
  - current head 的 `snapshot.json` / `journal.json`。
  - 另含 `.distfs-state/blobs`，按 256 KiB 默认 chunk 大小写入 sidecar blob。
- `output/node-distfs/<node_id>`
  - 复制热消息、commit 镜像、`files_index.json` 和本地 blob store。

### 3.2 已观测问题
- `store` 中约 `2102` 个唯一 `snapshot_ref` 累积约 `1175.58 MiB`，而 `journal_ref` 仅约 `7.70 MiB`。
- `reward-runtime-execution-world/.distfs-state/blobs` 累积约 `635.28 MiB`，但 latest manifest + journal segments 实际只引用约 `1.55 MiB`。
- `snapshot.json` 达到 `2.2 MiB`，其中 `tick_consensus_records` 单项约 `1.67 MiB`。
- `replication_commit_messages` 目前仍是热窗口文件；只有超过默认 `4096` 条后才下沉到 cold index。

### 3.3 根因
- execution bridge 把“每高度完整快照”当作默认持久化语义，而不是“checkpoint + replay log”。
- execution world sidecar 只有写入与恢复，没有 generation-aware sweep。
- snapshot 把 `tick_consensus_records` 全量驻留在热状态，而不是热冷分层。
- 复制层的 hot/cold 保留策略与 execution bridge / sidecar 不统一，状态接口也没有直观暴露预算与实际增长。

## 4. 设计原则与回放契约
### 4.1 Canonical replay source
本方案把运行态数据分为“权威输入日志”和“派生缓存”两类：

- **权威输入日志（必须保留）**
  - `consensus/commits/<height>.json` 对应的 commit 内容与顺序。
  - 影响确定性执行的外部输入必须在 commit 前物化为 committed payload，或以“commit-height -> external-effect receipt”形式被权威引用。
  - 模块工件 hash / manifest / active version 等执行依赖。
- **派生缓存（允许回收）**
  - 每高度 `snapshot_ref` / `journal_ref`。
  - `reward-runtime-execution-world/.distfs-state/blobs` 的旧 generation chunk。
  - 热提交消息镜像、快照内热窗口外的 `tick_consensus_records`。

### 4.2 Replay contract
- **R0 Latest Recovery Contract**
  - latest successful committed height 必须总是可以直接恢复。
  - latest head 相关 manifest / snapshot / journal / module refs 永远 pin。
- **R1 Retained Replay Contract**
  - 对于 retention policy 明确保留的任意目标高度 `H`，系统必须能从“最近 checkpoint `C <= H` + canonical replay log `(C+1..H)`”重建出与原记录一致的 `execution_state_root`。
- **R2 Audit Contract**
  - 即使默认 profile 不保留每高度完整 snapshot，也必须保留足以证明 replay chain 完整性的 metadata：`height`、`execution_state_root`、`execution_block_hash`、`journal_len`、`checkpoint_anchor`、`commit_ref`。
- **R3 Fail-safe Contract**
  - 任意 GC、compaction、generation 切换失败都不得破坏 R0；若 R1 暂时不可验证，系统必须显式进入 `degraded`，而不是 silent success。

### 4.3 Profile matrix
| Profile | 目标 | 快照保留 | Checkpoint 保留 | 日志保留 | 适用场景 |
| --- | --- | --- | --- | --- | --- |
| `dev_local` | 控制磁盘增长 | latest head + 短 hot window | 稀疏 checkpoint | 保留 replay 所需 canonical log | 本地开发、启动器 |
| `release_default` | 保证长期服务恢复 | latest head + 较长 hot window | 规则化 checkpoint | 保留 replay 所需 canonical log | 单节点/发行运行 |
| `soak_forensics` | 强取证 | latest head + 长 hot window | 高密度或全量 checkpoint | 保留 replay 所需 canonical log + 更多审计输出 | 长跑、事故复盘 |

## 5. 数据分类与目录方案

### 5.1 目录角色
建议保持现有根路径不变，在其下新增清晰子角色：

```text
output/
  chain-runtime/<node_id>/
    reward-runtime-execution-world/        # latest head + sidecar
    reward-runtime-execution-records/      # 每高度 metadata index
    execution-checkpoints/                 # 稀疏 checkpoint manifest/index
    execution-archives/                    # tick consensus archive/index
    store/                                 # execution bridge CAS (仅保留 pinned blobs)
    reward-runtime-state.json
    reward-runtime-execution-bridge-state.json
    reward-runtime-storage-metrics.json
  node-distfs/<node_id>/
    replication_commit_messages/           # hot commit mirror
    replication_commit_messages_cold_index.json
    replication_commit_messages.cold-index/
      index.json
      segments/                           # fixed height-range packed cold commits
    store/                                 # distfs blobs + legacy cold commit compatibility blobs
```

### 5.2 数据分层
- `reward-runtime-execution-world/`
  - 只表示 **latest recoverable head**。
  - `snapshot.json` / `journal.json` 继续保留，供快速恢复和人工检查。
  - `.distfs-state` 只保留 latest generation 与 rollback-safe generation。
- `execution-checkpoints/`
  - 新增 checkpoint manifest/index，承载可回放的稀疏快照。
  - checkpoint 使用 execution store CAS，和 latest head 的 sidecar 分离，避免 latest directory 无限保留历史 generation。
- `reward-runtime-execution-records/`
  - 保留每高度 metadata，但不要求每条都持有唯一 snapshot/journal blob。
  - 对非 checkpoint 高度，仅保留 `checkpoint_anchor`、`commit_ref`、`execution_state_root` 等轻量索引。
- `store/`
  - 只保存 latest head、hot window snapshot/journal、checkpoint snapshot/journal、archive 索引和受保护 module artifacts 所引用的 blob。

## 6. 核心数据结构
### 6.1 StorageProfileConfig
```rust
struct StorageProfileConfig {
    profile: StorageProfile,
    execution_hot_head_heights: u64,
    execution_checkpoint_interval: u64,
    execution_checkpoint_keep: u64,
    execution_sidecar_generations_keep: u32,
    tick_consensus_hot_limit: usize,
    tick_consensus_archive_segment_len: usize,
    replication_max_hot_commit_messages: usize,
    metrics_emit_interval_ms: u64,
}
```

### 6.2 ExecutionBridgeRecordV2
```rust
struct ExecutionBridgeRecordV2 {
    world_id: String,
    height: u64,
    node_block_hash: Option<String>,
    execution_block_hash: String,
    execution_state_root: String,
    journal_len: usize,
    commit_ref: String,
    retention_class: ExecutionRetentionClass,
    checkpoint_anchor: Option<CheckpointAnchor>,
    hot_snapshot_ref: Option<String>,
    hot_journal_ref: Option<String>,
    simulator_mirror: Option<ExecutionSimulatorMirrorRecord>,
    timestamp_ms: i64,
}
```

#### 设计说明
- `commit_ref` 指向 canonical replay log 中该高度的 commit 内容（或其 hash / path）。
- `hot_snapshot_ref` / `hot_journal_ref` 仅在 latest/hot window/checkpoint 高度存在。
- 普通 archive-only 高度只保留 replay 所需轻索引，不再强制持有独占完整快照。

### 6.3 ExecutionCheckpointManifest
```rust
struct ExecutionCheckpointManifest {
    checkpoint_id: String,
    world_id: String,
    height: u64,
    snapshot_ref: String,
    journal_ref: String,
    execution_state_root: String,
    execution_block_hash: String,
    module_anchor: ModuleAnchor,
    created_at_ms: i64,
}
```

### 6.4 SidecarGenerationIndex
```rust
struct SidecarGenerationIndex {
    latest_generation: String,
    rollback_safe_generation: Option<String>,
    generations: BTreeMap<String, SidecarGenerationRecord>,
    last_gc_result: SidecarGcResult,
}

struct SidecarGenerationRecord {
    manifest_hash: String,
    journal_segment_hashes: Vec<String>,
    pinned_blob_hashes: Vec<String>,
    created_at_ms: i64,
}
```

### 6.5 TickConsensusArchiveIndex
```rust
struct TickConsensusArchiveIndex {
    hot_from_tick: u64,
    hot_to_tick: u64,
    archived_segments: Vec<TickConsensusArchiveSegment>,
}

struct TickConsensusArchiveSegment {
    from_tick: u64,
    to_tick: u64,
    content_hash: String,
    record_count: usize,
    hash_chain_anchor: String,
}
```

### 6.6 SharedColdIndexManifest
```rust
struct StorageColdIndexManifest {
    schema_version: u32,
    namespace: String,
    key_kind: String,
    value_kind: String,
    hot_range: Option<StorageColdIndexRange>,
    cold_range_anchor: Option<StorageColdIndexRangeAnchor>,
}

struct StorageColdIndexRange {
    from_key: u64,
    to_key: u64,
}

struct StorageColdIndexRangeAnchor {
    from_key: u64,
    to_key: u64,
    first_content_hash: String,
    last_content_hash: String,
    entry_count: usize,
}

struct CommitMessagePackRef {
    segment_id: String,
    offset: u64,
    len: u64,
    content_hash: String,
}
```

#### 设计说明
- 共享 cold index 目录采用 `<namespace>.cold-index/index.json`；有分段数据时统一使用同级 `segments/` 目录承载 payload。
- `hot_range` 表示当前仍保留在热路径中的连续键范围；对 replication 即 latest-based height window，对 tick/archive 则可映射为 tick range。
- `replication_commit_messages` 的冷归档默认采用固定高度跨度的 segmented pack 布局：同一高度段内的 commit message 以 `len + payload` 追加进共享 `.pack` 文件，cold index 只保留 `segment_id + offset + len + content_hash`，从而避免“小 commit 一文件”带来的 inode 与块边界放大。
- 旧 `height -> content_hash` 冷索引在第一次读回/维护时会自动迁移到 `CommitMessagePackRef`，并在 pack 索引落盘成功后删除不再被 file index / pin 引用的 legacy CAS blob。
- `cold_range_anchor` 只承载冷区边界锚点而不是全量 payload，最低要求为 `from_key` / `to_key` / `first_content_hash` / `last_content_hash` / `entry_count`，供审计、metrics 与 seek 策略共用；cold-index scan 与按 key seek 的边界必须从同一 anchor 派生。
- rollout 期间 canonical 与 legacy alias 采用双写 + 读时回填策略：若 `<namespace>.cold-index/index.json` 或旧别名缺失，读取路径会补回另一侧，避免已有样本和脚本立即失效。

### 6.7 StorageMetricsSnapshot
```rust
struct StorageMetricsSnapshot {
    storage_profile: String,
    effective_budget: StorageProfileConfig,
    bytes_by_dir: BTreeMap<String, u64>,
    blob_counts: BTreeMap<String, u64>,
    ref_count: u64,
    pin_count: u64,
    retained_heights: Vec<u64>,
    checkpoint_count: usize,
    replay_summary: StorageReplaySummary,
    orphan_blob_count: u64,
    last_gc_at_ms: Option<i64>,
    last_gc_result: String,
    last_gc_error: Option<String>,
    degraded_reason: Option<String>,
}

struct StorageReplaySummary {
    retained_height_count: usize,
    earliest_retained_height: Option<u64>,
    latest_retained_height: Option<u64>,
    earliest_checkpoint_height: Option<u64>,
    latest_checkpoint_height: Option<u64>,
    mode: String,
}
```

- `effective_budget` 直接回显当前 profile 对应的预算口径，避免 launcher / 脚本重复推导。
- `replay_summary.mode` 只允许 `latest_only` / `full_log_only` / `checkpoint_plus_log` 三档，供外部快速判断回放保证等级。

### 6.8 Bundle Wrapper Profile Contract
- `run-game.sh`、`run-web-launcher.sh`、`run-chain-runtime.sh` 只在 `OASIS7_CHAIN_STORAGE_PROFILE` 非空时注入对应 profile 参数。
- 未设置 `OASIS7_CHAIN_STORAGE_PROFILE` 时，wrapper 必须继承底层二进制默认值，避免在 shell 中复制 `dev_local` 等默认常量。
- `run-game.sh` / `run-web-launcher.sh` 必须显式指向 bundle 内 `oasis7_chain_runtime` 二进制，保证 profile 覆盖不会落到外部 PATH 上的其他 runtime。

## 7. 写路径设计
### 7.1 Commit 落账顺序
目标：保证 canonical log 先成立，再生成派生执行数据。

1. `NodeRuntime` 提交共识高度 `H`。
2. canonical commit log 持久化成功，得到 `commit_ref(H)`。
3. execution bridge 执行 commit，得到 latest `snapshot_value` / `journal_value` / `execution_state_root`。
4. 判断 `H` 是否属于：
   - latest head
   - hot window
   - sparse checkpoint
5. 写入 `ExecutionBridgeRecordV2`：
   - 所有高度都记录 `commit_ref`、`execution_state_root`、`execution_block_hash`。
   - 只有 hot/checkpoint 高度才写 `hot_snapshot_ref` / `hot_journal_ref`。
6. 更新 latest execution world 目录。
7. 若 `H` 触发 checkpoint，则写 `execution-checkpoints/<checkpoint_id>.json` 并 pin checkpoint refs。
8. 运行 retention manager 重算 pin set，并对 execution store 做 sweep。
9. 发布 metrics/state。

### 7.2 为什么不能“每高度先写完整快照”
- 现状证明每高度 snapshot 会造成 GiB 级增长。
- 对 replay 来说，真正必须的是：
  - 最近可恢复 head
  - 足以跳点的 checkpoint
  - 不可丢的 canonical replay log
- 因此完整快照必须从“默认每高度产物”降级为“按 retention class 生成的加速产物”。

## 8. Latest head 与 sidecar generation 设计
### 8.1 Latest head 目录角色
`reward-runtime-execution-world/` 只服务以下目标：
- latest-state 快速重启
- 人工查看当前 `snapshot.json` / `journal.json`
- latest generation sidecar 恢复

### 8.2 Generation 两阶段切换
1. 生成新的 manifest / journal segments / pinned blob set。
2. 写入 `generation.tmp/<id>` 元数据。
3. 校验 new generation 是否完整可读。
4. 原子更新 `latest_generation` 指针，并把旧 latest 标为 `rollback_safe_generation`。
5. sweep 所有不属于 `{latest, rollback_safe}` 且无外部 pin 的 blob。

### 8.3 Sweep 规则
- 允许删除：不属于 latest/rollback-safe generation 的 sidecar blobs。
- 禁止删除：
  - latest generation 任何 chunk
  - rollback-safe generation 任何 chunk
  - 未来扩展中被 checkpoint 明确复用的 chunk

## 9. Execution bridge retention 设计
### 9.1 Retention classes
- `LatestHead`
- `HotWindow`
- `Checkpoint`
- `ArchiveOnly`

### 9.2 Pin set 计算
每个 sweep 周期重算一次 execution store pin set：
- latest head record 的 `snapshot_ref` / `journal_ref`
- hot window 中所有 record 的 `hot_snapshot_ref` / `hot_journal_ref`
- checkpoint manifest 引用的 `snapshot_ref` / `journal_ref`
- replay archive index / tick archive segment blobs
- 恢复所需 module artifact refs

未在 pin set 中的 CAS blob 视为可回收。

### 9.3 Checkpoint 策略
- 默认每 `N` 个 committed heights 生成一个 checkpoint。
- checkpoint 间隔由 profile 决定：
  - `dev_local` 较稀疏
  - `release_default` 中等
  - `soak_forensics` 更密集
- 至少保留最近 `K` 个 checkpoint。
- 当删除旧 checkpoint 时，必须保证其之后仍存在更近的保留 checkpoint，且 replay contract 仍可覆盖保留窗口。

## 10. Replay 设计
### 10.1 指定高度重建
输入：目标高度 `H`

1. 在 checkpoint index 中找到最近的 `C <= H`。
2. 加载 checkpoint snapshot/journal/module anchor。
3. 从 canonical replay log 读取 `(C+1..H)` 对应 commits。
4. 使用确定性执行器顺序执行。
5. 将重建得到的 `execution_state_root` 与 `execution_records/H` 中记录值对比。
6. 匹配则返回 success；否则进入 mismatch fault。

### 10.2 无 checkpoint 情况
- 若 `H` 在 hot window 内，可直接使用最近 hot snapshot 作为 replay 起点。
- 若 `H` 不在 hot window 且无 checkpoint，视为违反 replay contract，GC 不允许发生到该状态。

### 10.3 外部非确定性约束
若某些效果当前未完全体现在 committed payload 中，则必须在实现前明确补齐。否则 replay 只能得到“结构上可重放”而不能保证 bit-identical state。必须逐项核对：
- LLM 决策是否已经在 commit 前固定成 action payload
- 模块版本/工件 hash 是否在 replay 时可唯一解析
- 随机种子/时钟依赖是否已包含在 block / commit 上下文

## 11. Tick consensus 热冷分层
### 11.1 目标
控制 `snapshot.json` 中 `tick_consensus_records` 的体积，同时保留链路验证与审计能力。

### 11.2 方案
- `snapshot.json` 仅保留最近 `tick_consensus_hot_limit` 条 record。
- 热窗口外 records 被编码为 archive segment，写入 execution store 或 archive 目录。
- `TickConsensusArchiveIndex` 记录 tick 范围、content hash 与 hash chain anchor。
- 审计读取按区间查询 archive segment，再验证 segment hash 与 anchor。

### 11.3 结果
- latest snapshot 只保留热记录，显著缩小 `snapshot.json`。
- 历史链路验证不依赖热内存常驻集合。

## 12. Metrics / 状态输出设计
### 12.1 输出位置
- `reward-runtime-storage-metrics.json`
- chain status API 新增 storage section
- launcher / soak 脚本直接读取上述字段

### 12.2 最低字段
- `storage_profile`
- `bytes_by_dir`
- `blob_counts`
- `ref_count`
- `pin_count`
- `retained_heights`
- `checkpoint_count`
- `effective_budget`
- `replay_summary`
- `orphan_blob_count`
- `last_gc_at_ms`
- `last_gc_result`
- `last_gc_error`
- `degraded_reason`

### 12.3 使用方式
- 开发者快速判断为什么涨体积。
- soak profile 可以把“保留策略导致的正常增长”和“孤儿泄漏”区分开。
- required/full 测试直接断言预算与错误状态。

## 13. 失败语义与恢复策略
### 13.1 GC 失败
- 任何 pin set 构建失败、引用校验失败、latest ref 缺失：
  - 停止 sweep
  - 写 `last_gc_result=failed`
  - 写 `degraded_reason`
  - 保留现有 pinned + unknown 数据，不冒进删除

### 13.2 Save 中断
- latest generation 切换必须两阶段提交。
- 若在 `.tmp` generation 阶段退出，重启后清理未提交 generation，但不得影响旧 latest。

### 13.3 Replay mismatch
- 若 checkpoint + canonical log 重建出的 `execution_state_root` 与记录不一致：
  - status 标记 `replay_contract_broken`
  - 阻断进一步 aggressive GC
  - 在 `test_tier_full` 与 release gate 中作为硬阻断

## 14. 迁移方案
### 14.1 向后兼容
- 旧 `ExecutionBridgeRecord` 仍可读；迁移时若缺 `commit_ref` / `retention_class`，按 legacy 模式处理。
- legacy 模式下禁止 aggressive GC，只允许 latest-safe sweep。

### 14.2 渐进迁移步骤
1. 先落 storage profile / metrics，不改变存储语义。
2. 再引入 sidecar generation pin/sweep。
3. 再把 execution bridge 改为“checkpoint + hot window”。
4. 最后引入 `tick_consensus_records` archive index。

### 14.3 回退策略
- 任一步失败都可回退到“保留更多数据、不删历史”的安全模式。
- 回退不允许改变 canonical log 结构。

## 15. 代码落点建议
- `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
  - `ExecutionBridgeRecordV2`
  - checkpoint 写入逻辑
  - pin set 计算与 retention manager
- `crates/oasis7/src/runtime/world/persistence.rs`
  - sidecar generation index
  - manifest-aware sweep
- `crates/oasis7/src/runtime/snapshot.rs`
  - `tick_consensus_records` 热冷分层索引字段
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
  - storage profile 配置与 metrics 发布
- `crates/oasis7_node/src/replication.rs`
  - hot/cold commit mirror 与 storage profile 对齐
- `testing-manual.md`
  - footprint / replay contract / GC fail-safe 验收矩阵

## 16. 测试设计
### 16.1 `test_tier_required`
- `2500` heights 样本下默认 profile 体积预算验证。
- latest-state restart 后 `execution_state_root` 一致。
- sidecar generation sweep 后 orphan blob 数量为 `0`。
- retained target height 可由 checkpoint + canonical log 重建。
- `tick_consensus_records` archive read + verify 成功。
- replication cold-index scan / seek 与 tick archive range seek 在边界口径上保持一致。

### 16.2 `test_tier_full`
- GC 中断/部分写入注入，验证 fail-safe。
- profile 切换：`dev_local -> soak_forensics -> dev_local`。
- replay mismatch 注入，验证系统进入 `degraded` 并阻断 aggressive GC。
- 长跑脚本对 metrics / budget gate 的联合验证。

## 17. 开放问题
- canonical commit log 当前是否已完整承载所有外部非确定性结果；若没有，需要先补“external effect ledger”再做 aggressive retention。
- checkpoint 是只保 latest `snapshot_ref + journal_ref` 还是同时保 module anchor 的完整 manifest；当前设计建议必须带 module anchor。
- `tick_consensus_records` archive segment 是放 execution store 还是 node-distfs store；当前建议优先放 execution store，避免跨子系统依赖扩大。

## 18. 结论
- 要同时满足“默认体积可控”和“必须可追溯可回放”，唯一可持续方案不是永久保留每高度完整快照，而是：
  - **canonical log 为真**
  - **checkpoint 为跳点**
  - **latest head 为快速恢复缓存**
  - **GC 只删除不被 replay contract 引用的数据**
- 这份设计将把当前的“快照主导存储”收敛为“日志主导、快照加速”的运行态持久化体系。
