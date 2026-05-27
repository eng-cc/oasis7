# GitHub Pages 首页激进改造（2026-02-26）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md`
审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确改动范围为首页 CN/EN 与共享样式

### 1. 首页激进改造实现（CN/EN + 样式）
- [x] 重写 `site/assets/styles.css`（视觉系统、版式、断点、动效节奏）
- [x] 重写 `site/index.html`（游戏优先叙事 + 引擎后置）
- [x] 重写 `site/en/index.html`（与中文同构）
- [x] 保持 `site/assets/app.js` 所需锚点与 `data-*` 兼容

### 2. 验证与收口
- [x] 执行 `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer`
- [x] 回写本项目管理文档状态
- [x] 写任务日志（`doc/devlog/README.md`）

## 依赖
- 沿用现有静态站点结构与 `site/assets/app.js` 交互逻辑。
- 页面内容基线来自 README、game docs、viewer manual。

## 状态
- 当前阶段：已完成（任务 0/1/2 全部完成）
- 最近更新：完成验证收口并结项（2026-02-26）
- 下一步：无。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
