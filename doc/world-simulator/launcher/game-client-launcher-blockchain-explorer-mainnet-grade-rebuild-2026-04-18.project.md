# 客户端启动器区块链浏览器主链级信息架构重构项目文档

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] launcher-explorer-mainnet-grade-rebuild-prd (PRD-WORLD_SIMULATOR-044) [test_tier_required]: 完成本专题 PRD / Design / Project 建模，并回写 `world-simulator` 主 PRD / project / index。 Trace: .pm/tasks/task_552222a529fa48489eab10deb789ed54.yaml
- [ ] launcher-explorer-mainnet-grade-rebuild (PRD-WORLD_SIMULATOR-044) [test_tier_required]: 对标主链浏览器重构启动器 explorer 的命令区、链健康概览、tab 导航、主列表、详情检查板与空态/错误态，并完成 native/wasm required 验证。 Trace: .pm/tasks/task_552222a529fa48489eab10deb789ed54.yaml

## 依赖
- `doc/world-simulator/prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/prd.index.md`
- `crates/oasis7_client_launcher/src/explorer_window.rs`
- `crates/oasis7_client_launcher/src/explorer_window_view.rs`
- `crates/oasis7_client_launcher/src/explorer_window_p1.rs`
- `crates/oasis7_client_launcher/src/main.rs`
- `crates/oasis7_client_launcher/src/main_tests.rs`
- `testing-manual.md`

## 状态
- 最近更新: 2026-04-18
- 当前阶段: active
- 当前任务: `launcher-explorer-mainnet-grade-rebuild`
- 备注: 本轮只重构 explorer 视图层和渲染辅助分层，不改 runtime / control-plane 协议。
