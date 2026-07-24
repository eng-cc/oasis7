# GitHub Pages 对标优化（三期）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-benchmark-polish-v3.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md`

审计轮次: 5
- 对应标准执行入口: `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md`

## ROUND-002 主从口径
- 主入口文档：`doc/site/prd.md`。
- 本文件仅维护对标优化（三期）增量内容。

## 目标
- 基于近年科技项目站点对标（Vercel / Supabase / Linear / Replicate）结果，继续提升 `oasis7` 对外展示页的“首屏记忆点 + 叙事节奏 + 证据可信度”。
- 将当前页面从“信息完整”升级为“更像产品官网”：先给价值与体验路径，再给架构与证据。
- 在不引入构建工具的前提下（仍为纯静态 HTML/CSS/JS），完成中英文双语同步改造。

## 范围
- **范围内**
  - Hero 改造：强化主叙事与双 CTA（体验 / 文档），加入更具识别度的世界观视觉。
  - 叙事结构增强：新增“三段式理解路径”模块（30 秒 / 3 分钟 / 30 分钟）。
  - 证据链增强：新增“真实运行证据”模块（截图、事件片段、近期更新）。
  - 证据交互增强：在 `#proof` 模块增加“场景事件时间线切换器”（按场景查看关键事件序列）。
  - 视觉层级优化：降低全局边框/发光强度，突出重点卡片与关键按钮。
  - 中英文同构更新：`site/index.html` 与 `site/en/index.html` 模块结构和交互一致。
- **范围外**
  - 接入后端 API 或在线实时沙盒服务。
  - 引入 React/Vite/SSR 等前端构建链路。
  - 大规模素材生产（视频/GIF）与专题子站拆分。

## 接口 / 数据

### 涉及文件
- 页面结构：
  - `site/index.html`
  - `site/en/index.html`
- 样式与交互：
  - `site/assets/styles.css`
  - `site/assets/app.js`
- 新增视觉素材：
  - `site/assets/images/world-constellation.svg`
- 文档与管理：
  - `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md`
  - `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md`

### 结构约定
- 新增导航锚点：`#path`、`#proof`。
- 新增叙事步骤数据标记：`data-story-step`（供滚动高亮使用）。
- 维持既有交互兼容：`data-reveal`、`data-timeline-*`、`data-counter-target` 不破坏。

## 里程碑
- **M1：文档与任务拆解**
  - 输出三期设计文档与项目管理文档。
- **M2：内容结构落地**
  - 完成 Hero、叙事路径、证据链模块的中英文页面改造。
- **M3：视觉系统精修**
  - 完成边框/发光层级收敛与新模块样式适配。
- **M4：验证与收口**
  - 完成截图回归与 `cargo check`，更新项目文档和当日 devlog。
- **M5：证据交互增强（增量）**
  - 在不引入额外依赖的前提下，完成 `#proof` 场景切换交互与键盘可访问性。
  - 完成中英文页面对齐与截图回归。

## 风险
- **风险：模块增加后页面信息再次过重**
  - 缓解：每个新增模块限制在 3 个核心点，避免长段落。
- **风险：视觉减重后“科技感”变弱**
  - 缓解：将强调聚焦到 Hero 与关键 CTA，而不是所有卡片统一高亮。
- **风险：双语页面结构漂移**
  - 缓解：按同锚点、同顺序改造并截图对照验证。
- **风险：静态资源增长影响加载**
  - 缓解：新增素材采用 SVG，复用现有截图，不引入重资源。
- **风险：交互复杂度提升影响可维护性**
  - 缓解：采用声明式 `data-*` 结构与单函数绑定，避免引入重型状态管理。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
