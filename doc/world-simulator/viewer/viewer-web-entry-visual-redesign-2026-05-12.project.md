# viewer-web-entry-visual-redesign-2026-05-12 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] viewer-web-entry-structure-reset (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将当前三栏等权重控制台重排为“世界主舞台 + 辅助抽屉/侧栏”结构，明确 Player 主路径只保留世界、目标、下一步动作和关键反馈。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml
- [x] viewer-web-entry-visual-language-refresh (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 基于“工业世界指挥桌”方向重构 Web 入口的字体、配色、边界、按钮、空状态和 overlay 表达，消除当前通用深色 dashboard 观感。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml
- [x] viewer-web-entry-regression-rebaseline (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑 Web 构建、repo-owned regression、agent-browser 截图与文档治理，确认结构重排后主入口能力、脚本选择器与截图证据仍然稳定。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml

## 依赖
- `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `crates/oasis7_viewer/software_safe.html`
- `crates/oasis7_viewer/software_safe_src/main.jsx`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `testing-manual.md`

## 状态
- 更新日期: 2026-05-12
- 当前状态: ready_for_closeout
- 下一任务: none
- 最新完成: `viewer-web-entry-regression-rebaseline`

## 实施顺序
1. 先完成结构重排，确保世界画布、目标列表和命令入口的主次关系成立。
2. 再完成视觉语言刷新，统一字体、配色和状态表达。
3. 最后补回归截图与文档收口，避免“设计更好了但脚本断了”。
