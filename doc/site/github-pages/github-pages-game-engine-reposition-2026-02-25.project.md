# GitHub Pages 游戏+引擎定位重写（2026-02-25）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`

审计轮次: 5

## ROUND-002 主从口径
- 本文件为 github-pages 项目主入口（master）。
- `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md`、`doc/site/github-pages/github-pages-benchmark-polish-v3.project.md`、`doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md` 为本批增量计划文档（slave）。

## 任务拆解

### 0. 文档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确内容输入基线（README、game docs、viewer manual）

### 1. 首页重写（CN/EN）
- [x] 重写 `site/index.html` 游戏+引擎叙事（保留视觉结构与交互）
- [x] 重写 `site/en/index.html` 同构英文页面
- [x] 更新首页外链入口到当前叙事文档

### 2. 文档中心重写（CN/EN）
- [x] 更新 `site/doc/cn/index.html`，新增游戏与引擎文档入口
- [x] 更新 `site/doc/en/index.html`，保持与中文同构
- [x] 文档目录状态文案改为“定位重写已完成，手册分层维护”

### 3. 验证与收口
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新本项目管理文档状态
- [x] 写任务日志（`doc/devlog/README.md`）

## 依赖
- 沿用 `site/` 静态目录与 GitHub Pages 发布流程。
- 交互兼容 `site/assets/app.js` 现有 `data-*` 标记。

## 状态
- 当前阶段：已完成（任务 0/1/2/3 全部完成）
- 最近更新：完成任务 3（全量校验与状态收口，2026-02-25）
- 下一步：无。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
