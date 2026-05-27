# oasis7 Runtime：无限时长运行的序列号滚动与数值防溢出（项目管理文档）

- 对应设计文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.design.md`
- 对应需求文档: `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md`
- [x] 新建项目管理文档：`doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md`

### T1 序列滚动与持久化
- [x] 为 runtime 四类 `next_*_id` 增加 era 状态与滚动分配逻辑
- [x] snapshot 增加 era 字段并保持旧快照兼容
- [x] 补充序列滚动与快照 roundtrip 测试

### T2 数值防溢出加固
- [x] 修复关键未保护加法（资源与规则聚合）
- [x] 修复 `len as u32` 与 `u64 as i64` 窄化转换风险
- [x] 补充对应拒绝路径/边界测试

### T3 回归与收口
- [x] 运行 runtime 与相关 crate 定向测试
- [x] 更新设计/项目文档状态
- [x] 更新 `doc/devlog/README.md`

## 依赖
- Runtime world 核心：
  - `crates/oasis7/src/runtime/world/mod.rs`
  - `crates/oasis7/src/runtime/world/actions.rs`
  - `crates/oasis7/src/runtime/world/effects.rs`
  - `crates/oasis7/src/runtime/world/governance.rs`
  - `crates/oasis7/src/runtime/world/event_processing.rs`
  - `crates/oasis7/src/runtime/world/persistence.rs`
- Snapshot 结构：
  - `crates/oasis7/src/runtime/snapshot.rs`
- 相关防溢出路径：
  - `crates/oasis7/src/runtime/world/resources.rs`
  - `crates/oasis7/src/runtime/rules.rs`
  - `crates/oasis7/src/simulator/types.rs`
  - `crates/oasis7/src/runtime/world/module_runtime.rs`
  - `crates/oasis7_wasm_executor/src/lib.rs`
  - `crates/oasis7_net/src/execution_storage.rs`

## 状态
- 当前状态：`已完成`
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
