# oasis7：Web UI agent-browser 闭环测试操作手册

审计轮次: 9

## 文档定位
- 本文件是 Web UI `agent-browser` 闭环的 canonical `*.manual.md` 操作手册。
- 需求边界、成功标准与决策记录仍以 `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` 为准。
- 项目任务与历史执行状态仍以 `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md` 为准。

## 适用范围
- 适用于 `oasis7_viewer_live` / Viewer Web 页面闭环。
- 不适用于 `oasis7_web_launcher` / launcher Web 控制面产品动作；后者默认先走 GUI Agent，再用页面校验状态与字段。
- 本手册负责“具体怎么执行”；分层模型、suite 选择与 release 总口径仍以 `testing-manual.md` 为准。

## 前置条件
- 已安装 `agent-browser`，并可直接通过 `PATH` 调用。
- 已安装 `trunk`：`cargo install trunk`
- 已安装 wasm 目标：`rustup target add wasm32-unknown-unknown`
- 建议先执行一次 `agent-browser close-all`，避免残留会话影响本轮采样。

## 标准闭环
### 1. 启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

无 LLM 场景：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --no-llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

### 2. 启动 Web Viewer
```bash
env -u NO_COLOR ./scripts/run-viewer-web.sh --address 127.0.0.1 --port 4173
```

### 3. 打开 Viewer 页面并采样
默认要求 `--headed`，并固定硬件 WebGL 参数；如需额外浏览器参数，可通过 `AGENT_BROWSER_ARGS` 叠加。

```bash
command -v agent-browser >/dev/null || { echo "missing agent-browser" >&2; exit 1; }
mkdir -p output/playwright/viewer
agent-browser close-all || true
agent-browser --headed --args="--use-angle=gl,--ignore-gpu-blocklist" open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&test_api=1"
agent-browser wait --load networkidle
agent-browser snapshot -i
agent-browser eval "JSON.stringify(window.__AW_TEST__?.getState?.() ?? null)" | tee output/playwright/viewer/state.json
agent-browser console | tee output/playwright/viewer/console.log
agent-browser screenshot output/playwright/viewer/viewer-web.png
agent-browser close
```

## 最小通过标准
- `snapshot -i` 可见交互树，主视区正常加载。
- `state.json` 中 `connectionStatus=connected`，且 `lastError=null`。
- 至少产出 1 张截图、1 份 console 日志、1 份状态快照。
- `console.log` 中不出现 `copy_deferred_lighting_id_pipeline`、`CONTEXT_LOST_WEBGL`、`SwiftShader` 等图形 fatal 签名。

## software_safe 回归
- 若浏览器环境只能落到 software renderer，先看 `window.__AW_TEST__.getState().renderMode`。
- `renderMode=software_safe` 时，允许继续做最小闭环验证，并优先使用专用脚本：

```bash
./scripts/viewer-software-safe-chat-regression.sh --bundle-dir output/release/game-launcher-local
```

- 需要稳定观测一条标准 `AgentSpoke` 时，在 runtime 启动前显式设置：

```bash
OASIS7_RUNTIME_AGENT_CHAT_ECHO=1
```

- `agent_spoke` 缺失默认记为可追溯 warning；显式加 `--require-agent-spoke` 时才升级为阻断失败。

## launcher 控制面边界
- `oasis7_web_launcher` 的产品动作默认走 GUI Agent 接口：`/api/gui-agent/capabilities`、`/api/gui-agent/state`、`/api/gui-agent/action`。
- `agent-browser` 在 launcher 场景里只用于页面加载、字段核对和截图留证，不作为首选动作驱动器。

## Fail-Fast
- F1 `ERR_CONNECTION_REFUSED`：先检查 4173/5011 监听与主页可达，再重试。
- F2 渲染初始化崩溃：出现 `RuntimeError: unreachable`、`CONTEXT_LOST_WEBGL` 等时立即归档证据并判失败。
- F3 长时间 `connecting + tick=0`：先执行 `play` 并额外观察约 12 秒，仍不推进则失败。
- F4 URL 被 shell 截断：带 `&` 的 URL 一律加引号。
- headed 仍命中 `SwiftShader` / software renderer：`renderMode!=software_safe` 时直接按环境阻断，不得给出玩法结论。

## 发布与延伸入口
- 系统总手册：`testing-manual.md`
- 发布前人工体验清单：`doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
- QA loop：`./scripts/viewer-release-qa-loop.sh`
- full coverage：`./scripts/viewer-release-full-coverage.sh --quick`
