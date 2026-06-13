# Viewer Web Runtime Fatal 透出与 SwiftShader 口径对齐（2026-03-12）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.project.md`

审计轮次: 7

## 1. Executive Summary
- 真实 producer Web 闭环复测表明：launcher stale recovery 已恢复，但 Viewer 在 `agent-browser` 的 SwiftShader/WebGL2 环境下仍可能触发致命渲染失败（`copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL`）。
- 当前失败模式不是“明确报错”，而是页面保持 `connecting`、`logicalTime=0`，容易被误判为链路或使用方式问题。
- 本专题在保留 fatal 透出/快失败口径的同时，补充首开恢复修复：当 Viewer Web 命中已知图形 fatal（如 `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL`）时，前端只自动 reload 一次，尽量把“需要手动 reopen”收敛为产品内自恢复。

## 2. User Experience & Functionality

### In Scope
- 文件：`crates/oasis7_viewer/src/web_test_api.rs`
- 文件：`scripts/run-game-test-ab.sh`
- 文件：`doc/world-simulator/viewer/viewer-manual.manual.md`
- 文件：`testing-manual.md`
- 当 Web Viewer 遇到浏览器端致命错误时：
  - `window.__AW_TEST__.getState()` 必须给出 `connectionStatus="error"` 与非空 `lastError`。
  - 自动化脚本必须基于 `lastError` 快失败，而不是长时间卡在 `connecting`。
  - 手册必须明确 `test_api=1`、SwiftShader/software-renderer 判定与 native fallback 口径。

### Out of Scope
- 不修改 `third_party` 或 Bevy 上游实现。
- 不修改 `third_party` 或 Bevy 上游实现。
- 不承诺本轮内彻底消除所有 SwiftShader/WebGL2 渲染差异。
- 不改变 native Viewer、runtime 协议或 GUI Agent 接口语义。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- `web_test_api` 的 runtime diagnostic hook 新增有界自动恢复：
  - 命中 `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` / `wgpu error` 等已知图形 fatal 时，浏览器端只自动 `reload` 一次。
  - 第二次仍失败时不再循环 reload，而是保留 fatal 到 `__AW_TEST__` 供脚本/人工判定。
- `web_test_api` 新增浏览器运行时 fatal 采集：
  - `window.error`
  - `window.unhandledrejection`
  - `console.error` 中的已知渲染 fatal 签名
  - `webglcontextlost`
- `record_runtime_fatal_error()` 与 `publish_web_test_api_state()` 在存在 runtime fatal 时：
  - 立即把 `connectionStatus` 改写为 `error`
  - 立即把 fatal 消息写入 `lastError`
  - 立即递增 `errorCount`，避免 wasm panic 后快照停在旧值
- `run-game-test-ab.sh` 的连接等待逻辑：
  - 一旦 `lastError` 非空，直接返回 fatal 分支并输出错误原因。
- `viewer-manual.manual.md`：
  - Web 闭环命令统一使用 `agent-browser --headed open`。
  - 示例 URL 显式带上 `test_api=1`。
  - 说明 SwiftShader / `copy_deferred_lighting_id_pipeline` / `CONTEXT_LOST_WEBGL` 为环境/图形门禁失败，不进入玩法结论。

## 5. Risks & Roadmap
- M0：完成专题建档与模块主文档回写。
- M1：完成浏览器 runtime fatal 透出与自动化快失败改造。
- M2：完成 Viewer / testing 手册口径同步。
- M3：完成定向验证、devlog 与提交收口。

### Technical Risks
- 风险 1：`console.error` 关键字匹配过宽，可能把非阻断报错误记为 fatal。
  - 缓解：仅匹配当前已复现的渲染致命签名；后续按真实证据收敛名单。
- 风险 2：SwiftShader/WebGL2 环境仍可能出现其他未覆盖的 shader / context 差异。
  - 缓解：本轮先把高频 fatal 从“手动 reopen”收敛为“一次自动 reload + `__AW_TEST__` 快失败”，并保留手册门禁。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
