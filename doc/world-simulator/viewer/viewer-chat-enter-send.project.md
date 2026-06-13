# Viewer Chat 回车发送（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-enter-send.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-enter-send.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] CES1 输出设计文档（`doc/world-simulator/viewer/viewer-chat-enter-send.prd.md`）
- [x] CES2 输出项目管理文档（本文件）
- [x] CES3 实现 Chat 输入框回车发送并补充测试
- [x] CES4 更新手册、回写状态与 devlog 收口

## 依赖
- `crates/oasis7_viewer/src/egui_right_panel_chat.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `site/doc/cn/viewer-manual.html`
- `site/doc/en/viewer-manual.html`

## 状态
- 当前阶段：CES1-CES4 已全部完成。
- 下一步：等待验收；如需扩展可评估配置化快捷键与“Ctrl/Cmd+Enter 发送”模式。
- 最近更新：2026-02-16。
