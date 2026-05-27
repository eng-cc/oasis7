# GitHub Pages 用户视角问题修复（2026-02-26）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护本专题增量任务，不重复主项目文档任务编排。

## 任务拆解

### 0. 建档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确本轮目标：sticky、语言重定向时机、字体加载、移动端可访问性

### 1. 导航与移动端可访问性修复
- [x] 修复顶部导航 sticky 失效问题
- [x] 扩大移动端菜单/语言按钮点击热区
- [x] 为菜单展开状态补齐 `aria-expanded` 同步与关联语义

### 2. 语言与性能修复
- [x] 调整语言自动跳转完成标记写入时机（docs 路径不提前写入）
- [x] 优化字体加载，减少英文首页不必要的外部字体请求

### 3. 验证与收口
- [x] agent-browser 关键路径回归（首页、语言切换、文档入口、移动端）
- [x] 执行 `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer`
- [x] 回写本项目管理文档状态
- [x] 写任务日志（`doc/devlog/README.md`）

## 依赖
- 站点静态结构：`site/` 与 `.github/workflows/pages.yml`。
- 现有交互脚本：`site/assets/app.js`。
- 回归工具链：agent-browser CLI 与本地静态服务器。

## 状态
- 当前阶段：已完成（任务 0/1/2/3 全部完成）
- 最近更新：完成任务 3（验证与收口）（2026-02-26）
- 下一步：无

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
