# 启动器 chain runtime execution world 输出路径收敛（2026-03-09）项目管理文档

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-033) [test_tier_required]: 完成“launcher execution world 输出路径收敛”PRD 建模与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-033) [test_tier_required]: 在 `oasis7_game_launcher` / `oasis7_web_launcher` 显式传递 `--execution-world-dir` 到 `output/chain-runtime/<node_id>/reward-runtime-execution-world`，并补齐定向回归测试。

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_game_launcher.rs`
- `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/oasis7_web_launcher_tests.rs`

## 状态
- 最近更新：2026-03-09
- 当前阶段: completed
- 当前任务: 无
- 备注: `T0/T1` 已完成，启动器双入口已显式收敛 execution world 输出目录规则。
