# 客户端启动器 native/web 同控制面与客户端服务端分离（2026-03-04）项目管理文档

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] T0 (PRD-WORLD_SIMULATOR-015) [test_tier_required]: 完成专题 PRD 建模、验收标准冻结与模块文档树回写。
- [x] T1 (PRD-WORLD_SIMULATOR-015) [test_tier_required]: 升级 `oasis7_web_launcher` 为游戏/区块链独立编排控制面，新增链独立启停 API 与状态快照。
- [x] T2 (PRD-WORLD_SIMULATOR-015) [test_tier_required]: `oasis7_client_launcher` native 改为客户端-服务端分离并复用同一 API 控制链路，恢复 web 端链启停按钮与状态对齐。
- [x] T3 (PRD-WORLD_SIMULATOR-015) [test_tier_required]: 执行 `cargo test/check` + `agent-browser --headed` 闭环（含链/游戏独立启停），归档证据并收口文档。
- [x] launcher-p2p-peer-list-ui (PRD-WORLD_SIMULATOR-015) [test_tier_required]: 透传 `/api/state` 的 `chain_replication_status`，并在启动器 `节点观测` 摘要卡展示 peer 健康概览，同时提供可单独打开的 peer 明细窗口展示本地 peer id 与已连接 peer 明细。 Trace: .pm/tasks/task_ee3cc0c5d2d741658b404100843f93d8.yaml

## 依赖
- `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.design.md`
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7/src/bin/oasis7_web_launcher.rs`
- `crates/oasis7_client_launcher/src/main.rs`
- `crates/oasis7_client_launcher/src/app_process.rs`
- `crates/oasis7_client_launcher/src/app_process_web.rs`
- `output/playwright/`

## 状态
- 最近更新：2026-04-21（launcher peer list UI）
- 当前阶段: completed
- 当前任务: none
- 备注: 已完成 native/web 同控制面收口，agent-browser 证据归档于 `output/playwright/launcher-control-plane-unification-20260304/artifacts-final/`。
