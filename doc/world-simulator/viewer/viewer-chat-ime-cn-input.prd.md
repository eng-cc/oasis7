# oasis7 Simulator：Viewer Chat 中文输入兼容修复（设计文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-ime-cn-input.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-chat-ime-cn-input.project.md`

审计轮次: 5

## 1. Executive Summary
- 修复 Viewer 右侧 Agent Chat 输入框无法稳定输入中文（IME 组合输入）的问题。
- 保持现有英文输入、快捷键、控制台与 WebSocket 链路行为不回退。
- 保持现有 `oasis7_viewer_live + run-viewer-web.sh` Web 闭环流程可用。

## 2. User Experience & Functionality

### In Scope
- 调整 `oasis7_viewer` 的 Web 窗口输入事件处理配置，避免中文输入法组合态被提前拦截。
- 保持 Chat 模块现有消息发送协议、UI 结构和 trace 展示逻辑不变。
- 增加/更新必要的回归校验（`test_tier_required`）与 wasm 目标编译检查。

### Out of Scope
- 聊天模块交互重设计（例如消息气泡样式、快捷键体系变更）。
- LLM 服务端消息处理策略改造。
- Native 平台输入法增强（本次优先 Web 链路）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 不新增协议字段，不修改 `ViewerRequest::AgentChat` 或 `ViewerResponse::*`。
- 仅调整 Viewer 窗口初始化参数（Web 平台）以改善浏览器输入法事件链路。
- 预期行为：
  - 中文输入法候选词可正常上屏到 Chat 输入框。
  - 已有英文输入与消息发送行为不变。

## 5. Risks & Roadmap
- M1：完成设计与任务拆解文档。
- M2：完成 Web 输入事件配置修复与代码提交。
- M3：完成 `test_tier_required` 校验、wasm 编译检查与 Web 闭环截图取证。
- M4：更新项目文档状态与 devlog 收口。

### Technical Risks
- Web 端按键默认行为调整可能影响部分浏览器快捷键体验。
- 不同浏览器 IME 事件差异可能导致边缘行为不一致。
- 若输入法兼容问题来自上游依赖实现限制，可能需要后续补充更深层 workaround。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
