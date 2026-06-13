# Viewer Chat 独立最右侧 Panel（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] DCR1 输出设计文档（`doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.prd.md`）
- [x] DCR2 输出项目管理文档（本文件）
- [x] DCR3 改造 EGUI 布局：拆分独立 Chat 右侧 Panel（最右侧）
- [x] DCR4 适配右侧总占用宽度并补充/更新测试
- [x] DCR5 更新手册、回写状态与 devlog 收口

## 依赖
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/egui_right_panel_chat.rs`
- `crates/oasis7_viewer/src/ui_state_types.rs`
- `crates/oasis7_viewer/src/camera_controls.rs`
- `crates/oasis7_viewer/src/selection_linking.rs`
- `crates/oasis7_viewer/src/egui_right_panel_tests.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 当前阶段：DCR1-DCR5 已全部完成。
- 下一步：等待验收；如需继续演进，可评估在窄屏下为 Chat panel 增加自动折叠策略。
- 最近更新：2026-02-16。
