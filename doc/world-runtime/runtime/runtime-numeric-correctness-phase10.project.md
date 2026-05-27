# oasis7 Runtime：Governance Tiered Offload 与 Rollback Audit 算术语义硬化（15 点清单第十阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md`

### T1 Tiered Offload/Drill Alert 受检化
- [x] `membership_recovery/replay_archive_tiered.rs` 的 offload/silence/cooldown 时间差改为受检减法。
- [x] 去除 `plan_governance_audit_tiered_offload` 中不必要饱和计数语义。
- [x] 补齐 underflow/overflow 拒绝测试并验证无状态污染。

### T2 Rollback Audit/Governance 受检化
- [x] `membership_recovery/replay_audit.rs` 的 rollback streak 递进改为受检加法。
- [x] rollback alert 窗口/cooldown 时间差改为受检减法。
- [x] 补齐 overflow/underflow 测试并验证治理状态不被部分更新。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_recovery/replay_archive_tiered.rs`
- `crates/oasis7_consensus/src/membership_recovery/replay_audit.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_archive_tests.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_audit_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
