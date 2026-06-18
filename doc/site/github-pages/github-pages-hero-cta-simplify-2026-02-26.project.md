# GitHub Pages 首屏 CTA 收敛与文案校准（2026-02-26）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.prd.md`

审计轮次: 5

## 治理状态（2026-06-18）
- 状态：历史压缩专题项目记录，任务 0/1 已完成且无下一步。
- 当前入口：`doc/site/project.md` 汇总当前站点任务状态；本文件仅保留该 CTA 微专题的完成证据。
- 边界：后续首页 CTA、公开叙事或下载入口变化，应在新的 task/project 记录中增量维护，不继续扩写本文。

## 审计备注
- 主项目入口统一指向 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`，本文仅维护增量任务。

## 任务拆解

### 0. 建档与范围确认
- [x] 新增设计文档（`doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确改动范围仅为首页 Hero CTA（CN/EN）

### 1. Hero CTA 收敛与文案改写
- [x] `site/index.html`：首屏 CTA 收敛为 2 个（主：试玩/演示；次：文档）
- [x] `site/en/index.html`：首屏 CTA 收敛为 2 个（同构）
- [x] 重写主按钮文案，去除“30 秒进入首局”承诺性措辞
- [x] 执行 `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer`
- [x] 回写项目管理文档状态与任务日志

## 依赖
- 复用现有 `site/assets/styles.css` 的 `.cta-row` 与 `.button` 样式能力，不新增样式依赖。
- 保持 `site/assets/app.js` 现有行为，不引入新脚本。

## 状态
- 当前阶段：已完成（任务 0/1 全部完成）
- 最近更新：完成 CTA 收敛与文案改写并结项（2026-02-26）
- 下一步：无。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
