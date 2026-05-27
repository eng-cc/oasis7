# oasis7 Runtime：Membership Reconciliation 调度门控与对账计数算术语义硬化（15 点清单第十三阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md`

### T1 调度与 dedup 时间门控受检化
- [x] `schedule_due` 的时间差从 `saturating_sub` 改为受检减法并透传错误。
- [x] `deduplicate_revocation_alerts` 的 suppress window 时间差从 `saturating_sub` 改为受检减法。
- [x] 增加 overflow 拒绝测试并验证 `schedule_state` / `dedup_state` 不被污染。

### T2 对账报告计数受检化
- [x] `reconcile_revocations_with_policy` 的关键计数累加改为受检语义。
- [x] 增加计数 helper overflow 测试并验证错误语义一致。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_reconciliation.rs`
- `crates/oasis7_consensus/src/membership_reconciliation_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
