# oasis7 Runtime：执行桥接与运行态存储体积治理（2026-03-08）

- 对应设计文档: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
- 对应项目管理文档: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md`

审计轮次: 4

- 详细技术设计文档: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`

## 1. Executive Summary
- Problem Statement: 当前 `oasis7_viewer_live` / `oasis7_chain_runtime` 的默认运行态持久化在短时本地闭环中也会产生明显的磁盘膨胀：一次约 `2102` 高度的运行样本中，`output/chain-runtime/viewer-live-node/store` 约 `1.18 GiB`、`reward-runtime-execution-world/.distfs-state/blobs` 约 `635 MiB`，而当前最新执行世界实际只引用约 `1.55 MiB` 的 sidecar blob。执行桥接按高度保留全量 `snapshot_ref`、执行世界 sidecar 缺少引用回收、`snapshot.json` 中 `tick_consensus_records` 持续增长，导致本地调试、重复启动与长跑验证的磁盘成本过高。
- Proposed Solution: 为 runtime 引入“canonical replay log + checkpoint + 分层保留 + 引用治理 + 指标可观测”方案：把运行态数据拆分为当前可恢复 head、热历史窗口、稀疏检查点、权威变更日志、冷元数据与可回收孤儿 blob 六类数据，分别实施 retention / compaction / GC；同时对 `tick_consensus_records` 做热冷分层，明确“日志为真、快照为缓存”的回放契约，在控制默认磁盘占用的同时保证可追溯与可回放。
- Proposed Solution 补充（2026-05-15）: 在不改变 `content_hash` / `snapshot_ref` / `journal_ref` 合同的前提下，`LocalCasStore` 允许对大体积 blob 采用“磁盘透明压缩、读出仍返回原始 bytes”的落盘策略，优先压缩 execution bridge snapshots/journals 与 sidecar segments 的重复结构数据。
- Success Criteria:
  - SC-1: 默认 `dev_local` / launcher profile 在 `llm_bootstrap` 场景连续运行到 `2500` committed heights 后，`output/chain-runtime/<node_id>/store` 稳态占用 `<= 256 MiB`。
  - SC-2: `reward-runtime-execution-world/.distfs-state/blobs` 在每次成功保存后仅保留被当前有效 manifest / journal segments / 受保护 generation 引用的 blob；`2500` heights 样本下总占用 `<= 16 MiB`，孤儿 blob 数量为 `0`。
  - SC-3: 默认 profile 下 `reward-runtime-execution-world/snapshot.json` 在 `2500` heights 样本下 `<= 512 KiB`，且 `tick_consensus_records` 热窗口外的数据仍可经归档索引审计读取。
  - SC-4: 在 retention / GC 执行后，latest-state 重启恢复成功率 `100%`，恢复后的 `execution_state_root` 与 GC 前一致。
  - SC-5: status/metrics 输出必须包含当前 storage profile、各目录字节数、pin 集大小、最近一次 GC 结果与失败原因，供 launcher / soak / release gate 直接消费。
  - SC-6: 对 retention policy 明确保留的任意目标高度 `H`，系统必须能从最近 checkpoint + canonical replay log 重建出与原记录一致的 `execution_state_root`。
  - SC-7: `release_default` 必须把 `execution_hot_head_heights` 收敛到不高于 `execution_checkpoint_interval`；在 exact-height restore 仍依赖热窗口快照的前提下，默认发布档位不得再为同一 replay 区间重复保留额外一倍的 execution snapshots。

## 2. User Experience & Functionality
- User Personas:
  - Runtime 维护者：希望默认运行态可长期调试，不因历史快照无限增长而频繁手工清目录。
  - QA / 长跑维护者：希望根据 `dev_local` / `release_default` / `soak_forensics` 选择不同保留策略，同时不破坏恢复和审计。
  - 发布工程师：希望启动器与链运行时有可预测的磁盘预算，并能在 status/报表中看到增长来源。
  - 安全/审计评审者：希望 GC 只删除未引用历史，不删除当前 head 或仍被审计链路依赖的数据。
- User Scenarios & Frequency:
  - 本地启动器闭环：开发者每天多次执行，要求默认 profile 不出现无界磁盘增长。
  - CI / required-tier 回归：每次 runtime 存储链路改动都要验证 footprint、重启恢复和引用完整性。
  - Soak / forensic 长跑：按周或发布前运行，允许更高占用，但必须显式切换 retention profile。
  - 事故复盘：低频但关键，需要在 head window 或归档 checkpoint 范围内快速定位指定高度。
- User Stories:
  - PRD-WORLD_RUNTIME-013: As a Runtime 维护者, I want execution-bridge snapshots and sidecar blobs to follow bounded retention with explicit pin sets, so that default runs do not grow disk usage without limit.
  - PRD-WORLD_RUNTIME-014: As a QA/审计维护者, I want latest-state recovery and checkpointed replay evidence to survive GC, so that volume reduction does not break restart, diagnosis, or auditability.
  - PRD-WORLD_RUNTIME-015: As a 发布工程师, I want profile-based storage policies and metrics surfaced through runtime status, so that dev/release/soak environments can enforce different disk budgets predictably.
- Critical User Flows:
  1. Flow-RSF-001（默认开发 profile 持久化）:
     `committed height 推进 -> execution_bridge 写 latest snapshot/journal refs -> retention manager 更新 head pin set -> 清理不再被保留窗口引用的 CAS blob -> status 输出最新 storage metrics`。
  2. Flow-RSF-002（执行世界 sidecar 保存）:
     `RuntimeWorld::save_to_dir -> 生成 snapshot.manifest/journal.segments -> 写入新 generation pin set -> manifest-aware sweep 删除旧 generation 孤儿 blob -> 保留 latest + rollback-safe generation`。
  3. Flow-RSF-003（重启恢复）:
     `进程重启 -> 读取 latest execution bridge state + execution world head -> 验证被 pin 的 snapshot/journal/manifest refs -> 恢复 runtime -> 对比恢复前后 state_root`。
  4. Flow-RSF-004（soak / forensic 模式）:
     `运维选择 soak_forensics profile -> 放宽 hot window / checkpoint 保留 -> 长跑完成后导出 metrics/report -> 按 profile 关闭或回到 dev_local`。
  5. Flow-RSF-005（快照内链路压缩）:
     `tick_consensus_records 超过热窗口 -> 旧 records 写入 archive segment / index -> snapshot.json 仅保留热窗口 + archive anchor -> 审计查询按需读取归档段`。
  6. Flow-RSF-006（指定高度回放）:
     `选择目标高度 H -> 定位最近 checkpoint C -> 加载 checkpoint state -> 回放 canonical replay log (C+1..H) -> 对比 execution_state_root -> 输出 replay 成功/失败结论`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Storage profile 选择 | `storage_profile`, `execution_hot_head_heights`, `execution_checkpoint_interval`, `sidecar_generations_keep`, `tick_consensus_hot_limit` | runtime / launcher / script 启动时加载统一 profile 枚举，可由 CLI/config 显式覆盖，默认 `dev_local` | `configured -> active -> degraded/failed` | 显式参数覆盖 profile 默认值；非法组合直接拒绝启动；`release_default` 默认要求 `execution_hot_head_heights <= execution_checkpoint_interval`，避免在 checkpoint 已覆盖的区间内重复保留整份 execution snapshot | 仅启动参数/配置维护者可放宽预算 |
| Execution bridge retention | `snapshot_ref`, `journal_ref`, `height`, `retention_class=head/checkpoint/archive` | 每个高度写 record 后重算 pin set，并对 unpinned blob 执行 sweep | `written -> pinned -> pruned/archived` | latest 高度永远保留；checkpoint 按高度升序选取；journal ref 仅按被引用集合保留 | 自动执行，无用户绕过入口 |
| CAS blob 落盘压缩 | `content_hash`, `blob_encoding`, `raw_size_bytes`, `stored_size_bytes` | 大体积 blob 写入 CAS 时可透明压缩；读路径始终返回原始 bytes，hash 校验仍基于原始 payload | `raw -> compressed-on-disk` 或 `raw -> raw-on-disk` | 仅当压缩后连同 header 都小于原始 bytes 时才启用；`content_hash` 始终保持原始 payload hash，不因磁盘编码变化 | 自动执行；调用方无感知 |
| Sidecar manifest GC | `generation_id`, `manifest_hash`, `journal_segment_hashes`, `pinned_blob_hashes` | world 保存成功后执行两阶段 pin/sweep；失败则回退到旧 generation | `saving -> committed -> swept` 或 `saving -> rollback` | 仅清理未被 latest/rollback generation 引用的 blob | 自动执行；仅 runtime 自身可写 |
| Tick consensus compaction | `tick_consensus_hot_limit`, `archive_segment_size`, `archive_index` | 超出热窗口时把旧记录转入 archive 段并更新索引 | `hot -> archived` | 热窗口按最新 tick 保留；archive segment 按 tick 连续范围分段；archive seek 必须能按 `from_tick..to_tick` 读回 | 自动执行；审计只读 |
| Replay contract | `commit_ref`, `checkpoint_anchor`, `retained_heights`, `replay_profile` | 指定高度回放时从最近 checkpoint + canonical log 重建目标状态 | `requested -> replaying -> matched/mismatched` | 仅对 retention policy 保留的高度提供强保证；重建结果以 `execution_state_root` 校验 | 自动执行；测试/审计只读 |
| Metrics / status 输出 | `bytes_by_dir`, `blob_counts`, `ref_count`, `pin_count`, `orphan_blob_count`, `last_gc_at_ms`, `last_gc_result`, `last_gc_error`, `degraded_reason`, `storage_profile`, `effective_budget`, `replay_summary` | status API / state file 输出指标；异常时给出原因，launcher / 脚本无需扫描内部目录即可判断预算与回放能力 | `collected -> published` | 字节统计按目录聚合；GC 结果按时间覆盖 latest 并写审计历史；`replay_summary.mode` 仅允许 `latest_only` / `full_log_only` / `checkpoint_plus_log` | status 只读；配置修改受启动权限控制 |
| Replication hot/cold retention 对齐 | `max_hot_commit_messages`, `cold_index`, `archive_bytes` | 超出热窗口后迁移到冷索引/对象存储，并更新 metrics | `hot -> cold-indexed` | 热窗口按最新高度回推的连续高度范围定义，不再按“最近 N 个现存文件”解释；冷索引统一走 `<namespace>.cold-index/index.json` 元数据协议，并以 `from_key/to_key + first/last_content_hash + entry_count` 表示 cold range anchor；cold-index scan 与按高度 seek 必须共享同一边界语义；rollout 期间若 canonical/legacy 任一缺失，读路径会回填另一侧别名 | 自动执行；策略由 profile 决定 |
- Acceptance Criteria:
  - AC-1 (PRD-WORLD_RUNTIME-013): `oasis7_viewer_live` / `oasis7_chain_runtime` 默认开发 profile 在 `2500` heights 样本下，`output/chain-runtime/<node_id>/store <= 256 MiB`，且 CAS sweep 后不存在“record 仍引用、blob 已删除”的 dangling refs。
  - AC-2 (PRD-WORLD_RUNTIME-013): `reward-runtime-execution-world/.distfs-state/blobs` 经过 successful save 后仅保留 latest + rollback-safe generation 的引用集合；测试中孤儿 blob 计数为 `0`。
  - AC-3 (PRD-WORLD_RUNTIME-014): 在 execution-bridge retention 与 sidecar GC 生效后，latest-state restart 恢复成功，恢复后 `execution_state_root`、`journal_len`、`module_registry` 与 GC 前一致。
  - AC-4 (PRD-WORLD_RUNTIME-014): 任意 GC 中断、部分写入或 pin set 构建失败都不得删除 latest generation；系统必须进入 `degraded` 并保留恢复所需数据。
  - AC-5 (PRD-WORLD_RUNTIME-014): 对 retention policy 明确保留的任意目标高度 `H`，系统必须能从最近 checkpoint + canonical replay log 重建出与 `execution_records/H` 一致的 `execution_state_root`。
  - AC-6 (PRD-WORLD_RUNTIME-015): status / metrics 输出必须暴露 `storage_profile`、`effective_budget`、`bytes_by_dir`、`retained_heights`、`checkpoint_count`、`replay_summary`、`last_gc_result`、`last_gc_error`、`degraded_reason`，并可被 launcher 或脚本直接采样，无需额外扫描内部目录。
  - AC-6.1 (PRD-WORLD_RUNTIME-015): `oasis7_chain_runtime`、`oasis7_game_launcher`、`oasis7_web_launcher` 与 launcher UI 必须暴露同名 profile 枚举输入，并沿进程边界无损透传到链运行时。bundle 入口 `run-game.sh`、`run-web-launcher.sh`、`run-chain-runtime.sh` 必须共享同一 `OASIS7_CHAIN_STORAGE_PROFILE` 覆盖通道，且未设置时继承各自二进制默认值，不在 shell 中重复硬编码默认 profile。
  - AC-7 (PRD-WORLD_RUNTIME-015): `snapshot.json` 在 `2500` heights 样本下 `<= 512 KiB`；旧 `tick_consensus_records` 通过 archive index 可按区间读取并通过链路校验。
  - AC-8 (PRD-WORLD_RUNTIME-013/014/015): required/full 测试矩阵中必须包含 footprint 上限、restart recovery、GC fail-safe、profile 切换、archive 读取与 retained-height replay 回归。
- Non-Goals:
  - 不修改共识协议、区块语义或 DistFS challenge 计费语义。
  - 不对 `target/`、打包产物或 third-party 工具链缓存做统一清理策略。
  - 不在本阶段引入远端对象存储、外部数据库或跨节点历史归档服务。
  - 不保证默认开发 profile 可随机访问任意历史高度的完整 `snapshot_ref`；默认 profile 对超出热窗口的历史高度以“checkpoint + canonical replay log”保证重建能力，超出 retention policy 的深历史完整取证能力由 `soak_forensics` profile 承担。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不引入新的 AI 推理链路）。
- Evaluation Strategy: 以磁盘占用、恢复一致性、GC 引用完整性、状态接口可观测性为主，避免使用主观评价指标。

## 4. Technical Specifications
- Architecture Overview:
  - 当前运行态实际形成三套相互关联的数据：
    1. `execution_bridge` 的高度级 CAS 历史（`output/chain-runtime/<node_id>/store`）。
    2. `execution_world` 当前 head + `.distfs-state` sidecar（`reward-runtime-execution-world`）。
    3. `node-distfs` 的复制热数据与冷索引（`output/node-distfs/<node_id>`）。
  - 2026-03-08 的本地样本显示：约 `2102` heights 时，`store` 为 `1.18 GiB`、`reward-runtime-execution-world/.distfs-state/blobs` 为 `635 MiB`、`snapshot.json` 为 `2.2 MiB`，其中 `tick_consensus_records` 单项约 `1.67 MiB`；而当前 sidecar 最新 manifest 实际只引用约 `1.55 MiB` 的 blob。这说明问题主要来自“历史版本无限保留”与“当前 sidecar 缺少 sweep”。
  - 目标架构明确采用“canonical replay log 为真、checkpoint 为跳点、latest snapshot 为缓存”的持久化层次，把数据划分为六类：
    - `Recoverable Head`: latest state 恢复必需数据，永远 pin。
    - `Hot Window`: 最近连续若干高度的完整历史，供调试和短回放。
    - `Sparse Checkpoints`: 低密度保留的历史快照，作为回放跳点。
    - `Canonical Replay Log`: 可重建 retained heights 的权威输入日志，GC 不得破坏。
    - `Cold Metadata`: 小体积索引/记录，允许长期保留。
    - `Orphan / Unpinned Data`: 不再被任一有效 generation / retention class 引用的数据，允许 GC。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_viewer_live.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
  - `crates/oasis7/src/runtime/world/persistence.rs`
  - `crates/oasis7/src/runtime/snapshot.rs`
  - `crates/oasis7_node/src/replication.rs`
  - `crates/oasis7_distfs/src/lib.rs`
  - `doc/world-runtime/runtime/runtime-integration.md`
  - `doc/world-runtime/module/module-storage.prd.md`
- Edge Cases & Error Handling:
  - 保存新 generation 时磁盘写满：必须中止切换、保留旧 generation、标记 `last_gc_result=save_failed`。
  - GC 扫描过程中发现 latest ref 缺失：立即停止 sweep，状态置为 `degraded`，禁止继续删除。
  - execution record、checkpoint 或 replay log 引用已被外部手工删除：重启或指定高度回放时必须给出结构化错误，并在开发 profile 下允许“仅 latest state 恢复 / retained-height replay 不可用”的显式降级模式，不得静默成功。
  - profile 从 `soak_forensics` 切回 `dev_local`：必须执行一次完整重算 pin set，再按新策略裁剪。
  - `tick_consensus_records` 归档索引损坏：不得影响 latest state 启动，但审计查询返回结构化错误并要求修复。
  - `replication_commit_messages` 热数据降冷时失败：保留热文件，不得先删后写。
  - 多进程误指向同一 `output/chain-runtime/<node_id>`：保留现有单 writer 假设，本阶段仅要求检测并阻止第二写者，不负责多写者并发协调。
- Non-Functional Requirements:
  - NFR-1: `dev_local` / launcher 默认 profile 在 `2500` heights 样本下，`output/chain-runtime/<node_id>` 总占用 `<= 384 MiB`，其中 `store <= 256 MiB`。
  - NFR-2: sidecar manifest-aware GC 的单次 sweep `p95 <= 500 ms`（`2500` heights 样本、本地 SSD 环境）。
  - NFR-3: latest-state restart 在默认 profile 下 `p95 <= 5 s`。
  - NFR-4: footprint 优化不得降低回放/恢复确定性；相关一致性回归必须保持 `100%` 通过。
  - NFR-5: retention policy 保留范围内的目标高度回放成功率必须为 `100%`，且重建结果 `execution_state_root` 与原记录一致。
  - NFR-6: storage metrics 采集与发布频率不高于每 `1 s` 一次，额外 CPU 开销 `<= 5%`（本地单节点样本）。
  - NFR-7: `soak_forensics` profile 明确允许更高磁盘占用，但必须通过 metrics 暴露实际增长并可配置上限。
- Security & Privacy:
  - GC 只能在 runtime 自己的 `output/chain-runtime/<node_id>`、`output/node-distfs/<node_id>` 根下运行，不得支持任意路径删除。
  - 删除决策必须基于显式 pin 集和 manifest/record 引用计算，不允许“按文件时间猜测删除”。
  - 与恢复相关的 latest generation / checkpoint 必须在原子切换完成前保持可读，避免中间态造成不可恢复损坏。
  - metrics/status 仅暴露体积、引用、结果与错误，不新增敏感 payload 导出。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (2026-03): 引入 storage profile、execution-world sidecar manifest-aware GC、基础 metrics 输出，先解决 latest head 与孤儿 blob 无界增长。
  - v1.1: 实现 execution bridge head window + sparse checkpoint retention，减少 `store` 历史 snapshot 累积。
  - v2.0: 完成 `tick_consensus_records` 热冷分层与 archive index，建立 launcher/soak 统一 footprint gate。
- Technical Risks:
  - 风险-1: 过于激进的 retention 导致历史 record 仍存在但其 `snapshot_ref` 已不可读。
    - 缓解：先定义显式 retention class，再用 pin-set sweep；dangling-ref 测试列为 required。
  - 风险-2: sidecar GC 与 save_to_dir 原子性不足，可能在异常退出后留下不可恢复状态。
    - 缓解：采用 generation pin + rollback-safe keep=2 的两阶段切换。
  - 风险-3: `tick_consensus_records` 压缩破坏链路校验与审计查询。
    - 缓解：保留 archive anchor/hash chain，验证 archive read + verify 路径。
  - 风险-4: 默认 profile 与 soak profile 口径分裂，导致脚本/launcher/运行时行为不一致。
    - 缓解：用统一 `storage_profile` 枚举和 status 输出；脚本/launcher 只透传，不自定义语义。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_RUNTIME-013 | TASK-WORLD_RUNTIME-030/031/032 | `test_tier_required` | 构造 `>= 2500` heights 的 execution-bridge 样本，验证 `store`/sidecar 体积上限、pin set 完整性、无 dangling refs | execution bridge、world persistence、CAS blob retention |
| PRD-WORLD_RUNTIME-014 | TASK-WORLD_RUNTIME-030/031/032/033 | `test_tier_required` + `test_tier_full` | latest-state restart、checkpoint replay、GC fail-safe 注入、保留高度回放校验、soak profile 恢复验证 | 重启恢复、审计与长跑取证 |
| PRD-WORLD_RUNTIME-015 | TASK-WORLD_RUNTIME-031/032/033 | `test_tier_required` | status/metrics 输出字段测试、profile 参数透传测试、snapshot size regression、archive 读取验证 | 可观测性、launcher/脚本配置与治理口径 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RSF-001 | 默认 profile 仅保留 latest head + 热窗口 + 稀疏 checkpoint | 每个 committed height 永久保留完整 snapshot/journal refs | 当前本地样本证明全量历史保留会在短时运行中产生 GiB 级膨胀，不适合作为默认链路。 |
| DEC-RSF-002 | sidecar 采用 manifest-aware pin/sweep | 每次 save 前清空 `.distfs-state` 目录重写 | 直接清空在异常退出时有更高恢复风险，也无法保证 rollback-safe 恢复。 |
| DEC-RSF-003 | `tick_consensus_records` 热冷分层并保留 archive anchor | 直接禁止记录 tick consensus 历史 | 会损失审计与链路验证能力，不符合 runtime 可追溯目标。 |
| DEC-RSF-004 | 采用 profile-based storage policy（`dev_local` / `release_default` / `soak_forensics`） | 单一全局 retention 常量 | 开发、启动器、长跑取证三类环境的预算差异显著，需要明确可观测的模式治理。 |
| DEC-RSF-005 | canonical replay source 采用变更日志（commit log）+ checkpoint，snapshot 仅作为加速缓存 | 继续把每高度完整 snapshot 作为默认真相源 | 现有样本证明“每高度全量快照”体积不可控；日志为真更符合可回放要求。 |
