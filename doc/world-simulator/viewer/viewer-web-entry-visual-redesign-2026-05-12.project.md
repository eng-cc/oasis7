# viewer-web-entry-visual-redesign-2026-05-12 项目管理

- 对应需求文档: `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] viewer-web-entry-structure-reset (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 将当前三栏等权重控制台重排为“世界主舞台 + 辅助抽屉/侧栏”结构，明确 Player 主路径只保留世界、目标、下一步动作和关键反馈。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml
- [x] viewer-web-entry-visual-language-refresh (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 基于“工业世界指挥桌”方向重构 Web 入口的字体、配色、边界、按钮、空状态和 overlay 表达，消除当前通用深色 dashboard 观感。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml
- [x] viewer-web-entry-regression-rebaseline (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 回跑 Web 构建、repo-owned regression、agent-browser 截图与文档治理，确认结构重排后主入口能力、脚本选择器与截图证据仍然稳定。 Trace: .pm/tasks/task_367fa36f5a514e9ea3bc11da95cd8d5d.yaml
- [x] viewer-web-ui-automation-baseline (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 为 `software_safe_src` 主入口补 repo-owned Solid UI 回归，覆盖 `World / Targets / Command` 锚点、`Runtime Diagnostics` 降级面，以及 `Agent Chat` / `Prompt Overrides` 的 DOM 可达性，并把脚本接入现有 required gate。 Trace: .pm/tasks/task_3432ce6ab4fc47fb84811bcfef2c22c8.yaml
- [x] viewer-web-pixel-world-wasm-only (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 去掉 pixel-world 嵌入舞台的 JS renderer fallback，固定以 wasm bridge 为唯一 runtime，并补齐“成功加载 / 显式失败” repo-owned 测试。 Trace: .pm/tasks/task_15efbff5922a421e976430906e54c01f.yaml
- [x] viewer-web-pixel-world-content-density (PRD-WORLD_SIMULATOR-046) [test_tier_required]: 为像素世界主舞台补地点尺度提示、Agent-地点关系线与 gameplay/event 热点层，确保稀疏快照下首屏仍能读出世界关系。 Trace: .pm/tasks/task_f5838a7590534c93a50d3433d1103851.yaml

## 依赖
- `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `crates/oasis7_viewer/software_safe.html`
- `crates/oasis7_viewer/software_safe_src/main.jsx`
- `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- `testing-manual.md`

## 状态
- 更新日期: 2026-05-13
- 当前状态: done
- 下一任务: 若重启下一轮 Web 入口结构/视觉专题，先按 `PRD-ENGINEERING-031` 的 optional visual companion 边界产出 IA/wireframe/layout compare，再创建新的实现 task。
- 最新完成: `viewer-web-pixel-world-wasm-only`
- 最新完成: `viewer-web-pixel-world-content-density`

## 实施顺序
1. 先完成结构重排，确保世界画布、目标列表和命令入口的主次关系成立。
2. 再完成视觉语言刷新，统一字体、配色和状态表达。
3. 最后补回归截图与文档收口，避免“设计更好了但脚本断了”。
4. 追加 repo-owned Solid UI 回归，先锁定 DOM 锚点与 surface 可达性，再保留 headed browser 证据作为上层发布验证。
5. 固定 pixel-world 渲染链只认 wasm bridge，把 runtime 缺失改成显式 fallback/callout，并用 repo-owned 测试锁定该契约。
6. 在不新增协议字段的前提下，利用现有 snapshot/gameplay 数据给像素世界补关系线、热点与地点尺度提示，收口“画面只剩几个孤点”的可读性缺口。
