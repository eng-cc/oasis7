# oasis7 Runtime：节点高度/Slot 递进与复制补洞数值语义硬化（15 点清单第三阶段）项目管理文档

- 对应设计文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md`

### T1 Node 引擎高度/Slot 显式溢出语义
- [x] `apply_decision` / `record_synced_replication_height` 改为可失败接口，移除关键路径 `saturating_add(1)`。
- [x] 复制摄取与 gap sync 对 `committed_height + 1`、`next_height + 1` 统一受检处理。
- [x] proposal 摄取路径 `next_slot` 递进改为显式溢出错误。

### T2 快照恢复边界与测试
- [x] `restore_state_snapshot` 对 `committed_height + 1` 改为受检语义并透传启动错误。
- [x] 新增边界测试：溢出拒绝且状态不被部分更新。

### T3 回归与收口
- [x] 运行 `test_tier_required` 口径的 node 定向测试与编译检查。
- [x] 更新设计/项目文档状态与 `doc/devlog/README.md` 收口记录。

## 依赖
- Node：
  - `crates/oasis7_node/src/lib.rs`
  - `crates/oasis7_node/src/pos_state_store.rs`
- 测试：
  - `crates/oasis7_node/src/tests.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
