# oasis7 Runtime：PoS 超多数比率边界数值语义硬化（15 点清单第七阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md`

### T1 PoS 比率判定去饱和化
- [x] `crates/oasis7_proto/src/distributed_pos.rs` 超多数比率判定改为无溢出实现。
- [x] `crates/oasis7_consensus/src/pos.rs` 同步改造。
- [x] `crates/oasis7_node/src/pos_validation.rs` 同步改造。

### T2 边界测试补齐
- [x] 新增大整数比率边界测试（`denominator = u64::MAX`）验证不会误拒绝。
- [x] 校验 proto/consensus/node 三层行为一致。

### T3 回归与收口
- [x] 运行 `oasis7_proto`、`oasis7_consensus`、`oasis7_node` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_proto/src/distributed_pos.rs`
- `crates/oasis7_consensus/src/pos.rs`
- `crates/oasis7_node/src/pos_validation.rs`
- `crates/oasis7_node/src/tests_hardening.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
