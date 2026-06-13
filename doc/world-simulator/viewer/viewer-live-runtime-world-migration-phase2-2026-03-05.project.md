# Viewer Live runtime/world 接管 Phase 2（LLM/chat/prompt）（2026-03-05）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-017) [test_tier_required]: 完成专题 PRD 建模、验收标准冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-017) [test_tier_required]: `oasis7_viewer_live` 放开 `--runtime-world --llm` 组合并接入 runtime live llm 模式配置。
- [x] T2 (PRD-WORLD_SIMULATOR-017) [test_tier_required]: 在 `runtime_live.rs` 落地 prompt/chat/auth/llm 决策桥接，消除 phase1 `unsupported` 断裂。
- [x] T3 (PRD-WORLD_SIMULATOR-017) [test_tier_required]: 执行 `cargo test/check` 回归，更新 viewer 手册、模块项目状态与当日 devlog 收口。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/viewer/runtime_live.rs`
- `crates/oasis7/src/viewer/auth.rs`
- `crates/oasis7/src/viewer/protocol.rs`
- `crates/oasis7/src/bin/oasis7_llm_agent_demo/runtime_bridge.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段: completed
- 当前任务: none
- 备注: 已完成 runtime 模式 `LLM/chat/prompt` 控制打通，script 模式统一返回 `llm_mode_required`，并通过 required 回归。
