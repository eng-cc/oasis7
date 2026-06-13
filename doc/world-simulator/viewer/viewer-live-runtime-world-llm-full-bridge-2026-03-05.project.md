# Viewer Live runtime/world 真 LLM 全量接管（LLM 决策 + 100% 事件/快照 + hard-fail）（2026-03-05）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-019) [test_tier_required]: 完成专题 PRD 建模、验收标准冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-019) [test_tier_required]: 移除启发式 sidecar，落地真实 LLM driver + shadow WorldKernel，并接入硬失败语义。
- [x] T2 (PRD-WORLD_SIMULATOR-019) [test_tier_required]: 补齐 runtime 事件/快照 100% 映射、扩展 viewer 协议并输出 DecisionTrace。
- [x] T3 (PRD-WORLD_SIMULATOR-019) [test_tier_required]: 执行 required 回归、更新 viewer 手册与模块项目状态收口。

## 依赖
- `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.design.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/viewer/runtime_live.rs`
- `crates/oasis7/src/viewer/runtime_live/control_plane.rs`
- `crates/oasis7/src/viewer/protocol.rs`
- `crates/oasis7/src/simulator/llm_agent.rs`
- `crates/oasis7/src/simulator/runner.rs`
- `crates/oasis7/src/runtime/state.rs`
- `crates/oasis7/src/runtime/world/domain.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段: completed
- 当前任务: none
- 备注: true LLM 全量接管、事件/快照覆盖与手册/回归收口已完成。
