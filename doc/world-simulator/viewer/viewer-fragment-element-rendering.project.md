# oasis7 Simulator：Viewer Fragment 元素材质渲染与开关（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-fragment-element-rendering.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] FER1：输出设计文档与项目管理文档
- [x] FER2：实现 fragment 元素材质库与分块渲染
- [x] FER3：接入 Overlay 开关与环境变量配置
- [x] FER4：补充测试并完成回归
- [x] FER5：更新说明书与 devlog 收口

## 依赖
- doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md
- `crates/oasis7_viewer/src/material_library.rs`
- `crates/oasis7_viewer/src/location_fragment_render.rs`
- `crates/oasis7_viewer/src/scene_helpers.rs`
- `crates/oasis7_viewer/src/world_overlay.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `doc/world-simulator/viewer/viewer-manual.manual.md`

## 状态
- 当前阶段：已完成
- 最近更新：完成 FER5（元素着色分块渲染 + 开关控制 + 测试回归，2026-02-10）
