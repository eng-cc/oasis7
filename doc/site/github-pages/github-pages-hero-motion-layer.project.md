# GitHub Pages Hero 动态背景层（四期增量）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-hero-motion-layer.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-hero-motion-layer.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口统一指向 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`，本文仅维护增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增增量设计文档（`doc/site/github-pages/github-pages-hero-motion-layer.prd.md`）
- [x] 新增增量项目管理文档（本文件）
- [x] 明确验收标准（视觉增强/可访问性/性能保护）

### 1. Hero 动态层实现（双语同构）
- [x] 在中英文 Hero 区域接入 `data-hero-canvas`
- [x] 在 `app.js` 实现轻量 Canvas 动画循环（粒子/连线/扫描）
- [x] 控制移动端与小屏节点数量，避免过载

### 2. 样式与可访问性
- [x] 增加 Hero Canvas 层级样式，确保文本与按钮可读性
- [x] 接入 `prefers-reduced-motion` 自动降级
- [x] 页面不可见时暂停动画，恢复时继续

### 3. 验证与收口
- [x] 静态结构自检（双语 canvas 标记与脚本入口）
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新项目管理文档状态
- [x] 写入当日开发日志（`doc/devlog/README.md`）

## 依赖
- 沿用现有 `site/` 静态结构与 `assets/app.js` 交互入口。
- 不新增第三方依赖与构建链路。

## 状态
- 当前阶段：已完成
- 最近更新：完成双语 Hero Canvas 动态层接入、降级策略与 `cargo check` 校验（2026-02-12）
- 下一步：可继续增强“交互响应式光束”或“场景状态联动”动效策略。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
