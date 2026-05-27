# oasis7 Runtime：Membership Sync 与 Mempool 批处理计数算术语义硬化（15 点清单第十四阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md`

### T1 Membership Sync 计数受检化
- [x] `sync_key_revocations`、`sync_key_revocations_with_policy`、`sync_membership_directory` 关键计数改为受检累加。
- [x] 增加计数 helper overflow 测试并验证错误语义一致。

### T2 Mempool payload 累加受检化
- [x] `take_batch_with_rules` 的 `total_bytes + size_bytes` 改为受检累加。
- [x] 增加 overflow 测试并验证错误语义一致。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership.rs`
- `crates/oasis7_consensus/src/mempool.rs`
- `crates/oasis7_consensus/src/membership_tests.rs`
- `crates/oasis7_consensus/src/mempool.rs`（模块内测试）

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
