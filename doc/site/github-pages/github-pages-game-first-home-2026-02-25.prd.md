# GitHub Pages 首页“游戏优先”分层重构（2026-02-25）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-game-first-home-2026-02-25.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 主入口统一指向 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`，本文仅维护增量。

- 对应标准执行入口: `doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md`

## 目标
- 将首页叙事重构为“游戏优先，引擎后置”：前半屏主要讲玩家体验与玩法闭环，底部几屏再讲引擎架构与工程扩展。
- 保留现有视觉优点与交互资产：深色科技风、Hero 动效、cards 栅格、proof switcher、scroll reveal。
- 保持中英文页面同构，确保锚点、导航与语言切换一致。

## 范围
- 范围内
  - 重构首页信息层级：`site/index.html`、`site/en/index.html`。
  - 调整 section 顺序与文案重心（游戏内容上移，引擎内容后置）。
  - 保持 `site/assets/app.js` 依赖的 `data-*` 标记兼容。
- 范围外
  - 不改 `site/doc/cn/index.html`、`site/doc/en/index.html` 内容。
  - 不新增前端框架或构建系统。
  - 不改 viewer-manual 正文章节。

## 接口/数据
- 内容基线
  - `README.md`
  - `doc/game/gameplay/gameplay-top-level-design.prd.md`
  - `doc/game/gameplay/gameplay-engineering-architecture.md`
  - `doc/world-simulator/viewer/viewer-manual.manual.md`
- 目标页面
  - `site/index.html`
  - `site/en/index.html`

## 里程碑
- M0：建档与任务拆解。
- M1：首页 CN/EN 重排为“游戏优先”信息结构。
- M2：验证、项目文档回写与收口。

## 风险
- 风险：重排后破坏既有交互锚点。
  - 缓解：保留现有 `id` 与 `data-*` 标记，仅调整 section 顺序和文案。
- 风险：游戏与引擎边界不清。
  - 缓解：明确“上半页玩法价值、下半页引擎能力”两段式信息架构。
- 风险：中英文页面漂移。
  - 缓解：同任务同时更新 CN/EN，对齐模块顺序和锚点。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
