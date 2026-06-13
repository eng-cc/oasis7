# 启动器 chain runtime stale execution world 恢复（2026-03-12）项目管理文档

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 完成“launcher stale execution world 恢复”PRD/Design/Project 建模与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 为 launcher / GUI Agent 增加 stale execution world 结构化错误识别，并在状态层暴露恢复建议。
- [x] T2 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 实现 fresh node id 恢复链路与回归验证，覆盖 `start_chain -> query_explorer_overview -> start_game` 最小闭环。
- [x] T3 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 为 `scripts/run-game-test.sh` 增加 fresh `chain_node_id` 默认值与链参数透传，避免一键试玩复用脏 execution world。
- [x] T4 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 复验 chain-enabled 一键试玩栈与 agent-browser Web 闭环，确认 fresh node id 默认值可消除 stale execution world 启动阻断。
- [x] T5 (PRD-WORLD_SIMULATOR-034) [test_tier_required]: 将 `oasis7_web_launcher` / `oasis7_client_launcher` / `oasis7_game_launcher` 的默认链配置改为 fresh `chain_node_id`，并完成“默认 `start_chain -> query_explorer_overview`”实机复验。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/gui_agent_api.rs`
- `crates/oasis7_client_launcher/src/*`

## 状态
- 最近更新：2026-03-12
- 当前阶段: completed
- 当前任务: `none`
- owner: `viewer_engineer`
- 联审: `runtime_engineer`
- 备注: `T0~T5` 已完成；`oasis7_web_launcher` 已新增 stale execution world 结构化状态、GUI Agent `recover_chain` 动作与 fresh node id 恢复建议；现在 `oasis7_web_launcher` / `oasis7_client_launcher` / `oasis7_game_launcher` 默认链配置与 `scripts/run-game-test.sh` 一样，都会优先使用 fresh `chain_node_id`，减少默认入口复用脏 execution world 的概率。
