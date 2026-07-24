# GitHub Pages Hero 动态背景层（四期增量）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-hero-motion-layer.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-hero-motion-layer.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 主入口统一指向 `doc/site/prd.md`，本文仅维护增量。

- 对应标准执行入口: `doc/site/github-pages/github-pages-hero-motion-layer.project.md`

## 目标
- 在当前四期精简版基础上，为首屏增加轻量动态背景层，提升科技感辨识度。
- 动效应服务叙事而非喧宾夺主：不降低文案可读性，不影响核心 CTA。
- 保持纯静态实现（HTML/CSS/JS），不引入第三方前端框架。

## 范围
- 范围内
  - 在 Hero 区域新增 Canvas 动态背景层（粒子节点 + 连线 + 轻量扫描节奏）。
  - 动效与现有布局兼容，确保按钮、文本、导航交互不受影响。
  - 支持无障碍降级：`prefers-reduced-motion` 下自动停用动态渲染。
  - 中英文页面同构接入（同 DOM 标记、同样式策略）。
- 范围外
  - 全站全屏动态背景与复杂 WebGL 方案。
  - 引入动画库、构建工具或重型依赖。

## 接口/数据
- 页面结构
  - `site/index.html`
  - `site/en/index.html`
  - 新增 Hero Canvas 节点：`<canvas data-hero-canvas>`。
- 样式
  - `site/assets/styles.css`
  - 新增动态背景层定位、透明度、层级控制样式。
- 脚本
  - `site/assets/app.js`
  - 新增 `bindHeroCanvas()`：负责初始化、尺寸同步、动画循环和降级策略。

## 里程碑
- M1：文档与任务拆解
  - 输出增量设计文档与项目管理文档。
- M2：功能接入
  - 完成双语 Hero Canvas 接入与 JS 动画实现。
- M3：可访问性与性能保护
  - 接入 reduced-motion 降级、可见性暂停、移动端节点数量控制。
- M4：验证与收口
  - 完成静态结构自检、`cargo check`、文档状态与 devlog 更新。

## 风险
- 风险：动画过强影响可读性
  - 缓解：降低透明度与亮度，文本区域优先层级更高。
- 风险：移动端性能抖动
  - 缓解：按视口面积控制粒子数量，限制刷新频率。
- 风险：浏览器兼容性差异
  - 缓解：Canvas 初始化失败时静默降级，不影响页面主功能。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
