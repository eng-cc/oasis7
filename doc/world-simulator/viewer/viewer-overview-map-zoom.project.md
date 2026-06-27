# Viewer 全览图缩放切换（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-overview-map-zoom.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] OVZ1.1 输出设计文档（`doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md`）
- [x] OVZ1.2 输出项目管理文档（本文件）
- [x] OVZ2.1 引入 `TwoDZoomTier` 与缩放阈值状态机（含迟滞）
- [x] OVZ2.2 调整 2D 默认缩放口径，默认进入细节态（适合观察 Agent）
- [x] OVZ3.1 全览图可见性联动（`TwoDMapMarker` 与细节实体）
- [x] OVZ3.2 更新/新增相关单测并执行 `test_tier_required` 回归
- [x] OVZ3.3 修复启动首帧“自动聚焦后被默认半径回写”问题（`auto_focus` × `sync_camera_mode`）
- [x] OVZ3.4 修复场景层级可见性链（scene root / location anchor 补齐 `Visibility`）
- [x] OVZ3.5 全览图标记层策略收敛（仅 Overview 可见 + Overview 缩放放大）
- [x] OVZ4.1 更新总项目文档状态与开发日志

## 依赖
- `crates/oasis7_viewer/src/camera_controls.rs`
- `crates/oasis7_viewer/src/scene_helpers.rs`
- `crates/oasis7_viewer/src/location_fragment_render.rs`
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `doc/world-simulator/project.md`

## 状态
- 当前阶段：OVZ1~OVZ4 已完成。
- 下一步：无（本任务收口完成）。
- 最近更新：2026-02-15（修复首帧缩放回写、可见性链路与全览图标记层行为，完成回归）。
