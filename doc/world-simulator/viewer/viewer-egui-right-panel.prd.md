# Viewer 右侧 2D UI 迁移到 bevy_egui（SidePanel）设计

- 对应设计文档: `doc/world-simulator/viewer/viewer-egui-right-panel.design.md`
- 历史迁移、截图与 task 状态：GitHub task issue evidence。

审计轮次: 5

## 1. Executive Summary
- 将 `oasis7_viewer` 当前右侧全部 2D UI 从 Bevy UI (`Node/Text/Button`) 迁移到 `bevy_egui`。
- `bevy_egui` 不再使用悬浮窗模式，统一为右侧固定 `SidePanel`。
- 保持现有业务状态、控制能力与中英文切换能力不退化。
- 保持 3D 视口交互边界正确（面板区域不触发 3D 相机与拾取）。

## 2. User Experience & Functionality
- **范围内**
  - 右侧面板全量迁移到 `egui::SidePanel::right`：
    - Top Controls（顶部折叠、语言切换、播放/暂停/单步/跳转）
    - 覆盖层控制、诊断区、事件联动区、时间轴控制区
    - 世界摘要、Agent 活动、选中详情、事件列表
  - 2D 交互按钮逻辑改由 EGUI 点击事件驱动。
  - 3D 视口宽度与鼠标命中边界改为读取 EGUI 右侧面板实时宽度。
  - 保留现有文本生成函数，减少业务逻辑重复。
- **范围外**
  - 不改造 3D 场景实体渲染与 3D 标签。
  - 不引入新的状态持久化机制。
  - 不重写 world/simulator 协议。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 右侧布局状态
- 新增 `EguiRightPanelState`（Resource）：
  - `width_px: f32`：右侧面板逻辑像素宽度（用于 3D 视口裁剪与命中边界）。

### 2) EGUI 渲染入口
- 新增 `render_right_side_panel_egui`（`EguiPrimaryContextPass`）：
  - 输出右侧固定 SidePanel。
  - 使用现有资源：`ViewerState`、`ViewerSelection`、`TimelineUiState`、`TimelineMarkFilterState`、`WorldOverlayConfig`、`DiagnosisState`、`EventObjectLinkState`、`UiI18n`。
  - 按钮事件直接更新资源与发送 `ViewerRequest::Control`。

### 3) 3D 视口边界
- `update_3d_viewport`、`orbit_camera_controls`、`pick_3d_selection` 读取 `EguiRightPanelState.width_px`。
- 面板宽度发生变化时，3D 相机 viewport 同步更新。

### 4) 文本可复制
- EGUI 面板正文统一使用可选中标签（`selectable(true)`），满足复制诉求。
- 中文字体继续通过 EGUI 字体注入 `ms-yahei.ttf` 保证 CJK 渲染。

## 5. Risks & Roadmap
- **M1**：文档与任务拆解完成。
- **M2**：EGUI SidePanel 骨架 + 顶部控制迁移。
- **M3**：摘要/详情/事件/诊断/联动/时间轴/覆盖层迁移。
- **M4**：移除旧 Bevy UI 右侧面板构建与交互调度。
- **M5**：截图闭环验证 + 回归测试 + 日志收口。

### Technical Risks
- **输入冲突**：EGUI 区域滚轮/拖拽干扰 3D 相机。
  - 缓解：统一按右侧面板边界过滤 3D 输入。
- **功能回归**：旧按钮系统迁移后遗漏状态更新。
  - 缓解：保留核心纯逻辑函数复用并补充定向测试。
- **布局漂移**：中英文文本长度差异导致区域拥挤。
  - 缓解：使用 EGUI 可折叠分组与滚动区域，避免固定高度截断。

## 6. Validation & Decision Record
- EGUI 遗留 surface 的实现历史、截图和 task 状态由 GitHub task issue evidence 追溯；它们不定义当前 SolidJS 主入口。
