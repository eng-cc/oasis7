# Non-Viewer 设计一致性修复（项目管理）

- 对应设计文档: `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.design.md`
- 对应需求文档: `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] T-MIG-20260303 (PRD-ENGINEERING-006): 逐篇阅读旧文档并完成人工重写迁移到 `.prd` 命名。
### T0 建档
- [x] 新建设计文档：`doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md`
- [x] 新建项目管理文档：`doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.project.md`

### T1 修复 rejected 分支静默丢失
- [x] `PosNodeEngine::apply_decision` rejected 分支回灌失败改为显式报错
- [x] `pending_consensus_action_capacity` 增加 pending proposal 回灌容量预留
- [x] `oasis7_node` 定向回归测试通过

### T2 修复 dead-letter 冷归档回放缺口
- [x] `FileMembershipRevocationAlertDeadLetterStore::list` 改为冷热聚合读取
- [x] `FileMembershipRevocationAlertDeadLetterStore::list_delivery_metrics` 改为冷热聚合读取
- [x] `replace` 清理并重建 archive refs，避免 stale cold refs
- [x] `oasis7_consensus` 定向回归测试通过

### T3 收口
- [x] 回写 `doc/headless-runtime/README.md` 活跃文档索引
- [x] 追加 `doc/devlog/README.md` 任务日志

## 依赖
- `crates/oasis7_node/src/lib_impl_part1.rs`
- `crates/oasis7_node/src/tests_action_payload.rs`
- `crates/oasis7_consensus/src/membership_recovery/dead_letter.rs`
- `crates/oasis7_consensus/src/membership_recovery_tests_split_part1.rs`

## 状态
- 当前状态：已完成
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
