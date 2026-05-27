# Viewer Web 全屏自适应与右侧面板整体显隐（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] VWFP-1：输出设计文档（`doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md`）
- [x] VWFP-2：输出项目管理文档（本文件）
- [x] VWFP-3：实现 Web 端窗口全屏自适应（wasm 路径）
- [x] VWFP-4：实现右侧面板总开关（隐藏/显示）与 3D 区域联动
- [x] VWFP-5：调整右侧面板宽度策略为动态上限（非固定像素上限）
- [x] VWFP-6：补充/更新测试并执行验证（`test_tier_required` 相关 viewer 子集）
- [x] VWFP-7：更新使用手册与任务日志，回写状态并提交

## 依赖
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/i18n.rs`
- `crates/oasis7_viewer/src/panel_layout.rs`
- `crates/oasis7_viewer/src/egui_right_panel_tests.rs`
- `doc/world-simulator/viewer/viewer-manual.md`
- `doc/devlog/README.md`

## 状态
- 当前阶段：VWFP-1 ~ VWFP-7 全部完成。
- 最近更新：2026-02-21（Web 全屏 + 面板总开关 + 动态宽度 + Web 闭环 smoke 验证完成）。
