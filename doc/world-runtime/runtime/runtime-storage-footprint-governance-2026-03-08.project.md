# oasis7 Runtime：执行桥接与运行态存储体积治理（项目管理文档）

- 对应设计文档: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
### T0 建档与文档树接线
- [x] T0 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 新建专题 PRD / project，并回写 `doc/world-runtime/prd.md`、`doc/world-runtime/project.md`、`doc/world-runtime/prd.index.md` 的映射关系。
- [x] T0.1 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 输出详细技术设计文档 `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`，明确 canonical replay log / checkpoint / GC / metrics / migration 方案。

### T1 Canonical replay contract
- [x] T1.1 (PRD-WORLD_RUNTIME-014) [test_tier_required]: 定义 `ExecutionBridgeRecordV2` 持久化字段与兼容读取策略，明确 `commit_log_ref` / `checkpoint_ref` / `latest_state_ref` / `external_effect_ref` 的角色边界。
- [x] T1.2 (PRD-WORLD_RUNTIME-014) [test_tier_required]: 定义 `ExecutionCheckpointManifest` 目录布局、pin 语义、hash/height 校验字段与 reader / writer 原子切换规则。
- [x] T1.3 (PRD-WORLD_RUNTIME-014) [test_tier_required]: 在 replay planner 中实现“最近 checkpoint + commit log”重建路径，并显式处理无 checkpoint 的全日志回放降级。
- [x] T1.4 (PRD-WORLD_RUNTIME-014) [test_tier_required]: 明确外部非确定性 effect 的 materialization contract，保证 replay 输入可闭包且 mismatch 时 fail-closed。
- [x] T1.5 (PRD-WORLD_RUNTIME-014) [test_tier_required]: 补齐 retained-height replay / no-checkpoint fallback / replay mismatch / checkpoint corruption 定向测试。

### T2 Execution bridge retention
- [x] T2.1 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 在 `execution_bridge.rs` 实现 latest head + hot window pin set 计算，不再为每个 committed height 默认固定完整 `snapshot_ref`。
- [x] T2.2 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 落地 sparse checkpoint cadence 与 pin set 计算，保证稀疏高度可直接跳转恢复。
- [x] T2.3 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 基于显式 pin set sweep 历史 `snapshot_ref` / `journal_ref`，删除后不得留下 dangling refs。
- [x] T2.4 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 完成 `ExecutionBridgeRecordV1 -> V2` 向后兼容读取与渐进迁移，保证旧样本可读且不强制一次性重写。
- [x] T2.5 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 补齐 head-window retention、稀疏 checkpoint、restart recovery、dangling-ref 拒绝回归测试。

### T3 Sidecar generation GC
- [x] T3.1 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 定义 `SidecarGenerationIndex` 目录布局、manifest 字段与 generation pin 集，区分 staging / latest / rollback-safe generation。
- [x] T3.2 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 在 `save_to_dir` 落地两阶段 generation 切换，确保 latest generation 原子更新且最少保留 `keep=2`。
- [x] T3.3 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 实现 manifest-aware sweep，successful save 后孤儿 blob 数量为 `0`，失败时不得删除仍被 latest/rollback generation 引用的数据。
- [x] T3.4 (PRD-WORLD_RUNTIME-013/014) [test_tier_required]: 补齐 save 中断、manifest 部分写入、rollback 恢复与 orphan cleanup 故障注入测试。

### T4 Tick consensus 热冷分层
- [x] T4.1 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 将 `tick_consensus_records` 从热快照拆分为热摘要 + 冷归档，控制 `snapshot.json` 默认体积。
- [x] T4.2 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 定义 `TickConsensusArchiveIndex` 的 anchor / hash chain / range metadata，保证审计与校验可顺序读取。
- [x] T4.3 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 落地 archive read / verify 路径与旧快照迁移逻辑，保证冷热分层后查询与验证语义不变。
- [x] T4.4 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 补齐 snapshot size regression、archive read、hash verify、旧样本迁移回归测试。

### T5 冷数据索引语义收敛
- [x] T5.1 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 收敛 `execution_records` 与 `replication_commit_messages` 的热/冷窗口口径，统一“热窗口 + 稀疏冷索引 + 归档读回”语义。
- [x] T5.2 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 定义共享 cold index 命名、目录布局、元数据字段与 range anchor 规则，避免不同子系统各自发明目录协议。
- [x] T5.3 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 完成旧目录布局兼容读取 / 别名迁移，保证已有样本与工具脚本不立即失效。
- [x] T5.4 (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 补齐 cold-index scan、archive seek、跨模块读回一致性回归测试。

### T6 Metrics / profile / launcher 透传
- [x] T6.1 (PRD-WORLD_RUNTIME-015) [test_tier_required]: 落地 `StorageProfileConfig` 解析、默认值与 runtime / launcher / script 的统一透传入口。
- [x] T6.2 (PRD-WORLD_RUNTIME-015) [test_tier_required]: 在 runtime status / state file 中输出 `StorageMetricsSnapshot`，覆盖 bytes、ref_count、pin_count、checkpoint_count、orphan_count、gc_last_result 等最低字段。
- [x] T6.3 (PRD-WORLD_RUNTIME-015) [test_tier_required]: 补齐 GC 最近结果、失败原因、profile、生效预算与回放能力摘要字段，保证脚本/launcher 无需读取内部目录即可判断状态。
- [x] T6.4 (PRD-WORLD_RUNTIME-015) [test_tier_required]: 对齐 launcher、`run-web-launcher.sh`、chain runtime 启动脚本的 profile 参数透传与默认口径，避免环境间语义漂移。
- [x] T6.5 (PRD-WORLD_RUNTIME-015) [test_tier_required]: 补齐 profile 参数、status 输出、错误字段、launcher 透传的定向测试。

### T7 Footprint gate / 回归 / 收口
- [x] T7.1 (PRD-WORLD_RUNTIME-014/015) [test_tier_full]: 构造 `>= 2500` heights 的可复现实验样本，作为 footprint gate 与 replay regression 的统一输入基线。
- [ ] T7.2 (PRD-WORLD_RUNTIME-014/015) [test_tier_required]: 建立默认 profile 的体积预算、restart recovery、retained-height replay gate，并输出失败时的目录/指标差异。
- [x] T7.3 (PRD-WORLD_RUNTIME-014/015) [test_tier_full]: 建立 GC fail-safe、profile 切换、archive read、checkpoint corruption、replay mismatch 的全量回归套件。
- [x] T7.4 (PRD-WORLD_RUNTIME-014/015) [test_tier_full]: 对接 launcher / chain runtime / soak 场景，验证 `dev_local`、`release_default`、`soak_forensics` 三档 profile 口径一致。
- [x] T7.5 (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 回写专题 PRD / project、模块项目文档、`testing-manual.md`（如测试入口变化）与 `doc/devlog/2026-03-08.md`，归档体积对比与回放验证结论。

### T8 Replication footprint follow-up
- [x] replication-storage-footprint-optimization (PRD-WORLD_RUNTIME-013/015) [test_tier_required]: 为 `node-distfs` replication store 增加 cold-index-aware orphan sweep，将 `files_index` / `replication_commit_messages` / cold-index 落盘切到 compact JSON，并把冷 commit 归档从“一块一文件”收敛为 segmented pack + offset 索引，减少 legacy orphan blob、inode 数和 pretty-json 块膨胀。 Trace: .pm/tasks/task_2aa685ee43244129b35535bea1f47fed.yaml
- [x] release-default-hot-window-budget-tuning (PRD-WORLD_RUNTIME-013/014/015) [test_tier_required]: 基于真实 triad 样本确认默认磁盘占用主要由 execution hot snapshots 主导，在 exact-height restore 仍依赖热窗口快照的前提下，将 `release_default.execution_hot_head_heights` 从 `128` 收紧到与 checkpoint cadence 对齐的 `64`，先砍掉重复 snapshot 驻留预算而不改 replay / recovery 合同。 Trace: .pm/tasks/task_dfb9d8eedfe14f218c2f6e77151dad25.yaml

## 执行顺序与依赖
- M1（契约冻结）: 先完成 T1.1 ~ T1.4，冻结 replay truth-source、checkpoint manifest 与外部 effect contract；T2 / T3 / T6 以此为前置。
- M2（写路径与 GC）: 再完成 T2.1 ~ T2.4 与 T3.1 ~ T3.3，优先解决 execution bridge 历史 refs 与 sidecar blob 无界增长。
- M3（冷热分层与观测）: 在 T4.1 ~ T5.3 与 T6.1 ~ T6.4 中统一 cold index / archive / metrics 语义，避免各子系统重复定义目录协议。
- M4（测试与收口）: T1.5、T2.5、T3.4、T4.4、T5.4、T6.5 与 T7.1 ~ T7.5 作为统一验证和回写阶段；未经 gate 通过不得切换默认 profile。
- 并行边界: T2 与 T3 可在 T1 完成后并行；T4 可与 T2/T3 并行推进，但 T5 / T6 需等待冷热目录语义稳定后再收口。

## 依赖
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md`
- `doc/world-runtime/prd.md`
- `doc/world-runtime/project.md`
- `doc/world-runtime/prd.index.md`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime.rs`
- `crates/oasis7/src/runtime/world/persistence.rs`
- `crates/oasis7/src/runtime/snapshot.rs`
- `crates/oasis7_node/src/replication.rs`
- `crates/oasis7_distfs/src/lib.rs`
- `testing-manual.md`

## 状态
- 更新日期: 2026-04-26
- 当前状态: completed
- 已完成: T0、T0.1、T1.1、T1.2、T1.3、T1.4、T1.5、T2.1、T2.2、T2.3、T2.4、T2.5、T3.1、T3.2、T3.3、T3.4、T4.1、T4.2、T4.3、T4.4、T5.1、T5.2、T5.3、T5.4、T6.1、T6.2、T6.3、T6.4、T6.5、T7.1、T8.1
- 已拆解待执行: 无
- 进行中: 无
- 阻塞项: 无；但 T2 / T3 / T6 / T7 的实现必须以前置 T1 契约冻结为准。
- 本轮新增: T6.1 已完成共享 `StorageProfileConfig` 协议、`oasis7_chain_runtime --storage-profile`、`oasis7_game_launcher --chain-storage-profile`、`oasis7_web_launcher` / launcher UI 同名透传入口，并先将 replication 热窗口预算接入 profile 默认值。
- 本轮新增: T6.2 已在 `oasis7_chain_runtime` 中新增共享 `StorageMetricsSnapshot`，按秒写出 `reward-runtime-storage-metrics.json`，并把 storage section 接到 `/v1/chain/status`，当前至少覆盖 bytes、blob_counts、ref_count、pin_count、checkpoint_count、orphan_blob_count 与 GC 最近结果。
- 本轮新增: T6.3 已把 `effective_budget` 与 `replay_summary` 接入 `StorageMetricsSnapshot` / `/v1/chain/status.storage`，并把 retained heights / checkpoint heights 收敛为 `latest_only`、`full_log_only`、`checkpoint_plus_log` 三档回放能力摘要，供 launcher / 脚本直接判断治理状态。
- 本轮新增: T6.4 已在 bundle 中新增 `run-chain-runtime.sh`，并让 `run-game.sh` / `run-web-launcher.sh` 共享 `OASIS7_CHAIN_STORAGE_PROFILE` 覆盖通道；wrapper 仅在显式设置时注入 profile 参数，默认继续继承底层二进制口径，避免 shell 常量漂移。
- 本轮新增: T6.5 已补齐定向测试：`oasis7_chain_runtime` status payload 现锁住 `last_gc_error` / `degraded_reason` / `replay_summary` 字段，`oasis7_game_launcher` 与 `oasis7_web_launcher` 也分别补上未知 profile 拒绝与 profile 透传回归。
- 本轮新增: T7.1 已在 `crates/oasis7/src/runtime/tests/storage_footprint_fixture.rs` 新增可复现实验基线：通过 `2500` 次 `World::step()` + `save_to_dir()` 生成 archive/snapshot 样本，并锁住 `tick_consensus_total_record_count`、archive index 与范围读回，供后续 footprint gate / replay regression 复用。
- 本轮新增（2026-03-10 / T7.2）: 已新增 `scripts/oasis7-runtime-storage-gate.sh`，可直接消费 `reward-runtime-storage-metrics.json` 或 `/v1/chain/status` JSON，校验 `storage_profile/effective_budget/checkpoint_count/orphan_blob_count/replay_summary/last_gc_result/degraded_reason` 并输出 `summary.md/json`。
- 本轮验证样本: `.tmp/world_runtime_storage_gate/20260310-234359/summary.md`（合成 `release_default` 样本通过）与 `doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md`（真实 `oasis7_chain_runtime` 样本已将根因从“未达 64”更新为“execution bridge 未绑定 profile cadence（已修复并完成 QA 复验）”）。
- 本轮新增（2026-03-10 / T7.2 root cause）: 真实 probe 已在 `height=32` 观察到 `checkpoint_count=1`，并结合读码确认 `oasis7_chain_runtime` 仍使用 execution bridge 的硬编码 `32/4` retention 默认值，而不是 `release_default` 的 `64/8`。
- 本轮新增（2026-03-11 / T7.2 QA）: `qa_engineer` 已用真实 `release_default` 样本确认 `height=47` 时 `checkpoint_count=0/full_log_only`，`height=65` 时 `checkpoint_count=1/checkpoint_plus_log`，说明修复后 cadence 与 budget 对齐。
- 本轮新增（2026-03-11 / T7.3）: 已新增 `doc/world-runtime/evidence/runtime-sidecar-orphan-gc-failsafe-2026-03-11.md` 与定向回归 `collect_storage_metrics_sidecar_orphan_recovers_after_successful_save`，证明 sidecar orphan 可在下一次成功 save/GC 后收敛到 `0`。
- 本轮新增（2026-03-11 / T7.4 启动）: 已补 `oasis7_game_launcher` / `oasis7_web_launcher` 的 tri-profile 参数透传回归，并向 `viewer_engineer` 发起 bundle / launcher 实测 handoff。
- 本轮新增（2026-03-11 / T7.4 viewer）: 已生成 `doc/world-runtime/evidence/runtime-launcher-profile-consistency-2026-03-11.md`，通过 bundle 实物与 `bash -x` trace 确认 `run-game.sh` / `run-web-launcher.sh` / `run-chain-runtime.sh` 对三档 profile 的注入口径一致。
- 本轮新增（2026-03-11 / T7.5）: 已完成专题/模块/testing-manual/devlog 收口，并将 T7.2~T7.4 的正式 evidence 归档到专题状态。
- 本轮新增（2026-04-26 / T8.1）: 已为 replication hot-window offload 增加 cold-index-aware orphan sweep；定向样本确认旧 `record + payload[]` envelope orphan blob 会在下一次 offload 后被清理，而不会误删 cold index 仍引用的归档 commit。
- 本轮新增（2026-04-26 / T8.1）: `LocalCasStore` 现会在 path overwrite/delete 后即时清理不再被 `files_index`/pin 引用的旧 blob；`files_index`、热 commit mirror 与 cold-index 现改为 compact JSON 落盘，避免 pretty-json 把 `<1 KiB` 提交记录推过 `4 KiB` 块边界。
- 本轮新增（2026-04-26 / T8.1）: `replication_commit_messages` 冷归档现默认写入 `<namespace>.cold-index/segments/*.pack` 的固定高度段 pack 文件，cold index 仅记录 `segment_id + offset + len + content_hash`；同段多条冷 commit 共享同一文件，避免继续为每条冷 commit 保留独立 CAS blob。
- 本轮新增（2026-04-26 / T8.1）: 旧 `height -> content_hash` 冷索引兼容样本会在第一次读回时自动迁移到 pack ref，并在 pack 索引落盘后删除不再被 file index / pin 引用的 legacy cold blob。
- 本轮新增（2026-05-15 / release-default budget）: 针对本地三节点真实样本复盘后确认，`release_default` 体积膨胀主要来自 `execution_store_root` 热窗口里的整份 execution snapshots，而不是 checkpoint 或 journal。由于当前 `stale/non-contiguous` 恢复仍按具体高度直接读取热窗口 snapshot，尚不能安全删除所有非最新热高度 snapshot，因此本轮先把 `release_default.execution_hot_head_heights` 从 `128` 收紧到 `64`，与 `execution_checkpoint_interval=64` 对齐，优先砍掉默认档位的重复 snapshot 驻留预算。
- 本轮验证（2026-05-15 / release-default budget）: 已新增定向回归，确认 `release_default` 在 `height=65` 时高度 `1` 的 snapshot/journal refs 会被裁剪，而高度 `2..65` 仍保留当前 64 高度热窗口所需的恢复数据；同时保留 `height=64` 的 checkpoint cadence 不变。
- 本轮新增（2026-05-15 / transparent blob compression）: `LocalCasStore` 已支持对大于阈值且压缩后更小的 blob 采用“磁盘透明压缩、读出仍返回原始 bytes”的落盘策略，不改变 `content_hash` 合同；这会直接作用于 execution bridge snapshots/journals 与 runtime sidecar blobs，而不要求上层 record/manifest/schema 迁移。
- 下一任务: 无（本专题当前轮次完成）
