# oasis7 Runtime：Membership Recovery 调度门控与计数聚合算术语义硬化（15 点清单第十二阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md`

### T1 调度时间门控受检化
- [x] `run_revocation_dead_letter_replay_schedule` 的 interval gate 改为受检减法。
- [x] 新增 overflow 拒绝测试并验证 `last_replay_at_ms` 不被污染。

### T2 Recovery 计数/容量受检化
- [x] `emit_revocation_reconcile_alerts_with_recovery_and_ack_retry_with_dead_letter` 的关键 `saturating_add` 改为受检语义。
- [x] 补齐计数边界 overflow 测试并验证 pending/dead-letter 不被部分更新。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_recovery/mod.rs`
- `crates/oasis7_consensus/src/membership_recovery_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
