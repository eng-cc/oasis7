# oasis7 Runtime：Replication Writer Epoch/Sequence 数值语义硬化（15 点清单第四阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md`

### T1 Replication Writer 递进显式溢出语义
- [x] `next_local_record_position` 改为可失败接口，移除关键路径 `saturating_add`。
- [x] `build_local_commit_message` 透传位置计算失败错误。
- [x] 新增 replication 模块溢出边界测试（3 类路径）。

### T2 回归与收口
- [x] 运行 node 定向测试并确认 required-tier 门禁通过。
- [x] 更新设计/项目文档状态与 `doc/devlog/README.md` 收口记录。

## 依赖
- `crates/oasis7_node/src/replication.rs`
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/tests.rs`
- `crates/oasis7_node/src/tests_hardening.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2
- 进行中：无
- 未开始：无
- 阻塞项：无
