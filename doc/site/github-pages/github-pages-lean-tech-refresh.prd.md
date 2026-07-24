# GitHub Pages 精简科技感升级（四期）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-lean-tech-refresh.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-lean-tech-refresh.project.md`
审计轮次: 5

- 对应标准执行入口: `doc/site/github-pages/github-pages-lean-tech-refresh.project.md`

## ROUND-002 主从口径
- 主入口：`doc/site/prd.md`
- 本文仅维护增量。

## 目标
- 解决当前展示站“文字密度偏高、页面过长、科技感辨识度一般”的问题。
- 将首页压缩为“6 屏内可读”的叙事结构：先价值主张，再能力与证据，最后给出行动入口。
- 在保持纯静态部署（HTML/CSS/JS）的前提下，完成中英文双页同构升级。

## 范围
- 范围内
  - 重排首页信息架构：减少导航与 section 数量，保留关键叙事链路。
  - 精简文案：减少重复说明，突出一句话价值、核心指标、可执行路径。
  - 升级视觉系统：重设字体与色板、增强重点区层次、弱化非重点卡片噪声。
  - 保留并简化证据模块：场景切换、事件片段、可复现命令闭环。
  - 中英文同步改造：`site/index.html` 与 `site/en/index.html` 保持锚点结构一致。
- 范围外
  - 引入前端框架或构建工具。
  - 接入后端实时接口与在线沙盒。
  - 录制大体积视频/GIF 素材。

## 接口/数据
- 页面文件
  - `site/index.html`
  - `site/en/index.html`
- 样式与交互
  - `site/assets/styles.css`
  - `site/assets/app.js`（保持兼容，不新增复杂状态）
- 结构约定
  - 首页主锚点收敛为：`#matrix`、`#architecture`、`#scenarios`、`#demo`、`#proof`、`#roadmap`。
  - 保持既有交互标记兼容：`data-reveal`、`data-counter-target`、`data-proof-*`。

## 里程碑
- M1：文档与任务拆解
  - 输出四期设计文档与项目管理文档。
- M2：内容结构瘦身（双语）
  - Hero 与导航收敛，删除低收益冗余模块，保留关键路径。
- M3：视觉系统升级
  - 完成字体、色板、卡片层级与重点区视觉强化。
- M4：验证与收口
  - 完成静态结构自检与 `cargo check`，更新项目管理文档与 devlog。

## 风险
- 风险：信息过度压缩导致“内容不够完整”
  - 缓解：保留架构、证据、快速开始三块硬信息，详细说明通过外链文档承接。
- 风险：视觉改动过激影响可读性
  - 缓解：使用高对比文本与有限高亮，移动端优先验证。
- 风险：中英文同步偏移
  - 缓解：采用同锚点、同模块顺序、同交互结构进行并行改造。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
