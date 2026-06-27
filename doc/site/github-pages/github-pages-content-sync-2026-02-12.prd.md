# GitHub Pages 内容状态同步（2026-02-12）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-content-sync-2026-02-12.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 主入口文档：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`。
- 本文件仅维护内容状态同步增量内容。

- 对应标准执行入口: `doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md`

## 目标
- 让展示站文案与当前代码/项目阶段保持一致，避免“路线图与实际进度不一致”。
- 同步中英文页面关键状态信息，确保对外表达统一。
- 在不改动页面结构和交互逻辑的前提下，完成文本层快速修正。

## 范围
- 范围内
  - 更新中英文首页路线图状态（M3 完成，M4 进行中，M5 下一阶段）。
  - 更新首页关键指标文案（明确“3 种运行模式”的含义）。
  - 补充证据区“近期更新”文案，反映四期增量（Hero 动效、指针联动、架构图精修）。
  - 更新 README 站点亮点描述，覆盖截至 2026-02-12 的阶段。
- 范围外
  - 页面布局、样式和交互重构。
  - 新增素材或新增页面。

## 接口/数据
- 页面文件
  - `site/index.html`
  - `site/en/index.html`
- 文档文件
  - `README.md`
- 一致性依据
  - `doc/world-simulator/project.md` 当前阶段与下一步状态。

## 里程碑
- M1：文档与任务拆解
  - 输出本次同步设计文档与项目管理文档。
- M2：内容修正
  - 完成中英文首页与 README 的状态同步修改。
- M3：验证收口
  - 校验关键文案行与链接，执行 `cargo check`，更新项目文档与 devlog。

## 风险
- 风险：只改中文或只改英文导致双语偏移
  - 缓解：同一任务内成对修改 `site/index.html` 与 `site/en/index.html`。
- 风险：里程碑措辞过细导致后续频繁改动
  - 缓解：采用“阶段级”表达（M3/M4/M5），减少细枝末节承诺。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
