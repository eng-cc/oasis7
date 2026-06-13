# Viewer Live runtime/world 接管 Phase 3（action 映射覆盖 + 旧分支移除）（2026-03-05）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-018) [test_tier_required]: 完成专题 PRD 建模、验收冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-018) [test_tier_required]: 扩展 `simulator_action_to_runtime` 覆盖并补齐关键动作映射（含模块工件动作）。
- [x] T2 (PRD-WORLD_SIMULATOR-018) [test_tier_required]: 新增 action 映射等价回归与拒绝语义回归测试。
- [x] T3 (PRD-WORLD_SIMULATOR-018) [test_tier_required]: 删除 `oasis7_viewer_live` simulator 启动分支，更新手册并完成 required 回归收口。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/viewer/runtime_live/control_plane.rs`
- `crates/oasis7/src/viewer/runtime_live.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段: completed
- 当前任务: none
- 备注: 已完成 action 映射覆盖、等价回归与 runtime-only 分支收敛，required 回归通过。
