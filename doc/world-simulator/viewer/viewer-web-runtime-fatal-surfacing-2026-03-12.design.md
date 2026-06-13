# Viewer Web Runtime Fatal 透出与 SwiftShader 口径对齐设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.project.md`

## 1. 设计定位
将 Viewer 在 Web/agent-browser 环境下的“渲染已致命失败但状态仍停在 connecting”改成可观测、可快失败、可按手册分流处理的闭环。

## 2. 设计结构
- 运行时诊断层：在 `web_test_api` 中挂接浏览器级错误来源。
- 状态透出层：把 runtime fatal 合并到 `getState().connectionStatus/lastError/errorCount`。
- 自动化快失败层：`run-game-test-ab.sh` 遇到 `lastError` 立即失败并打印原因。
- 文档分流层：Viewer 手册显式声明 `test_api=1`、headed、software renderer 风险与 native fallback。

## 3. 关键接口 / 入口
- `window.__AW_TEST__.reportFatalError(message, source)`
- `window.__AW_TEST__.getState()`
- `scripts/run-game-test-ab.sh`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `testing-manual.md`

## 4. 约束与边界
- 不改 Bevy upstream，不承诺本轮修复 SwiftShader 兼容。
- 优先把“错误透明度”补齐，避免 producer/QA 把环境渲染故障误当成业务连接故障。
- Viewer 页面仍是 Web-first 默认链路；只有在 Web 已确认被图形链路阻断时，才转 native fallback。

## 5. 设计演进计划
- 先透出 fatal。
- 再让脚本快失败。
- 再同步 Viewer / testing 手册。
- 后续如需根治，再单开上游/渲染兼容专题处理。
