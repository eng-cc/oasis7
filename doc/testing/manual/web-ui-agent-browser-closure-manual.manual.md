# oasis7：Web UI agent-browser 闭环测试操作手册

审计轮次: 10

## 文档定位
- 本文件是 Web UI `agent-browser` 闭环的 canonical `*.manual.md` 操作手册。
- 需求边界、成功标准与决策记录仍以 `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` 为准。
- 项目任务与历史执行状态仍以 `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md` 为准。

## 适用范围
- 适用于 `oasis7_viewer_live` + `software_safe` Viewer Web 页面闭环。
- 不适用于 `oasis7_web_launcher` / launcher Web 控制面产品动作；后者默认先走 GUI Agent，再用页面校验状态与字段。
- 本手册只覆盖当前仍存在的 Web 链路，不再覆盖历史 3D/native/visual-QA 工具。

## 前置条件
- 已安装 `agent-browser`
- 已安装 Node.js / npm
- 已安装 `python3`
- 建议先执行一次 `agent-browser close-all`

## 标准闭环

### 1. 启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

无 LLM 场景：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --no-llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
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
agent-browser --headed open "http://127.0.0.1:4173/?ws=ws://127.0.0.1:5011&render_mode=software_safe&test_api=1"
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
- `state.json` 中 `renderMode=software_safe`。
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

## 发布与延伸入口
- 系统总手册：`testing-manual.md`
- Viewer 操作手册：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 发布前人工体验清单：`doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md`
