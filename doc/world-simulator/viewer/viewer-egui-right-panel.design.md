# Viewer 右侧 EGUI SidePanel 迁移设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-egui-right-panel.prd.md`
- 历史迁移与验证：GitHub task issue evidence。

## 1. 设计定位
定义 Viewer 右侧 2D UI 向 `bevy_egui::SidePanel` 的迁移方案：统一右侧固定面板承载现有控制、摘要、诊断、事件和时间轴区域，同时保证 3D 视口交互边界与中英文能力不退化。

## 2. 设计结构
- 布局状态层：新增 `EguiRightPanelState`，统一暴露右侧面板宽度给相机、拾取和 viewport 裁剪逻辑。
- 面板渲染层：`render_right_side_panel_egui` 负责 SidePanel 骨架、折叠分组和滚动区域。
- 业务复用层：继续读取 `ViewerState`、`ViewerSelection`、`TimelineUiState`、`DiagnosisState` 等既有资源，避免重复状态源。
- 输入隔离层：右侧面板区域统一拦截鼠标与滚轮命中，避免 3D 相机与拾取误触发。

## 3. 关键接口 / 入口
- `EguiRightPanelState`
- `render_right_side_panel_egui`
- `update_3d_viewport` / `orbit_camera_controls` / `pick_3d_selection`
- `bevy_egui` 与中文字体注入

## 4. 约束与边界
- 本专题只迁移右侧 2D UI，不改 3D 场景实体渲染与世界协议。
- 现有文本生成与业务逻辑函数优先复用，不在迁移轮重写状态模型。
- 面板正文需保持可复制与中英文显示能力，避免迁移造成体验回退。
- 布局伸缩必须由实时面板宽度驱动，不能再依赖静态宽度常量。

## 5. 设计演进计划
- 先落 SidePanel 骨架与宽度状态。
- 再迁移顶部控制、摘要、详情、事件、诊断与时间轴。
- 最后清理旧 Bevy UI 路径并通过截图/测试收口。
