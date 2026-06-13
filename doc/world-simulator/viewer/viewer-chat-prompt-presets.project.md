# Viewer Chat 预设 Prompt 编辑区（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-prompt-presets.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] VCP1 输出设计文档（`doc/world-simulator/viewer/viewer-chat-prompt-presets.prd.md`）
- [x] VCP2 输出项目管理文档（本文件）
- [x] VCP3 删除 Prompt Ops 模块入口与面板接线
- [x] VCP4 在最右侧 Chat Panel 增加可展开预设 Prompt 编辑区（新增/编辑/删除/填充）
- [x] VCP5 更新测试并执行回归（`test_tier_required`）
- [x] VCP6 回写文档状态与 devlog，完成收口提交

## 依赖
- `crates/oasis7_viewer/src/main.rs`
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `crates/oasis7_viewer/src/i18n.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/egui_right_panel_chat.rs`
- `crates/oasis7_viewer/src/egui_right_panel_tests.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 当前阶段：已完成（VCP1-VCP6）。
- 下一步：等待验收；如需跨会话保留预设，可追加“预设持久化”子任务。
- 最近更新：2026-02-16。
