# Viewer 发行体验改造（项目管理文档）

- 对应设计文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md`
- 对应需求文档: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`

审计轮次: 5

## ROUND-002 物理合并
- 本文件为项目主入口文档（当前权威入口）。
- `immersion-phase8~10` 项目内容已物理合并入本文件，对应阶段项目文档已合并并从仓库移除（不再保留 archive 目录）。
- `immersion-phase2~7` 旧阶段项目三件套已退役删除；其完成态追溯只保留在 core review logs、git history 与 GitHub task issue evidence comments，当前执行/状态入口统一以本文件为准。

## 任务拆解（含 PRD-ID 映射）
- [x] VGRO0：建立设计文档与项目管理文档。
- [x] VGRO1：实现体验模式资源与环境变量解析（Player/Director）。
- [x] VGRO2：实现按模式覆盖默认布局与默认模块可见性。
- [x] VGRO3：实现 Player 模式入口可发现性增强（提示文案/入口行为）。
- [x] VGRO4：补充/更新测试并执行 viewer 相关回归。
- [x] VGRO5：回顾设计文档与项目文档，更新状态并收口。

## Phase 8~10 增量任务记录（已合并）

### Phase 8
- [x] VRI8P0：建立第八阶段设计文档与项目管理文档。
- [x] VRI8P1：收敛“下一步目标卡”与“新手引导卡”共存策略，去除重复提示。
- [x] VRI8P2：改造任务 HUD 的自适应锚点与展开态紧凑模式。
- [x] VRI8P3：执行回归与 Web 闭环验收并完成文档收口。

### Phase 9
- [x] VRI9P0：建立第九阶段设计文档与项目管理文档。
- [x] VRI9P1：实现顶部布局预设条减噪策略（隐藏态移除 + 锚点重排）并补单测。
- [x] VRI9P2：实现隐藏态任务 HUD“一键指挥 Agent”入口并补单测。
- [x] VRI9P3：执行回归与 Web 闭环验收并完成文档收口。

### Phase 10
- [x] VRI10P0：建立第十阶段设计文档与项目管理文档。
- [x] VRI10P1：修复教程进度跳步并强化第 4 步动作文案（含单测）。
- [x] VRI10P2：实现隐藏态引导层减噪策略并完成定向验证。
- [x] VRI10P3：执行回归与 agent-browser 新手流程复测并完成文档收口。

## 依赖
- doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md
- `crates/oasis7_viewer/src/app_bootstrap.rs`
- `crates/oasis7_viewer/src/egui_right_panel.rs`
- `crates/oasis7_viewer/src/right_panel_module_visibility.rs`
- `crates/oasis7_viewer/src/ui_state_types.rs`
- `crates/oasis7_viewer/src/tests.rs`
- `testing-manual.md`

## 状态
- 当前阶段：VGRO0~VGRO5 全部完成（项目收口）。
- 阻塞项：无。
- 最近更新：完成设计文档与项目文档终态回写，发行体验改造任务闭环。
- 备注：Phase 8~10 任务记录已合并归档，阶段项目文档转为历史追溯。
