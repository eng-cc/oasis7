# GitHub Pages Hero 指针微交互（四期增量）设计文档

- 对应设计文档: `doc/site/github-pages/github-pages-hero-pointer-interaction.design.md`
- 对应项目管理文档: `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md`
审计轮次: 5

- 对应标准执行入口: `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md`

## ROUND-002 主从口径
- 主入口：`doc/site/prd.md`
- 本文仅维护增量。

## 目标
- 在现有 Hero 动态背景层上增加鼠标/触摸微交互联动，增强“可感知科技感”。
- 交互反馈保持克制：只对局部节点和光束产生轻微响应，不影响文本可读与操作。
- 继续保持纯静态方案，不引入第三方动画依赖。

## 范围
- 范围内
  - Hero Canvas 接入指针位置感知（鼠标 + 触摸）。
  - 指针附近节点响应：轻微趋近/扰动与高亮增强。
  - 扫描光带速度/亮度在交互时短时提升。
  - 支持降级：`prefers-reduced-motion` 或无指针环境下退回基础动画。
- 范围外
  - 粒子系统重写或复杂物理模拟。
  - 引入 WebGL / Three.js 等重型渲染方案。

## 接口/数据
- 脚本
  - `site/assets/app.js`
  - 扩展 `bindHeroCanvas()` 指针状态管理与交互渲染逻辑。
- 样式
  - `site/assets/styles.css`
  - 仅按需补充轻量视觉变量（如交互态透明度阈值）。
- 页面
  - 复用既有 `data-hero-canvas`，不新增 DOM 结构。

## 里程碑
- M1：文档与任务拆解
  - 输出增量设计文档与项目管理文档。
- M2：交互实现
  - 完成指针状态采集、节点响应和扫描光带联动。
- M3：降级与验证
  - 验证 reduced-motion、触摸设备和页面隐藏恢复逻辑。
- M4：收口
  - `cargo check`、项目管理文档更新、devlog 记录。

## 风险
- 风险：交互过强造成视觉干扰
  - 缓解：限制响应半径和透明度上限，采用短时衰减。
- 风险：移动端触摸行为与滚动冲突
  - 缓解：仅被动读取触点坐标，不阻断默认触摸事件。
- 风险：额外计算影响帧率
  - 缓解：复用现有 32ms 节流，交互计算只作用于邻近节点。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
