# Viewer 控制区精简与高级调试折叠（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）
- [x] ADF1 输出设计文档（`doc/world-simulator/viewer/viewer-control-advanced-debug-folding.prd.md`）
- [x] ADF2 输出项目管理文档（本文件）
- [x] ADF3 实现 EGUI 控制区改造（播放/暂停单按钮 + 高级调试折叠）
- [x] ADF4 补充/更新测试并执行回归（`test_tier_required` 最小闭环）
- [x] ADF5 更新手册、回写状态与 devlog 收口

## 依赖
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/i18n.rs`
- `crates/oasis7_viewer/src/egui_right_panel_tests.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 当前阶段：已完成（ADF1-ADF5）。
- 下一步：等待验收反馈。
- 最近更新：2026-02-16。
