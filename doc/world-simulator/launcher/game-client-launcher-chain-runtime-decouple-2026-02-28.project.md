# 启动器链路重构：链运行时与游戏进程解耦（2026-02-28）项目管理

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.design.md`
- 对应需求文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md`

审计轮次: 6
## 任务拆解（含 PRD-ID 映射）
- [x] T0 建档：设计文档 + 项目管理文档。
- [x] T1 新增 `oasis7_chain_runtime`：节点启动/停止、状态 API、余额 API。
- [x] T2 重构 `oasis7_game_launcher`：默认托管 `oasis7_chain_runtime`，`oasis7_viewer_live` 收敛为纯观察服务。
- [x] T3 更新发行链路：`build-game-launcher-bundle.sh` 纳入 `oasis7_chain_runtime`，更新桌面/CLI 入口参数透传。
- [x] T4 回归与收口：required 测试、项目状态与文档更新。
- [x] T5 残留语义修正：对齐 PoS 时间锚定参数（`--chain-pos-*`）与 `oasis7_viewer_live` 纯观察服务边界描述。

## 依赖
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.design.md`
- doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md
- `oasis7_node`（NodeRuntime/PoS/P2P）
- `oasis7::runtime::World`（execution world 读取余额）
- 现有 `oasis7_game_launcher` 与 `oasis7_client_launcher`

## 状态
- 最近更新：2026-03-08（ROUND-006 语义口径回写）
- 当前阶段：已完成（T0~T5）。
- 当前任务：无（项目结项）。
