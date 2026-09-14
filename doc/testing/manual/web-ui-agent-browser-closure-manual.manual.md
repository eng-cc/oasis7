# oasis7：Web UI agent-browser 闭环测试操作手册

审计轮次: 10

## 文档定位
- 本文件是 Web UI `agent-browser` 闭环的 canonical `*.manual.md` 操作手册。
- 需求边界、成功标准与决策记录仍以 `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` 为准。
- 任务与历史执行状态以 GitHub task issue evidence comments 为准。

## 适用范围
- 适用于 `oasis7_viewer_live` + `software_safe` Viewer Web 页面闭环。
- 不适用于 `oasis7_web_launcher` / launcher Web 控制面产品动作；后者默认先走 GUI Agent，再用页面校验状态与字段。
- 本手册只覆盖当前仍存在的 Web 链路，不再覆盖历史第二 Viewer 或退役视觉专项工具。
- 若目标是纯本地真实 LetAI provider-backed 游戏试玩或复现 `agent_chat`，先使用 `./scripts/run-local-letai-game-test.sh --local-world-playtest` 启动完整 bridge + runtime/game/local-standalone-chain 栈；下方 live server + `run-viewer-web.sh` 步骤只作为 Viewer/debug 闭环。该入口不连接 formal/public testnet；纯本地测试不等同于“本地启动 test 环境”。

## 前置条件
- 已安装 `agent-browser`
- 已安装 Node.js / npm
- 已安装 `python3`
- 建议先执行一次 `agent-browser close-all`

## 纯本地真实 LLM 游戏测试入口
```bash
./scripts/run-local-letai-game-test.sh --local-world-playtest
```

- 该入口会统一处理 LetAI token config、默认 Rust direct `127.0.0.1:5841` provider bridge 与 launcher/runtime/viewer/local-standalone-chain 启动，并固化日常试玩端口 `48420/48421/48422`。
- 低层调试参数仍保留给专项排障；需要完整列表时执行 `./scripts/run-local-letai-game-test.sh --help-all`。
- 需要对已经启动的页面做 agent-browser 留证时，优先复用脚本输出的 `GAME_URL`，再执行本手册的采样步骤。
- 不要把单独的 `run-viewer-web.sh` 当作本地真实 provider-backed gameplay 启动方式；它不负责 provider bridge 或 launcher bootstrap。
- 若目标是本地启动 test 环境并证明本机入口接入 formal `public_testnet` 大世界，而不是纯本地 LetAI playtest 栈，先按 `doc/testing/manual/local-public-testnet-letai-test-environment-2026-06-23.manual.md` 启动并复核，再用本手册做页面采样。

### W3：DevLocal Builtin LLM 免邮箱启动

W3 的本地验收需要真实 WASM 与 build-suite metadata、DevLocal authority、Builtin LLM
和 HostedPublicJoin 的 loopback session projection。它不使用 ProviderBacked authority，
也不连接 formal/public testnet。先确认本 worktree 构建出的 launcher、viewer、chain
runtime 和 probe 来自同一个 HEAD，并确认真实 artifact 存在：

```bash
ROOT_DIR="$(pwd)"
LOCAL_PROVIDER_DIR="$ROOT_DIR/.tmp/wasm-build-suite/local-test-provider"
test -f "$LOCAL_PROVIDER_DIR/module.runtime.local-test-provider.wasm"
test -f "$LOCAL_PROVIDER_DIR/module.runtime.local-test-provider.metadata.json"
env -u RUSTC_WRAPPER cargo build -p oasis7 \
  --bin oasis7_llm_provider_probe \
  --bin oasis7_game_launcher \
  --bin oasis7_viewer_live \
  --bin oasis7_chain_runtime
```

为每次运行选择新的输出目录和空闲端口；下面端口只是示例。显式开启 loopback 的免邮箱
test-login，并将四个 local authority 参数一起传给 `run-launcher-stack.sh`：

```bash
RUN_ID="w3-local-builtin-$(date +%Y%m%d-%H%M%S)"
OUTPUT_DIR="$ROOT_DIR/.pm/scratch/w3-real-qa/$RUN_ID"
mkdir -p "$OUTPUT_DIR"
export OASIS7_HOSTED_TEST_LOGIN_ENABLED=1
export OASIS7_LOCAL_TEST_PROVIDER_SESSION_MODE=hosted_public_join

./scripts/run-launcher-stack.sh \
  --run-id "$RUN_ID" \
  --output-dir "$OUTPUT_DIR" \
  --viewer-host 127.0.0.1 \
  --viewer-port 4289 \
  --live-bind 127.0.0.1:5189 \
  --web-bind 127.0.0.1:5289 \
  --deployment-mode trusted_local_only \
  --allow-trusted-local-playtest \
  --chain-enable \
  --chain-local-standalone-test \
  --agent-decision-source builtin_llm \
  --skip-llm-provider-preflight \
  --local-test-provider-authority "$OUTPUT_DIR/local-test-provider-authority.json" \
  --local-test-provider-wasm "$LOCAL_PROVIDER_DIR/module.runtime.local-test-provider.wasm" \
  --local-test-provider-metadata "$LOCAL_PROVIDER_DIR/module.runtime.local-test-provider.metadata.json" \
  --local-test-provider-agent-id starter-agent-0 \
  --local-test-provider-owner-binding local-test-owner-0 \
  --local-test-provider-finality-block-hash "blake3:0000000000000000000000000000000000000000000000000000000000000000" \
  --local-test-provider-session-mode hosted_public_join \
  --with-llm \
  --auto-play \
  --json-ready
```

The launcher defaults its chain storage profile to `dev_local`; direct
`oasis7_game_launcher` invocations must add `--chain-storage-profile dev_local` explicitly.
The wrapper creates run-scoped file account/session/replay ledgers and a local issuer key in
the process environment; do not replace those with values copied into a command, report, or
task log. Keep `--agent-decision-source builtin_llm`: the local authority opt-in is rejected
for `provider_backed`.

Check the output `session.meta` for `STACK_READY=1`, then verify the actual chain process and
the email-free issuer route:

```bash
CHAIN_STATUS_BIND="127.0.0.1:5399"
curl -sS "http://$CHAIN_STATUS_BIND/v1/chain/status" | jq '{ok, readiness: .readiness.status, h: .consensus.committed_height, nh: .consensus.network_committed_height, runtime_last_error}'
curl -sS -X POST "http://127.0.0.1:4289/api/public/hosted-account/test-login" \
  -H 'Content-Type: application/json' \
  -d '{"public_key":"4848484848484848484848484848484848484848484848484848484848484848"}' \
  | jq '{ok, player_id, has_registration_grant: (.registration_grant != null)}'
```

The chain process smoke is green only when status is `ok=true`, readiness is `ready`, the
committed height advances to at least 3, and the persisted execution snapshot time matches
that height. The local authority setup leaves the persisted world at baseline height 2, so
the first real proposal is height 3; do not synthesize a height-2 record or reset the world
clock. A fresh DevLocal setup also applies the canonical 325 OC genesis and immediate vesting
claim for `starter-agent-0` at that baseline without advancing the clock; reuse validates the
immutable funding journal and does not refill a spent balance. A subsequent restart should
restore height 3 and advance to height 4. This process
smoke is separate from the focused Rust driver regression and is required before browser
claims for ordinary starter completion.

## 底层 Viewer Debug 闭环

### 1. 启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

无 LLM 场景：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --allow-debug-scenario --no-llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

### 2. 启动 Web Viewer
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

### 3. 打开页面并采样
```bash
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
mkdir -p output/playwright/viewer
agent-browser close-all || true
agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=viewer&test_api=1"
agent-browser wait --load networkidle
agent-browser snapshot -i
agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)" | tee output/playwright/viewer/state.json
agent-browser console | tee output/playwright/viewer/console.log
agent-browser screenshot output/playwright/viewer/viewer-web.png
agent-browser close
```

## 推荐回归脚本
- 主入口 contract：
```bash
./scripts/viewer-primary-web-entry-regression.sh --headed
```
- 实时推进 / blocker：
```bash
./scripts/viewer-software-safe-step-regression.sh --headed
```
- prompt/chat：
```bash
./scripts/viewer-software-safe-chat-regression.sh --headed
```

## 最小通过标准
- `snapshot -i` 可见交互树，主视区正常加载。
- `state.json` 中 `renderMode=viewer`（compat 专项可接受 `software_safe` alias）。
- `connectionStatus=connected`，或页面显式返回可追溯 blocker。
- 至少产出 1 张截图、1 份 console 日志、1 份状态快照。

## software_safe 专项
- 若只做 formal gameplay / summary 路径，优先跑 `viewer-software-safe-step-regression.sh`。
- 若验证 prompt/chat/rollback，优先跑 `viewer-software-safe-chat-regression.sh`。
- 若需要稳定观测一条标准 `AgentSpoke`，可在 runtime 启动前设置：
```bash
OASIS7_RUNTIME_AGENT_CHAT_ECHO=1
```

## launcher 控制面边界
- `oasis7_web_launcher` 的产品动作默认走 GUI Agent 接口：
  - `/api/gui-agent/capabilities`
  - `/api/gui-agent/state`
  - `/api/gui-agent/action`
- `agent-browser` 在 launcher 场景里只用于页面加载、字段核对和截图留证。

## Fail-Fast
- F1 `ERR_CONNECTION_REFUSED`：先检查 4173/5011 监听与主页可达。
- F2 页面初始化崩溃：立即归档证据并判失败。
- F3 长时间无推进：优先使用 `viewer-software-safe-step-regression.sh` 判断是正常 blocker 还是异常卡死。
- F4 URL 被 shell 截断：带 `&` 的 URL 一律加引号。
- F5 `connecting` 且 `logicalTime=0`：读取 `window.__AW_TEST__.getState().lastError` / `errorCount`，归档 screenshot、console 与 state；WebGL/SwiftShader fatal 按 S6 环境/图形失败分类，已知 fatal 最多允许一次自动 reload，不能当作 gameplay 进展。

## 发布与延伸入口
- 系统总手册：`testing-manual.md`
- Viewer 操作手册：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 发布前人工体验清单：`doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
