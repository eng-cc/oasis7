# GitHub Pages 质量门禁 + 文档镜像同步 + SEO 元信息加固（2026-02-26）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护本专题增量任务，不重复主项目文档任务编排。

## 任务拆解

### T0 建档与基线
- [x] 新建设计文档：`doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md`
- [x] 新建项目管理文档：`doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md`
- [x] 明确实施范围：Pages 门禁、手册镜像同步、SEO 元信息

### T1 手册镜像路径修复与一致性检查
- [x] 修复 `site/doc/cn/viewer-manual.html` 过时 agent-browser 路径
- [x] 修复 `site/doc/en/viewer-manual.html` 过时 agent-browser 路径
- [x] 新增 `scripts/site-manual-sync-check.sh`（关键口径一致性校验）
- [x] 任务测试与提交

### T2 Pages 发布门禁接入
- [x] 新增 `scripts/site-link-check.sh`（站内相对链接校验）
- [x] 更新 `.github/workflows/pages.yml`，部署前执行校验脚本
- [x] 任务测试与提交

### T3 首页 SEO 元信息加固
- [x] 更新 `site/index.html` SEO/社交元信息
- [x] 更新 `site/en/index.html` SEO/社交元信息
- [x] 校验 canonical/hreflang 与 OG/Twitter 字段一致性
- [x] 任务测试与提交

### T4 回归验证与文档收口
- [x] 执行站点脚本回归校验与页面冒烟
- [x] 回写本项目管理文档状态
- [x] 写任务日志：`doc/devlog/2026-02-26.md`
- [x] 任务测试与提交

## 依赖
- 站点静态结构：`site/` 与 `site/assets/app.js` 既有交互契约。
- 发布流程：`.github/workflows/pages.yml`。
- 手册内容基线：`doc/world-simulator/viewer/viewer-manual.manual.md`。

## 状态
- 当前阶段：已完成（T0~T4 全部完成）
- 最近更新：完成 T4（回归验证与文档收口）（2026-02-26）
- 下一步：无（本项目已结项）。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
