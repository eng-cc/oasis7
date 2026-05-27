# oasis7 Runtime：Sequencer 主循环与 Lease 递进数值语义硬化（15 点清单第五阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md`

### T1 Sequencer 递进语义硬化
- [x] `sequencer_mainloop` 的 `next_slot`/`next_height` 递进改为受检溢出语义。
- [x] 终态更新改为“先计算后提交”，失败时避免局部状态写入。
- [x] 新增对应溢出拒绝测试。

### T2 Lease 递进语义硬化
- [x] `lease` 的 `next_term` 与 `expires_at_ms` 递进改为受检语义。
- [x] acquire/renew 越界时拒绝并保持原状态。
- [x] 新增对应溢出拒绝测试。

### T3 回归与收口
- [x] 运行 `oasis7_consensus` 定向回归测试。
- [x] 回写设计文档状态（M0~M3）。
- [x] 回写项目状态与 `doc/devlog/README.md`。

## 依赖
- `crates/oasis7_consensus/src/sequencer_mainloop.rs`
- `crates/oasis7_consensus/src/lease.rs`
- `crates/oasis7_consensus/src/lib.rs`
- `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
