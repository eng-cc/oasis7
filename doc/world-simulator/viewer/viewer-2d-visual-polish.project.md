# Viewer 2D 可视化精修（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-2d-visual-polish.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-2d-visual-polish.prd.md`

审计轮次: 5

## 任务拆解（含 PRD-ID 映射）

### V2D0 文档与挂载
- [x] V2D0.1 输出设计文档（`doc/world-simulator/viewer/viewer-2d-visual-polish.prd.md`）
- [x] V2D0.2 输出项目管理文档（本文件）
- [x] V2D0.3 在总项目文档挂载分册入口

### V2D1 地图符号层（2D）
- [x] V2D1.1 Location 2D 地图符号（平面底板/中心点）
- [x] V2D1.2 Agent 2D 地图符号（平面高亮标记）
- [x] V2D1.3 2D/3D 模式切换联动（2D 显示、3D 隐藏）

### V2D2 标签可读性增强（2D）
- [x] V2D2.1 标签 LOD 增加 2D 配置分支（距离/容量/遮挡）
- [x] V2D2.2 单测补齐（2D 配置与可见性行为）
- [x] V2D2.3 回归验证与截图闭环

### V2D3 流向层工业化表达（2D）
- [x] V2D3.1 Flow Overlay 增加 2D 渲染参数分支（平面化/线宽增强）
- [x] V2D3.2 Flow 终点箭头头部渲染（Power/Trade）
- [x] V2D3.3 单测补齐与截图闭环

## 依赖
- `crates/oasis7_viewer/src/scene_helpers.rs`
- `crates/oasis7_viewer/src/scene_helpers_entities.rs`
- `crates/oasis7_viewer/src/camera_controls.rs`
- `crates/oasis7_viewer/src/label_lod.rs`
- `crates/oasis7_viewer/src/tests_scene_entities.rs`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

## 状态
- 当前阶段：V2D0~V2D3 已完成。
- 下一阶段：按新反馈评估是否进入 V2D4（2D 细节继续精修）。
- 最近更新：完成 V2D3（2D flow overlay 平面化 + 方向箭头 + 回归闭环，2026-02-15）。
