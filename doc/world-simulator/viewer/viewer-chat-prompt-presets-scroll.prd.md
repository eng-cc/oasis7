# Chat Panel 预设 Prompt 展开区滚动改造

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.project.md`

审计轮次: 5

## 1. Executive Summary
- 解决最右侧 Chat 面板在展开“预设 Prompt”后内容过高、超出可视区域的问题。
- 保证在小窗口或低分辨率下，预设编辑区与 Agent Prompt 字段可通过内部滚动完整访问。

## 2. User Experience & Functionality
- `crates/oasis7_viewer/src/egui_right_panel_chat.rs` 的预设 Prompt 展开区域布局。
- `doc/world-simulator/viewer/viewer-manual.manual.md` 的交互说明补充。
- 不改动 Prompt 语义、apply 逻辑、消息发送逻辑。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 无协议字段变更、无持久化格式变更。
- UI 侧新增“预设展开区最大滚动高度”布局常量，并基于可用高度计算 scroll 容器高度上限。

## 5. Risks & Roadmap
1. 输出设计文档与项目管理文档。
2. 在“预设 Prompt”展开状态下引入垂直 `ScrollArea`，包裹预设编辑 + Agent Prompt 草稿区域。
3. 补充布局计算测试与手册说明，执行 `test_tier_required` 回归。
4. 回写项目状态与 devlog 收口。

### Technical Risks
- 过小高度下滚动区可能压缩可见内容，需要设定合理上限策略并避免布局抖动。
- 需要避免滚动容器影响现有输入焦点和 Enter 发送行为。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
