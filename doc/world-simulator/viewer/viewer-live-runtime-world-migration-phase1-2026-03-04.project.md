# Viewer Live runtime/world 接管 Phase 1（2026-03-04）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-016) [test_tier_required]: 完成专题 PRD 建模、验收标准冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-016) [test_tier_required]: 新增 runtime live server，实现 runtime->simulator 协议兼容适配并接入 `oasis7_viewer_live --runtime-world`。
- [x] T2 (PRD-WORLD_SIMULATOR-016) [test_tier_required]: 执行 `cargo test/check` 回归、更新 viewer 手册与模块项目状态，完成 devlog 收口。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/viewer/mod.rs`
- `crates/oasis7/src/viewer/runtime_live.rs`
- `crates/oasis7/src/viewer/protocol.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段: completed
- 当前任务: none
- 备注: Phase 1 已完成“runtime 驱动 + 协议兼容适配 + required 回归”，simulator 默认路径保持不变。
