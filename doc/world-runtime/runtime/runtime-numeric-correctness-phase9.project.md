# oasis7 Runtime：Governance Drill/Retention 时间算术数值语义硬化（15 点清单第九阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md`

### T1 调度与保留时间算术受检化
- [x] `membership_recovery/replay_archive.rs` 的 drill schedule 时间算术改为受检语义。
- [x] `membership_recovery/replay_archive.rs` 的 audit retention 年龄计算改为受检语义。
- [x] 新增 schedule/retention 溢出拒绝测试并验证无状态污染。

### T2 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/membership_recovery/replay_archive.rs`
- `crates/oasis7_consensus/src/membership_dead_letter_replay_archive_tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2
- 进行中：无
- 未开始：无
- 阻塞项：无
