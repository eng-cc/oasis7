# oasis7 Web UI Playwright 实跑闭环测试手册

## 定位

本手册是 Playwright 实跑测试系列的入口文档，用来管理从真实浏览器进入游戏、执行玩家操作、等待 runtime/provider 返回、并归档证据的端到端测试。

它和 `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` 的关系：
- `agent-browser` manual 仍是通用 Web UI 闭环与截图采证入口。
- 本手册专管直接 Playwright 脚本，目标是逐步覆盖“人实际玩游戏会做的所有关键操作流程”。
- Playwright 系列可以使用 `window.__AW_TEST__` 做 readiness/state/断言读取，但玩家动作本身必须优先通过可见 UI 控件完成；除非用例明确标成 protocol-only/debug-only，否则不得用测试 API 代替点击、输入、选择或提交。

## 总目标

Playwright 实跑系列长期目标是形成一套可重复执行的玩家操作流程矩阵：

1. 启动纯本地真实栈，或复用一个已经按 runbook 启动的本地 test 环境 URL。
2. 打开真实 Viewer。
3. 完成玩家可见 UI 操作。
4. 覆盖 runtime / live websocket / viewer state / provider / message-flow 等链路。
5. 输出 screenshot、state、summary、console log 与明确 pass/fail。

这类测试回答的是“这条玩家操作链路在真实本地运行时是否闭环”。它不能单独证明游戏好玩，也不能替代 `L4B` embodied-agent playtest、内部真人校准或 `L5` 真实人类 / 线上样本。

## 当前入口

### PWT-001: Real Agent Chat

命令：

```bash
./scripts/viewer-real-agent-chat-regression.sh
```

复用已有 Viewer URL：

```bash
./scripts/viewer-real-agent-chat-regression.sh --url "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1&locale=zh"
```

若目标是本地启动 test 环境上的玩家 UI 实跑，先按 `doc/testing/manual/local-public-testnet-letai-test-environment-2026-06-23.manual.md` 启动并复核环境，再用 `--url` 指向该 runbook 输出的 URL。不要把 PWT-001 默认自启动栈当作 public_testnet attach-existing-node 证明；默认栈只证明纯本地真实 provider-backed 玩法链路。

覆盖链路：
- 本地真实 LetAI provider bridge。
- `oasis7_game_launcher` / `oasis7_viewer_live` / websocket bridge。
- 真实 Viewer 页面。
- UI 输入流程：进入沉浸模式、打开命令抽屉、填写 `#agent-chat-message`、点击 `button[data-chat-send="1"]`。
- runtime `agent_chat` ack。
- provider-backed Agent reply。
- Message Flow / `chatHistory` 中的真实 `AgentSpoke`。

通过标准：
- `summary.json` 中 `inputMode=ui`。
- `chatInputMode=ui`。
- 收到 `agentReply`。
- 回复包含默认 required fragments：`我在 runtime:`、`data 8`、`electricity 32`。
- 回复不得包含 `[local-mock-receipt]` 或 `[local-mock-chat]`。
- 脚本结束后默认清理自己启动的本地端口。

选择契约：
- WebGPU/canvas 渲染路径必须暴露稳定 DOM Agent marker：`[data-pixel-world-agent-marker="true"][data-agent-id="<agent-id>"]`。
- `PWT-001` 必须先点击该 marker，并在 summary 中记录 `selectionMode=ui-agent-marker`，再继续执行真实 UI 聊天输入/发送。
- 若该 marker 不可见，测试必须失败并保留 failure state / screenshot，而不是退回 preselected 兜底。

主要产物：

```text
output/playwright/viewer-real-agent-chat/<run-id>/
output/playwright/viewer-real-agent-chat/<run-id>/playwright/real-agent-chat-summary.json
output/playwright/viewer-real-agent-chat/<run-id>/playwright/final_state.json
output/playwright/viewer-real-agent-chat/<run-id>/playwright/real-agent-chat.png
output/playwright/viewer-real-agent-chat/<run-id>/playwright/browser-console.log
```

## 计划中的流程矩阵

新增用例时使用 `PWT-###` 编号。每个条目都必须说明玩家动作、真实依赖、断言、产物和边界。

| ID | 流程 | 状态 | 目标 |
| --- | --- | --- | --- |
| PWT-001 | Real Agent Chat | active | 覆盖真实 UI 输入聊天到真实 provider Agent 回复 |
| PWT-002 | Select Agent From World | planned | 覆盖从世界/地图可见目标选择 Agent，并验证 detail / command surface 切换 |
| PWT-003 | Submit Recommended Gameplay Action | planned | 覆盖点击推荐玩法动作、等待 ack/receipt/world delta |
| PWT-004 | Prompt Preview / Apply / Rollback | planned | 覆盖 prompt 控制面 UI 输入、预览、应用、回滚和版本刷新 |
| PWT-005 | Pause / Play / Progress Feedback | planned | 覆盖玩家控制 runtime 推进、阻塞提示和恢复反馈 |
| PWT-006 | Diagnostics And Recovery | planned | 覆盖连接异常、provider blocker、重试/刷新后的可理解报错 |
| PWT-007 | Pure API / Viewer Parity Spot | planned | 用同一场景交叉验证 Viewer UI 操作和 pure API 观测一致性 |

## 新增用例规则

每个新 Playwright 实跑用例必须满足：

- 默认启动真实本地栈；若允许 `--url` 复用已有栈，必须在 summary 中记录 `gameUrl`。
- 默认禁止 mock 作为通过证据；如果某用例确实是 mock plumbing smoke，文件名和 summary 必须显式标 `mock` / `debug-only`。
- 玩家动作优先使用可见 UI locator，例如 role、label、id、data attribute；`__AW_TEST__` 只用于 readiness、state snapshot、history 查证和失败诊断。
- 失败必须落盘 `failure_state.json`、截图、console log，并给出明确错误文本。
- summary 至少包含：`ok`、`caseId` 或脚本名、`gameUrl`、关键输入、关键输出、`inputMode`、mock 禁用/检测结果、artifact 路径。
- 如果脚本启动了本地栈，默认负责清理端口；需要保留时必须提供 `--keep-stack` 并在输出中写明。

## 运行前检查

推荐先确认：

```bash
lsof -nP -iTCP:4173 -iTCP:5011 -iTCP:5023 -iTCP:5841 -sTCP:LISTEN || true
bash -n scripts/viewer-real-agent-chat-regression.sh
```

如果当前 shell 没有 `node`，脚本会尝试使用 Codex bundled Node。若在非 Codex 环境运行，需要设置：

```bash
export OASIS7_NODE_BIN=/path/to/node
export OASIS7_PLAYWRIGHT_NODE_MODULES=/path/to/node_modules
```

## 证据判读

一次有效 Playwright 实跑证据至少需要：

- 命令行通过输出。
- `real-agent-chat-summary.json` 或同级 summary。
- `final_state.json`。
- screenshot。
- console log。
- 若涉及 provider/LLM，必须明确是真实 provider 路径，并有禁止 mock 的断言。

PWT 系列可以作为 S6 Web UI 闭环证据，也可以作为 `L4A synthetic` 的一部分输入；但只有当后续 L4 review packet、role cards、persona cards 或 L4B agent playtest 收口后，才能支撑对应 playability 结论。
