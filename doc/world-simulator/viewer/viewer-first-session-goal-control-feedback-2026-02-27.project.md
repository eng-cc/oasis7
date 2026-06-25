# Viewer 首局目标与控制语义可解释反馈优化（2026-02-27）项目管理文档

- 对应设计文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0 建立设计文档与项目文档，明确边界与接口
- [x] T1 首局目标改造：输出 1 个主目标 + 2 个短目标，并接入引导 HUD
- [x] T2 控制语义可发现：补充 `describeControls` 与示例填充入口
- [x] T3 输入可解释反馈：`sendControl` 结构化返回 + `getState.lastControlFeedback`
- [x] T4 测试与收口：补充/更新测试、回写文档状态、沉淀任务日志

## 依赖
- doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md
- `doc/playability_test_result/game-test.prd.md`
- `crates/oasis7_viewer/src/egui_right_panel_player_guide.rs`
- `crates/oasis7_viewer/src/egui_right_panel_player_experience.rs`
- `crates/oasis7_viewer/src/web_test_api.rs`
- `crates/oasis7_viewer/src/egui_right_panel_tests.rs`

## 状态
- 当前阶段：已完成（T0~T4）
- 最近更新：2026-06-25
- 说明：2026-06-25 follow-up 是呈现层强化，不改变 runtime action 语义、LLM 策略或 `PRD-GAME-012` gate verdict；`software_safe` 在 `Formal Gameplay Summary` 顶部新增 `Control Proof` surface，用既有 `player_gameplay` / feedback 派生 `Player Intent / World Consequence / Recovery Move / Next Move`，并追加 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation`、`Share Replay` 承接 P1/P2 制作人落点；这些 surface 均以 contract/UI 测试锁定，canonical truth 仍来自 runtime/player_gameplay，small-player lane 设计真值挂回 `PRD-GAME-015`。
