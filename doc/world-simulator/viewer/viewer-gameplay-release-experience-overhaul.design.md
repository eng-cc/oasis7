# Viewer 发行体验总改造设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- 历史阶段实施与验收记录：GitHub task issue evidence。

## 1. 设计定位
定义 Viewer 从调试工具向可发行游戏前端切换的总体方案：以 Player/Director 双模式为主轴，重排默认面板、目标提示和反馈层，让世界画面与玩家目标成为默认焦点。

## 2. 设计结构
- 模式分流层：`ViewerExperienceMode` 统一区分 `player` 与 `director` 两套默认行为。
- 默认布局层：按模式覆盖右侧面板显隐、顶部折叠和模块可见性，降低玩家默认噪声。
- 目标反馈层：把主任务、事件反馈、引导 HUD 和阶段提示组织成持续可玩的前端体验。
- 阶段治理层：Phase 2~10 子专题承接沉浸、引导、反馈和降噪细化任务，并最终回并到总文档。

## 3. 关键接口 / 入口
- `OASIS7_VIEWER_EXPERIENCE_MODE`
- `ViewerExperienceMode`
- `RightPanelLayoutState` / `RightPanelModuleVisibilityState`
- `egui_right_panel.rs` 与相关测试

## 4. 约束与边界
- 总改造只重排 Viewer 体验，不改网络协议、仿真核心逻辑和 `third_party`。
- Player 模式强调世界优先，但不能牺牲 Director 模式的可观测性和调试能力。
- 阶段合并后由总文档作为唯一权威入口，避免 phase 文档与总文档口径漂移。
- 所有新增交互都必须能被 Web 闭环验证，避免只停留在视觉描述。

## 5. 设计演进计划
- 先冻结 Player/Director 双模式与默认布局策略。
- 再分阶段推进沉浸、引导、反馈和入口减噪。
- 最后将阶段成果物理合并回总文档并完成验收收口。
