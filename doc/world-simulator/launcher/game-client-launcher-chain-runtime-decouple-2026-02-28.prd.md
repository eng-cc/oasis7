# 启动器链路重构：链运行时与游戏进程解耦（2026-02-28）

- 对应设计文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.design.md`
- 对应项目管理文档: `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.project.md`

审计轮次: 6

## 1. Executive Summary
- 将 P2P 节点/链运行时从游戏进程层（`oasis7_viewer_live`）中拆出，改为独立进程承载。
- `oasis7_game_launcher` 默认一键拉起“游戏服务 + 链运行时 + Web 静态服务”，实现对外统一发行入口。
- 在 launcher 链路中提供链配置能力，并暴露可观测接口（含 token 余额视图）。

## 2. User Experience & Functionality
- 新增独立二进制：`oasis7_chain_runtime`（归属 `oasis7` crate）。
- `oasis7_chain_runtime` 负责：
  - 启动/停止 NodeRuntime；
  - 维护执行世界（execution world）落盘路径；
  - 暴露 HTTP 状态接口（health/status/balances）。
- `oasis7_game_launcher` 负责：
  - 启动并托管 `oasis7_chain_runtime` 子进程（默认启用）；
  - 启动 `oasis7_viewer_live` 纯观察服务（不再承载内嵌节点控制面）；
  - 透传链配置参数并输出链状态入口 URL。
- 更新发行打包脚本：将 `oasis7_chain_runtime` 纳入 bundle。

## 非目标
- 本轮不实现复杂钱包 UI、助记词管理、链上转账签名流程。
- 本轮不让 `oasis7_viewer_live` 接收 `--node-*`/`--triad-*` 控制面参数（节点控制面统一由 `oasis7_chain_runtime` 承载）。
- 本轮不覆盖跨机器大规模集群编排（先覆盖单机发行链路）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
### 1) `oasis7_chain_runtime` CLI（新增）
- `--node-id <id>`：默认 `viewer-live-node`。
- `--world-id <id>`：技术运行分区标识；当前默认见 `oasis7_chain_runtime` 的 `DEFAULT_WORLD_ID`，用于统一持久大世界的 runtime state/cache，不是玩家世界名。
- `--status-bind <host:port>`：默认 `127.0.0.1:5121`。
- `--node-role <sequencer|storage|observer>`：默认 `sequencer`。
- `--node-tick-ms <n>`：默认 `200`，仅用于 worker 轮询/回退间隔。
- `--pos-slot-duration-ms <n>`：默认见 `config/chain-pos-defaults.env` 的 `POS_SLOT_DURATION_MS`，定义 slot 时长（ms）。
- `--pos-ticks-per-slot <n>`：默认见 `config/chain-pos-defaults.env` 的 `POS_TICKS_PER_SLOT`，定义每个 slot 的逻辑 tick 数。
- `--pos-proposal-tick-phase <n>`：默认见 `config/chain-pos-defaults.env` 的 `POS_PROPOSAL_TICK_PHASE`，定义提案相位。
- `--pos-adaptive-tick-scheduler` / `--pos-no-adaptive-tick-scheduler`。
- `--pos-slot-clock-genesis-unix-ms <n>`：可选，定义 slot 时钟起点。
- `--pos-max-past-slot-lag <n>`：默认见 `config/chain-pos-defaults.env` 的 `POS_MAX_PAST_SLOT_LAG`，定义最大允许过旧槽滞后。
- `--node-validator <id:stake>`：可重复；为空时默认单 validator（本节点）。
- `--node-auto-attest-all` / `--node-no-auto-attest-all`。
- `--node-gossip-bind <addr:port>` / `--node-gossip-peer <addr:port>`（可选）。
- `--execution-world-dir <path>`：默认 `output/chain-runtime/execution-world`。
- `--no-openapi`：可选（当前仅纯 JSON HTTP）。

### 2) `oasis7_chain_runtime` HTTP 接口（新增）
- `GET /healthz`：存活探针。
- `GET /v1/chain/status`：节点运行状态、共识高度、错误信息。
- `GET /v1/chain/balances`：从 execution world 读取
  - `node_asset_balances`
  - `reward_mint_records`（最近记录）
  - `main_token_balances`（若存在）

### 3) `oasis7_game_launcher` CLI 扩展
- `--chain-enable` / `--chain-disable`（默认 enable）。
- `--chain-status-bind <host:port>`：默认 `127.0.0.1:5121`。
- `--chain-node-id <id>`。
- `--chain-world-id <id>`（默认跟随 scenario 推导）。
- `--chain-node-role <role>`。
- `--chain-node-tick-ms <n>`（worker poll/fallback interval）。
- `--chain-pos-slot-duration-ms <n>`（默认见 `config/chain-pos-defaults.env` 的 `POS_SLOT_DURATION_MS`）。
- `--chain-pos-ticks-per-slot <n>`（默认见 `config/chain-pos-defaults.env` 的 `POS_TICKS_PER_SLOT`）。
- `--chain-pos-proposal-tick-phase <n>`（默认见 `config/chain-pos-defaults.env` 的 `POS_PROPOSAL_TICK_PHASE`）。
- `--chain-pos-adaptive-tick-scheduler` / `--chain-pos-no-adaptive-tick-scheduler`。
- `--chain-pos-slot-clock-genesis-unix-ms <n>`。
- `--chain-pos-max-past-slot-lag <n>`（默认见 `config/chain-pos-defaults.env` 的 `POS_MAX_PAST_SLOT_LAG`）。
- `--chain-node-validator <id:stake>`（repeatable）。

### 4) 关键链路
- 桌面入口：`run-client.sh -> oasis7_client_launcher -> oasis7_game_launcher`。
- CLI 入口：`run-game.sh -> oasis7_game_launcher`。
- 启动器编排：`oasis7_game_launcher -> oasis7_chain_runtime + oasis7_viewer_live + static_http`。

## 5. Risks & Roadmap
- M1：`oasis7_chain_runtime` 落地（节点主循环 + status/balances API）。
- M2：`oasis7_game_launcher` 完成链子进程托管，viewer 进程收敛为纯观察服务。
- M3：发行打包与桌面启动器参数透传完成。
- M4：回归测试、文档收口、发布口径验证。

### Technical Risks
- 运行时状态拆分后，链状态与游戏状态可能出现可观测时序差。
  - 缓解：status/balances 接口明确 `observed_at_unix_ms` 与数据来源路径。
- 默认启用链运行时会增加启动依赖（端口冲突、文件权限）。
  - 缓解：提供 `--chain-disable` 快速降级路径。
- token 余额来源于 execution world，早期 run 可能为空。
  - 缓解：接口显式返回空集合与原因字段，不视为错误。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
