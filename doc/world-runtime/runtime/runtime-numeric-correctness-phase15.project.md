# oasis7 Runtime：Membership Recovery Federated 聚合扫描与游标计数算术语义硬化（15 点清单第十五阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md`

### T1 Federated 聚合扫描计数受检化
- [x] `scanned_hot` / `scanned_cold` 从饱和累加改为受检累加。
- [x] 增加对应 overflow 单元测试并验证错误语义一致。

### T2 复合游标偏移计数受检化
- [x] `node_event_offset` 递进从饱和递进改为受检递进。
- [x] 增加对应 overflow 单元测试并验证错误语义一致。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_recovery/replay_archive_federated.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_archive_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
