# oasis7 Viewer 使用说明书

审计轮次: 10

## 文档定位
- 本文件是 Viewer 使用说明的 canonical `*.manual.md` 入口。
- 历史兼容路径 `doc/world-simulator/viewer/viewer-manual.md` 仅保留跳转说明。
- 系统级测试分层与 suite 选择仍以 `testing-manual.md` 为权威总入口。

## 目标
- 提供 `viewer` Viewer Web 主入口的统一操作手册，并说明 `software_safe` 兼容 alias。
- 统一 live server、Web 静态入口、agent-browser 闭环与常见排查步骤。
- 明确当前仓库已不再提供旧 3D / native / 视觉专项工具链。

## 适用范围
- live server：`crates/oasis7 --bin oasis7_viewer_live`
- Web 静态入口：发布产物使用 `viewer.html`（canonical）/ `software_safe.html`（compat）；源码页面文件当前仍由 `crates/oasis7_viewer/software_safe.html` 生成并同步复制
- Web 启动脚本：`scripts/run-viewer-web.sh`
- Web 回归脚本：
  - `scripts/viewer-primary-web-entry-regression.sh`
  - `scripts/viewer-software-safe-step-regression.sh`
  - `scripts/viewer-software-safe-chat-regression.sh`
- 边界说明：本手册只适用于 `viewer` Viewer Web 页面（兼容 `software_safe` alias），不适用于 `oasis7_web_launcher` / launcher Web 控制面；后者默认先走 GUI Agent。

## 快速开始

### 1）启动 live server
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
```

- `oasis7_viewer_live` 当前默认走 runtime/world 链路。
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
- `main_token_transfer` 仍保持阻断，页面只显示 lane verdict，不提供资产转账表单。
- 页面不再提供 `standard Viewer` 跳转，也不再承担材质/theme/3D 视觉 QA 职责。

## 证据边界
- 当前 formal gameplay PASS 证据以 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 为准。
  - 该证据包含 2026-04-08 的 LLM-enabled follow-up PASS，结论是当前 Web 主入口已具备 release-grade formal gameplay 样本。
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md` 只证明 formal-release default world 在 `--no-llm` 观察链路下可加载、可连接、可选中实体。
  - 这份截图证据属于 observer/debug，不替代 formal gameplay 证明。

## Web 闭环

### 标准人工闭环
终端 A：
```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_viewer_live -- llm_bootstrap --llm --bind 127.0.0.1:5023 --web-bind 127.0.0.1:5011
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
- 如果只看到 `--no-llm` 截图证据：不要把它当成 formal gameplay PASS；回到 `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md` 看 LLM-enabled follow-up 结论。

## 已移除能力
- 原生 Viewer crate 启动路径
- 旧 3D / visual QA surface
- 旧材质、theme、抓帧与视觉专项工具链

## 参考文档
- `testing-manual.md`
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/testing/evidence/software-safe-primary-web-entry-evidence-2026-04-07.md`
- `doc/world-runtime/evidence/formal-release-fixed-genesis-default-viewer-2026-05-16.md`
