# 客户端启动器 native/web 同控制面与客户端服务端分离（2026-03-04）

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.design.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.project.md`

审计轮次: 5

## 1. Executive Summary
- Problem Statement: 当前启动器 native 端直接本地拉起游戏/区块链进程，而 web 端通过 `oasis7_web_launcher` 间接控制，导致两端状态机与按钮行为不一致，链路回归成本高。
- Proposed Solution: 将 `oasis7_web_launcher` 升级为统一控制面（游戏/区块链独立编排）；native 启动器改为“客户端 + 本地服务端”模式，和 web 启动器共用同一套 API 状态机。
- Success Criteria:
  - SC-1: `oasis7_web_launcher` 提供游戏与区块链独立启动/停止 API，并返回独立状态。
  - SC-2: `oasis7_client_launcher` native 与 wasm 统一消费同一 API 契约，不再维护双套进程编排逻辑。
  - SC-3: Web 端“启动区块链/停止区块链”按钮恢复可用，状态文案与 native 对齐。
  - SC-4: native 端功能行为与历史桌面版一致（自动拉起链、独立控制游戏/区块链、链状态门控）。
  - SC-5: `agent-browser --headed` 闭环可覆盖链/游戏独立启停并输出证据。

## 2. User Experience & Functionality
- User Personas:
  - 启动器玩家：希望无论桌面还是浏览器，看到一致的状态和按钮行为。
  - 运维人员：希望 headless 服务器与本地桌面调试使用同一条控制链路。
  - 启动器开发者：希望减少平台分叉，只维护一个控制面协议。
- User Scenarios & Frequency:
  - 每次调试启动链路时（高频）：桌面与浏览器应表现一致。
  - 每次发布前回归（每版本）：必须用同一 API 契约验证链/游戏独立启停。
- User Stories:
  - As a 启动器玩家, I want native and web launcher to share the same control-plane behavior, so that game/blockchain status and actions stay consistent.
  - As an 运维人员, I want browser and desktop launcher to control chain/game independently through one API contract, so that headless and local workflows are interchangeable.
- Critical User Flows:
  1. Flow-LAUNCHER-CP-001（统一控制面启动）:
     `启动 oasis7_web_launcher -> native 或 web 客户端连接 /api/state -> 展示一致状态`
  2. Flow-LAUNCHER-CP-002（链独立控制）:
     `点击启动区块链 -> /api/chain/start -> 状态 not_started/starting/ready -> 点击停止区块链 -> /api/chain/stop`
  3. Flow-LAUNCHER-CP-003（游戏独立控制）:
     `点击启动游戏 -> /api/start -> 状态 running -> 点击停止游戏 -> /api/stop`
  4. Flow-LAUNCHER-CP-004（native 客户端服务端分离）:
     `打开 native 启动器 -> 本地拉起 oasis7_web_launcher -> 后续全部操作经 HTTP API`
  5. Flow-LAUNCHER-CP-005（功能对齐回归）:
     ``agent-browser --headed` 打开 web 启动器 -> 链/游戏独立启停 -> 状态与按钮反馈一致`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 控制面 API 契约 | `/api/state` `/api/start` `/api/stop` `/api/chain/start` `/api/chain/stop`；`/api/state` 附带 `chain_p2p_status/chain_observability_status/chain_replication_status` | 游戏与区块链分别触发独立 API，并向 native/web 提供同一份链状态与 peer 明细快照 | `idle/running/stopped/...` + `disabled/not_started/starting/ready/...` | 轮询刷新状态；动作响应覆盖最新快照 | 受信网络部署 |
| native 客户端服务端分离 | 本地 `oasis7_web_launcher` 子进程 + 监听地址 | 启动器 UI 不再直接 spawn 游戏/链，改为 API 调用 | `service_booting -> service_ready -> control_ready` | native 端优先连接本地服务端 | 仅本机会话可管理本地子进程 |
| 链状态门控对齐 | `chain_enabled` + `chain_status_bind` | 链未就绪时反馈入口禁用，链就绪后启用 | `not_started/starting/ready/unreachable/config_error` | 以服务端状态为唯一来源 | 客户端只读状态 |
- Acceptance Criteria:
  - AC-1: `oasis7_web_launcher` 支持 `POST /api/chain/start`、`POST /api/chain/stop`，且与游戏启停互不耦合。
  - AC-2: `/api/state` 返回游戏与区块链独立状态字段，客户端不再用 `snapshot.running` 推断链状态。
  - AC-2a: `/api/state` 在链就绪时同步返回 `chain_replication_status.local_peer_id/connected_peers/peer_healths`，供 native/web 启动器在 `节点观测` 摘要卡与可单独打开的 peer 明细窗口里直接展示已连接 peer 信息，而不新增第二套客户端探针。
  - AC-2b: 启动器配置增加 `chain_replication_bootstrap_peers` 入口，支持通过持久化配置文件与 launcher 界面填写 bootstrap peer multiaddr，并在 native/web 控制面启动链时统一透传为 `oasis7_chain_runtime --replication-network-peer <multiaddr>`。
  - AC-3: `oasis7_client_launcher` native 不再直接拉起 `oasis7_game_launcher` / `oasis7_chain_runtime`，改为通过 `oasis7_web_launcher` API 控制。
  - AC-4: wasm/web 启动器的“启动区块链/停止区块链”按钮恢复可操作，并与 native 同状态语义。
  - AC-5: native 与 web 在“自动拉起链 + 游戏/链独立启停 + 状态展示”行为上保持一致。
  - AC-6: `test_tier_required` 通过：`cargo test/check` + `agent-browser --headed` 闭环证据。
- Non-Goals:
  - 不改造 `oasis7_chain_runtime` 转账/反馈协议本身。
  - 不在本轮扩展新的链上业务动作。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属能力。

## 4. Technical Specifications
- Architecture Overview:
  - `oasis7_web_launcher` 升级为统一控制平面：内部维护游戏进程与区块链进程两套状态。
  - `oasis7_client_launcher` native 与 wasm 共用 API 驱动状态机；native 增加本地服务进程守护。
  - UI 层继续复用同一套 egui 渲染与 schema 字段映射。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_web_launcher.rs`
  - `crates/oasis7_client_launcher/src/main.rs`
  - `crates/oasis7_client_launcher/src/app_process.rs`
  - `crates/oasis7_client_launcher/src/app_process_web.rs`
  - `crates/oasis7_client_launcher/src/launcher_core.rs`
  - `scripts/build-game-launcher-bundle.sh`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 本地 native 服务端拉起失败：UI 显示可诊断错误并禁止误导性按钮状态。
  - 链运行时配置非法：`/api/chain/start` 返回结构化错误，链状态进入 `config_error`。
  - 游戏启动失败但链仍运行：游戏状态进入 `start_failed`，链状态不受影响。
  - 链停止时游戏仍运行：允许继续运行游戏，但反馈门控仍按链就绪状态禁用。
  - API 短时不可达：客户端状态降级为 `query_failed` 并自动重试轮询。
- Non-Functional Requirements:
  - NFR-1: `/api/state` 轮询 `p95 <= 200ms`（本地网络）。
  - NFR-2: native 与 web 客户端状态刷新节奏一致（默认 1s），不得出现持续状态漂移。
  - NFR-3: Rust 单文件维持 <=1200 行（必要时拆分模块）。
  - NFR-4: 本地 native 服务端启动后首个可用状态反馈时间 `p95 <= 2s`。
- Security & Privacy:
  - 仅暴露受控 API；日志不得输出敏感凭据。
  - 静态资源路径继续保持目录穿越防护。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1: PRD 建模与任务拆解。
  - M2: `oasis7_web_launcher` 双进程控制面能力落地。
  - M3: native 客户端接入统一 API，完成 web/native 对齐回归。
- Technical Risks:
  - 风险-1: 控制面状态机变更引发旧 UI 映射兼容问题。
  - 风险-2: native 本地服务端守护在端口冲突场景下启动失败。
  - 风险-3: 游戏与区块链独立编排后，日志上下文合并可读性下降。

## 6. Validation & Decision Record
- Test Plan & Traceability:
  - PRD-WORLD_SIMULATOR-015 -> TASK-WORLD_SIMULATOR-033/034/035 -> `test_tier_required`。
- Decision Log:
  - DEC-LAUNCHER-CP-001: 采用“native/web 共用 oasis7_web_launcher 控制面”的客户端-服务端分离方案，而非继续维护 native 本地直连进程 + web API 双路径。理由：双端行为可强制对齐，回归与维护成本显著下降。
