# oasis7 Viewer：Web Chat 面板锁重入与回车发送闭环修复（设计文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.project.md`

审计轮次: 5

## 1. Executive Summary
- 修复 Viewer Web 模式在 Chat 面板渲染时出现的 `parking_lot` panic（页面卡死）。
- 保证 Chat 输入在 Web 模式下可稳定发送消息到 Agent。
- 完成“写信息 -> 发送给 Agent”的 Web 闭环验证并产出证据。

## 2. User Experience & Functionality

### 范围内
- `oasis7_viewer` Web 运行时稳定性修复：
  - 修复 Chat 面板内 `egui::Context` 重入锁路径。
  - 避免 wasm 调试构建触发 `epaint` timed-lock 路径导致的 `Instant`/parking panic。
- Chat 输入发送判定修复：
  - 回车发送判定使用输入活跃态，兼容 wasm IME bridge 场景。
- Web 闭环验证：
  - 页面稳定运行无 panic。
  - 在 UI 输入消息并发送，看到“消息已发送”状态。

### 范围外
- 不改动 `ViewerRequest::AgentChat` / `ViewerResponse::AgentChat*` 协议定义。
- 不改动 Agent 决策内核和 trace 结构。
- 不改动本地 Viewer 交互设计。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 复用既有 `ViewerRequest::AgentChat` 请求结构。
- 复用 `AgentChatDraftState`，仅调整回车发送判定使用的焦点语义。
- 工程配置新增 `Cargo.toml` profile 项，约束 `epaint` 在 dev 下的 debug assertions 行为以适配 wasm。

## 5. Risks & Roadmap
- M1：定位 Web 卡死栈并确认锁重入触发点。
- M2：完成 Chat 面板重入修复与回车发送判定修复。
- M3：完成 wasm 编译、viewer 测试、Web 闭环验证。
- M4：文档/devlog 收口。

### Technical Risks
- 风险：调整输入活跃态判定可能影响 Enter/Shift+Enter 边界行为。
  - 缓解：保留 `modifiers.is_none()` 约束，并通过现有 Enter 判定单测回归。
- 风险：`epaint` profile 调整可能影响本地 debug 诊断体验。
  - 缓解：仅作用于 `epaint` 包，保持业务 crate 调试行为不变。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
