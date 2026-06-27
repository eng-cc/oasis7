# GitHub Pages 首页激进改造（2026-02-26）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md`
审计轮次: 5

- 对应标准执行入口: `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md`

## ROUND-002 主从口径
- 主入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`
- 本文仅维护增量。

## 目标
- 对 `site/` 首页执行“大刀阔斧”改造，在不破坏现有交互闭环的前提下，显著提升视觉记忆点与信息冲击力。
- 保持“游戏优先、引擎后置”的产品叙事：上半页强调可玩性与玩家价值，下半页再进入引擎架构。
- 中英文首页同构维护，保证导航锚点、语言切换、proof switcher、reveal/counter 动效持续可用。

## 范围
- 范围内
  - 重写共享视觉系统：`site/assets/styles.css`。
  - 重写首页结构与文案：`site/index.html`、`site/en/index.html`。
  - 保持 `site/assets/app.js` 依赖的 `id` 与 `data-*` 标记兼容。
- 范围外
  - 不改文档中心页面 `site/doc/cn/index.html`、`site/doc/en/index.html`。
  - 不改 Rust 业务逻辑与 viewer 功能代码。
  - 不引入新的前端构建链路。

## 接口/数据
- 交互兼容接口
  - section 锚点：`#matrix`、`#scenarios`、`#demo`、`#proof`、`#roadmap`、`#architecture`、`#contribute`
  - 交互标记：`data-reveal`、`data-counter-target`、`data-menu*`、`data-lang*`、`data-proof-*`
- 内容基线
  - `README.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
  - `doc/world-simulator/viewer/viewer-manual.manual.md`

## 里程碑
- M0：建档与任务拆解。
- M1：首页视觉系统激进升级（版式、字体、色彩、层次、动效节奏）。
- M2：CN/EN 首页内容重写并同构对齐。
- M3：校验、项目文档回写、任务日志收口。

## 风险
- 风险：激进视觉改动造成信息密度过高，移动端可读性下降。
  - 缓解：移动断点单独控制字号、按钮数量、栅格折叠与段间距。
- 风险：结构改动破坏现有 JS 交互。
  - 缓解：保留既有锚点与 `data-*`，只重构视觉层与文案层。
- 风险：中英文页出现结构漂移。
  - 缓解：同一任务内同步修改 CN/EN，逐段对齐 section 顺序与语义。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
