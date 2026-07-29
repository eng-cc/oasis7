# oasis7 Viewer 使用说明书

审计轮次: 10

## 文档定位
- 本文件是 Viewer 使用说明的 canonical `*.manual.md` 入口。
- 历史兼容路径已删除；请直接使用本 canonical `*.manual.md` 手册。
- 系统级测试分层与 suite 选择仍以 `testing-manual.md` 为权威总入口。

## 目标
- 提供 `viewer` Viewer Web 主入口的统一操作手册，并说明 `software_safe` 兼容 alias。
- 统一 live server、Web 静态入口、agent-browser 闭环与常见排查步骤。
- 明确当前仓库已不再提供退役的第二 Viewer 工具链。

## 适用范围
- live server：`crates/oasis7 --bin oasis7_viewer_live`
- Web 静态入口：源码与发布 canonical 页面使用 `crates/oasis7_viewer/viewer.html`；`software_safe.html` 作为 compat HTML 副本继续同步产出
- Web 启动脚本：`scripts/run-viewer-web.sh`
- Web 回归脚本：
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression.sh`
  - `scripts/viewer-software-safe-chat-regression.sh`
- 边界说明：本手册只适用于 `viewer` Viewer Web 页面（兼容 `software_safe` alias），不适用于 `oasis7_web_launcher` / launcher Web 控制面；后者默认先走 GUI Agent。
- 本地真实 LetAI provider-backed 游戏试玩不从裸 `run-viewer-web.sh` 开始；统一使用 `./scripts/run-local-letai-game-test.sh` 启动完全本地的 bridge + runtime/game 栈。该入口不连接 formal/public testnet。

## 本地真实 LLM 游戏测试
```bash
./scripts/run-local-letai-game-test.sh --local-world-playtest
```

- 该入口负责 LetAI token config 规范化、默认 Rust direct `127.0.0.1:5841` provider bridge、Rust bridge chat probe/provider contract smoke 与 launcher/runtime/viewer 启动。
- 日常手工试玩只用 `--local-world-playtest`。该 preset 固化以下易错项：`--startup-profile playtest`、`--provider-smoke-mode skip`、`--reuse-existing-build`、`--detach`、默认 `--auto-play`、viewer/web/live 端口 `48420/48421/48422`、`--json-ready`，以及 wrapper 默认的本地 standalone chain。若需要复现暂停态或手动 Play 调试，可显式传 `--no-auto-play`。
- 启动后等 `<output-dir>/launcher/session.meta` 出现 `STACK_READY=1`，再打开其中 `GAME_URL`。默认日常 URL 形如 `http://127.0.0.1:48420/?ws=ws://127.0.0.1:48421&test_api=1&locale=zh`。
- 需要冷构建、严格 provider smoke 或换端口时，再显式展开参数；例如第一次构建可去掉 `--local-world-playtest` 或去掉 `--reuse-existing-build`，临时换 HTTP 端口可在末尾透传 `-- --viewer-port 4174 --json-ready`。
- 默认 chain-enabled 路径会启动 launcher-managed chain runtime，并通过 `--chain-local-standalone-test` 保持本地 submit -> commit -> snapshot 闭环可在单节点试玩栈内完成。该路径只有在 `output/chain-runtime/<node-id>/reward-runtime-execution-world/snapshot.json` 与 `journal.json` 都出现后，才算 chain-enabled 本地世界就绪。
- launcher 管理 `oasis7_chain_runtime`、进程编排与持久 execution-world readiness；Viewer 只观察 runtime/world 输出。snapshot/event 是观测面，恢复与 replay 仍以权威 state + journal/event chain 为准。
- 如果页面停在“认领已提交，正在等待链上 committed 快照同步”，先检查启动命令是否漏掉 local standalone chain 配置；漏掉时 gameplay action 可能已进入 pending consensus queue，但本地单节点不会完成 commit/snapshot。
- 如果页面短暂打开后出现 `viewer.ws` / WebSocket 错误，先检查脚本输出目录下的 `launcher/oasis7_viewer_live.log` 是否报 execution-world persistence ready gate，而不要先把它归因成 Viewer 前端问题。
- 如只为人工查看页面或排查 WebSocket，可在 wrapper 参数后透传 `--chain-disable` 作为临时 page-play mitigation；该模式不代表本地 standalone chain-enabled 世界启动通过，更不能作为 public_testnet 证据。
- 下方 `oasis7_viewer_live` + `run-viewer-web.sh` 流程只用于 Viewer/debug 或定向 Web 回归，不代表本地真实 provider-backed gameplay 栈。

## 底层 Viewer Debug 快速开始

### 1）启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

- `oasis7_viewer_live` 当前默认走 runtime/world 链路。
- `oasis7_viewer_live` 负责 Viewer live server 与可选 Web bridge，不内嵌 node、consensus、reward runtime、topology 或 execution-world persistence，也不提供 `--tick-ms` 作为外部推进时钟。它仍可通过 `--chain-status-bind` 观察 committed world，并在配置 status bind 时通过 `--chain-submit-bind` 提交 chain-linked action；这些是 client endpoints，不授予 node ownership。Viewer event delivery 不能等同于 node/consensus tick。
- 正式 gameplay 要求已配置且可连通的 LLM provider。
- 若显式改用 `--no-llm`，则该链路只可用于 observer/debug，不计入正式 gameplay 证据。

### 2）启动 Web Viewer
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

- 默认访问地址：`http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011`
- 当前仓库只提供 `viewer` 单一 Web / UI 入口；`software_safe` 只保留为兼容 alias，不再作为正式模式名。

### 3）前置依赖
- Node.js / npm
- `python3`
- 若要跑 agent-browser 闭环，还需安装 `agent-browser`

## 页面能力
- 当前页面聚焦 `viewer` 实时观察与正式玩法摘要。
- 支持 `locale=zh|en` 初始化和页面内中英文切换。
- 支持最小 prompt/chat 控制面；仅在 auth/bootstrap 可用时开放。
- 在 `hosted_public_join` 路径下，页面支持获取/释放 hosted `player_session`、`reconnect_sync` 恢复，以及 `prompt_control` 的 preview-grade `strong_auth`（需 `Backend Approval Code`）。
- 面向普通用户的启动入口现在默认且只暴露 `hosted_public_join`。旧的 `trusted_local_only` 本地可信预览不再作为可选用户流程；本地旧配置应迁移到 hosted join，并走邮箱 hosted account / player session 登录链路。
- `main_token_transfer` 仍保持阻断，页面只显示 lane verdict，不提供资产转账表单。
- 页面不再提供第二 Viewer 跳转，也不再承担退役视觉专项工具职责。

### 当前 Agent Chat 与高级 Prompt 设置

- Chat 和 Prompt 控制只对当前账号已绑定/权威认领且当前可控制的 Agent 开放；选中共享世界中的其他 Agent 不会授予控制权。无可控制 Agent 时，页面保持 blocked，并引导先认领 Agent 或等待 binding sync。
- `Agent Chat` 是面向当前 Agent 的消息入口。发送成功只表示对应请求结果，不会绕过 runtime 权威裁决，也不证明产生了世界效果。
- 当前 canonical Web 使用普通多行输入框和显式 `Send Chat` 动作；未定义 Enter 发送快捷键，Enter/Shift+Enter 按浏览器多行文本编辑处理。当前文档不声明中文 IME 组合态或自动聚焦已经获得跨浏览器专项验证。
- `Advanced Prompt Settings` 属于 operator-level 控制，默认收起，不与玩家主路径竞争。显式展开后，当前页面提供 system/short-term/long-term override、preview、apply、rollback 与最近 Prompt feedback。
- preview 不等于 apply；apply acceptance 不等于 runtime 已应用。以页面返回的实际 feedback 为准，失败或未授权必须保持 blocker/error，不能显示假成功。
- `hosted_public_join` 下的 Prompt apply 需要有效 `player_session` 与后端重新授权；当前入口使用 `Backend Approval Code` 满足 preview-grade `strong_auth`。缺失、过期或拒绝时应重新注册/认证或按页面提示恢复。
- 产品层的对话、草稿、默认/override 与反馈承诺见 [`Agent 对话与 Prompt 控制`](../../product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md)；自动化 grammar 与状态字段见 [`viewer-web-semantic-test-api.prd.md`](viewer-web-semantic-test-api.prd.md)。

### 已退役的 legacy 操作

历史 Standard-3D / egui 表面曾提供 auto select、右侧模块显隐及本地缓存、选中详情、Agent 快速定位、2D 全览图分层缩放和可复制文本面板；该表面及这些操作均已退役，不能作为当前 canonical `viewer` Web 页面的使用说明或能力声明。当前支持边界仍以实时观察、当前选中对象、prompt/chat、`window.__AW_TEST__` 以及本手册列出的三条回归脚本为准。

## 证据边界
- 当前 formal gameplay PASS 证据以 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 为准。
  - 该证据包含 2026-04-08 的 LLM-enabled follow-up PASS，结论是当前 Web 主入口已具备 release-grade formal gameplay 样本。
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md` 只证明 formal-release 技术启动状态在 `--no-llm` 观察链路下可加载、可连接、可选中实体。
  - 这份截图证据属于 observer/debug，不替代 formal gameplay 证明。

## Web 闭环

### 进程与 Launcher 边界

- `oasis7_viewer_live` 不启动或拥有 chain node、consensus gate、reward runtime、topology 或 execution-world directory。
- chain-linked formal/local evidence 使用 Launcher 管理的 `oasis7_chain_runtime`；Viewer 的 `--chain-status-bind` / `--chain-submit-bind` 只承担 client linkage。
- 退役的 embedded-node flags 必须失败并引导到 `oasis7_chain_runtime` / `oasis7_game_launcher`；编排、stale-world 与恢复合同见 `../launcher/game-client-launcher-runtime-session-continuity.prd.md`。
- `--no-llm` 仍只可作为 observer/debug，不是 formal gameplay evidence。

### 底层 Viewer Debug 人工闭环
终端 A：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

终端 B：
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

终端 C：
```bash
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
mkdir -p output/playwright/viewer
agent-browser close-all || true
agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"
agent-browser wait --load networkidle
agent-browser snapshot -i
agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)"
agent-browser console | tee output/playwright/viewer/console.log
agent-browser screenshot output/playwright/viewer/viewer-web.png
agent-browser close
```

### 推荐自动化脚本
- 主入口 contract：
```bash
./scripts/viewer-primary-web-entry-regression.sh --headed
```
- 实时玩法推进 / blocker 观测：
```bash
./scripts/viewer-software-safe-step-regression.sh --headed
```
- prompt/chat 回归：
```bash
./scripts/viewer-software-safe-chat-regression.sh --headed
```

## 最小通过标准
- 页面可加载，且 `window.__AW_TEST__` 可用。
- `getState().renderMode=viewer`（兼容 alias 场景可回出 `software_safe` 但不再是 canonical 期望）。
- `connectionStatus=connected`，或页面显式给出可追溯 blocker。
- 至少产出 1 张截图与 1 份 console/state 证据。

## 常用调试点
- `window.__AW_TEST__.getState()`
- `window.__AW_TEST__.sendControl("step")`
- `window.__AW_TEST__.sendPromptControl("preview", { agentId: "agent-0", shortTermGoal: "test" })`
- `window.__AW_TEST__.sendPromptControl("apply", { agentId: "agent-0", shortTermGoal: "test" })`
- `window.__AW_TEST__.sendAgentChat("agent-0", "hello from viewer")`

## 常见问题排查
- 页面空白：确认 `run-viewer-web.sh` 已完成构建并监听目标端口。
- 连接失败：确认 `oasis7_viewer_live` 已启动，且 `ws=` 参数与 `--web-bind` 一致。
- 无法进入正式玩法：检查 LLM provider 配置；若显式 `--no-llm`，只允许 observer/debug。
- `agent-browser` 失败：先检查 `agent-browser --version` 与浏览器依赖。
- 有状态但不推进：优先跑 `viewer-software-safe-step-regression.sh`，确认是正常推进还是显式 blocker。
- `test_api=1` 下停在 `connecting` 且 `logicalTime=0`：立即检查 `getState().lastError` / `errorCount` 并归档 state、console、screenshot。已知 WebGL/SwiftShader fatal 最多自动 reload 一次；再次失败是 S6 环境/图形 blocker，不得当作 gameplay 证据。只有确认 Web 图形阻塞后，native 才可作为诊断 fallback。
- 如果只看到 `--no-llm` 截图证据：不要把它当成 formal gameplay PASS；回到 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 看 LLM-enabled follow-up 结论。

## 已移除能力
- 退役的第二 Viewer 启动路径
- 退役的辅助可视化检视 surface
- 退役的视觉资产、抓帧与专项检视工具链

## 参考文档
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- `doc/world-simulator/viewer/viewer-web-entry-compatibility.prd.md`
- `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md`
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md`
