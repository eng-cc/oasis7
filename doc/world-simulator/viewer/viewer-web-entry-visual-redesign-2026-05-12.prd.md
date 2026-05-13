# Viewer Web 主入口视觉重构（2026-05-12）

- 对应项目管理文档: `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.project.md`
- 关联主专题: `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md`

审计轮次: 1

## 1. Executive Summary
- 将当前 Viewer Web 主入口从“泛用深色诊断仪表板”重构为“世界优先的正式游戏入口”。
- 保留既有 `software_safe` / `viewer` Web 单入口能力边界，不改协议、不删主路径能力，只重排结构、视觉层级与默认暴露面。
- 让玩家首屏优先感知世界、目标和下一步动作，把诊断/治理/高级控制显式降级为按需展开。

## 目标
- 让 Viewer Web 主入口第一视觉焦点回到世界画布和当前任务，而不是左右等权的诊断信息面板。
- 在不改动 runtime / auth / gameplay contract 的前提下，重做主入口的视觉语言和默认信息层级。
- 保留 QA / producer 所需的诊断能力，但把它们收进次级面和按需展开路径。

## 2. User Experience & Functionality
- 范围内：
  - `crates/oasis7_viewer/software_safe.html`
  - `crates/oasis7_viewer/software_safe_src/main.jsx`
  - `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
  - 与上述入口直接相关的 repo-owned Web 回归与截图证据
- 范围外：
  - runtime 协议、世界规则、auth contract、hosted action policy 语义变更
  - 3D 视觉恢复、贴图资产链路、美术资源包引入
  - launcher 原生端或 explorer 专题 UI 改版

## 范围
- 范围内：
  - Viewer Web 单入口的布局重排、视觉 token、舞台/侧栏层级、移动端切换路径。
  - Player 首屏默认暴露面的精简与 diagnostics 面的降级。
- 范围外：
  - 协议字段、`__AW_TEST__` 语义、world snapshot schema 变更。
  - 原生 launcher / explorer 主题视觉改版。
  - 3D workstream 恢复或新的美术资源管线。

## 3. User Stories
- As a 玩家 / 制作人, I want the Viewer Web entry to feel like a playable world surface instead of an internal diagnostics console, so that I can understand the current world state and next action within one glance.
- As a `viewer_engineer`, I want diagnostics and advanced controls to remain available but visually demoted, so that Web-first regression and player readability can coexist without splitting the entry contract.

## 4. Technical Specifications

### 4.1 Visual Direction
- 视觉方向固定为“工业世界指挥桌”：
  - 世界画布是首屏主舞台。
  - 侧边信息采用分层工具面而非平均权重的三栏 SaaS 卡片墙。
  - 字体、配色、边界、按钮和空状态统一到一套非通用仪表板语汇。

### 4.2 Information Architecture
- Player 默认视图必须只强调三类信息：
  - 当前世界 / 选中目标
  - 当前目标 / blocker / 下一步动作
  - 最近一次反馈或可执行主动作
- 诊断信息必须显式降级：
  - `Runtime Diagnostics`
  - `Session Ladder`
  - `Hosted Action Matrix`
  - 原始 JSON / provider check / governance lane 说明

### 4.3 Layout Constraints
- 首屏默认布局改为“主舞台 + 辅助抽屉/侧栏”的世界优先结构，不再让左/中/右三栏保持同等视觉权重。
- 中央世界区必须拥有明显大于辅助信息区的可视占比。
- 移动端不能只做单列堆叠；至少要形成 `World / Targets / Command` 的清晰切换路径。

### 4.4 Capability Preservation
- 必须保持以下能力继续可用并可脚本化验证：
  - 连接状态
  - 世界摘要
  - 目标选择
  - gameplay summary / blocker / handoff surface
  - Agent chat
  - prompt override 展开链路
  - `__AW_TEST__` 合同
- 嵌入式世界舞台必须以 wasm bridge 作为唯一 renderer runtime；若 bridge 缺失或启动失败，页面应显式退回 host fallback/callout，而不是静默切到第二套 JS renderer。
- repo-owned 回归至少分两层：
  - 结构 / DOM 层：`Vitest + @solidjs/testing-library` 断言 `World / Targets / Command` 锚点、`Runtime Diagnostics` / `Session Ladder` / `Hosted Action Matrix` 降级面，以及 `Agent Chat` / `Prompt Overrides` 的可达性。
  - 渲染 runtime 层：repo-owned 测试必须覆盖“wasm bridge 成功加载”和“wasm bridge 缺失时显式 fallback surface”两条分支。
  - headed browser 层：保留现有 `agent-browser` / release strict 脚本，验证真实页面加载、选择、实时推进、prompt/chat flow 与截图证据。

## 接口 / 数据
- 静态入口：
  - `viewer.html` / `software_safe.html`
  - `viewer.js` / `software_safe.js`
- 源码入口：
  - `crates/oasis7_viewer/software_safe.html`
  - `crates/oasis7_viewer/software_safe_src/main.jsx`
  - `crates/oasis7_viewer/software_safe_src/pixel_world_host.jsx`
- 依赖的运行态数据：
  - canonical gameplay summary
  - snapshot model / selection / recent events
  - auth / hosted access truth
  - prompt/chat feedback
- 本专题不新增协议字段，只重排现有数据在 UI 中的展示方式。

## 5. Risks & Roadmap
- M1：专题 PRD / Project 建模，冻结视觉方向、结构边界与能力保留面。
- M2：完成首屏结构重排，确认世界主舞台、侧栏降权和移动端路径。
- M3：完成视觉语言刷新，包括字体、配色、按钮、卡片、状态表达与 overlay。
- M4：完成 Web 回归、截图采证与文档收口。

## 里程碑
- M1：专题建模完成。
- M2：结构重排完成。
- M3：视觉语言刷新完成。
- M4：回归和文档收口完成。

### Technical Risks
- 视觉重构若直接删减信息，可能破坏 QA / producer 观察路径。
  - 对策：降级而不是删除，把高级信息统一收进 Director/diagnostics surface。
- 主入口布局变化可能破坏现有浏览器自动化选择器或可见性假设。
  - 对策：保留关键 `data-*` 钩子和 `__AW_TEST__` 契约，必要时只补充新锚点，不随意改名。
- 移动端世界优先布局若处理不当，可能造成目标选择或命令路径过深。
  - 对策：把移动端切换模型纳入同一专题，而不是后补单列媒体查询。

## 风险
- 玩家态降噪后，Producer / QA 可能短期不适应新的 diagnostics 入口位置。
- 结构重排可能影响现有浏览器自动化的元素可见性假设。
- 视觉改版如果只换颜色不换层级，仍会保留原有“内部控制台”观感。

## 6. Acceptance Criteria
- AC-1: Viewer Web 首屏第一视觉焦点是世界画布，而不是平均权重的三栏信息面板。
- AC-2: Player 主路径首屏只暴露世界、目标、下一步动作和关键反馈；诊断/治理信息被显式降级。
- AC-3: `Agent Chat`、Prompt Overrides、关键 summary/blocker surface 仍保持可达且可测试。
- AC-4: 视觉系统明显区别于当前通用深色 dashboard 风格，并在桌面/移动端都保持一致的层级逻辑。

## 7. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-WORLD_SIMULATOR-046 | `task_367fa36f5a514e9ea3bc11da95cd8d5d` | `test_tier_required` | `npm run build:software-safe` + repo-owned Web regression + `agent-browser` 截图采证 + `./scripts/doc-governance-check.sh` + `git diff --check` | Viewer Web 单入口结构、视觉层级、移动端主路径与现有测试契约 |
| PRD-WORLD_SIMULATOR-046 | `task_3432ce6ab4fc47fb84811bcfef2c22c8` | `test_tier_required` | `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs` + `npm --prefix crates/oasis7_viewer run test:ui` + `npm --prefix crates/oasis7_viewer run build:software-safe` | Viewer Web 单入口结构锚点、Prompt/Chat surface、diagnostics 降级面与 repo-owned DOM 回归 |
| PRD-WORLD_SIMULATOR-046 | `task_15efbff5922a421e976430906e54c01f` | `test_tier_required` | `npm --prefix crates/oasis7_viewer run test:ui` + `npm --prefix crates/oasis7_viewer run build:software-safe` + `./scripts/doc-governance-check.sh` + `git diff --check` | Pixel-world wasm runtime contract、explicit fallback surface 与 Viewer Web 渲染链真值 |
