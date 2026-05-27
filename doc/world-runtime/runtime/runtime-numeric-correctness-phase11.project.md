# oasis7 Runtime：Membership Replay 调度/冷却时间门控算术语义硬化（15 点清单第十一阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md`

### T1 调度与策略冷却时间门控受检化
- [x] `run_revocation_dead_letter_replay_schedule_with_state_store` 的 interval gate 改为受检减法。
- [x] `recommend_revocation_dead_letter_replay_policy_with_adaptive_guard` 的 policy cooldown 改为受检减法。
- [x] 补齐对应 overflow 拒绝测试并验证 replay state 不被污染。

### T2 Rollback 冷却门控受检化
- [x] `should_rollback_to_stable_policy` 的 rollback cooldown 改为受检减法并沿调用链显式失败。
- [x] 补齐 rollback cooldown overflow 测试并验证 policy state 不被部分更新。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_recovery/replay.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_tests.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_persistence_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
