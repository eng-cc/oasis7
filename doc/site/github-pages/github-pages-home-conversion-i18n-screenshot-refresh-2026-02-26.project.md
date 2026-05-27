# GitHub Pages 首页转化文案与中英一致性收敛 + UI 截图刷新（2026-02-26）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md`
审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护增量任务。

## 任务拆解

### 0. 建档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确改动范围为首页文案/一致性与三张场景截图

### 1. 首屏转化文案收敛（CN/EN）
- [x] 调整 `site/index.html` 首屏与上手区文案，避免超预期承诺
- [x] 调整 `site/en/index.html` 对应文案并与中文语义对齐
- [x] 保持锚点与交互标记兼容

### 2. 中英文一致性校准
- [x] 清理中文页关键区域混杂英文术语问题（保留必要专有名词）
- [x] 校验 CN/EN section 同构与导航语义一致

### 3. 游戏 UI 截图刷新
- [x] 按 S6 Web 闭环尝试采集 `minimal` 截图（记录 Web 端 `wgpu` pipeline panic）
- [x] 按 S6 Web 闭环尝试采集 `twin_region_bootstrap` 截图（同上）
- [x] 按 S6 Web 闭环尝试采集 `triad_region_bootstrap` 截图（同上）
- [x] 按约定 fallback 到 native 抓图链路，重新采集三场景 PNG 证据
- [x] 替换 `site/assets/images/screenshots/*.webp`

### 4. 验证与收口
- [x] 执行 `env -u RUSTC_WRAPPER cargo check -p oasis7_viewer`
- [x] 回写本项目管理文档状态
- [x] 写任务日志（`doc/devlog/README.md`）

## 依赖
- 站点结构：`site/` 静态目录与 `site/assets/app.js` 现有交互。
- 测试流程：`testing-manual.md` S6 Web 闭环。

## 状态
- 当前阶段：已完成（任务 0/1/2/3/4 全部完成）
- 最近更新：完成任务 4（验证与收口）（2026-02-26）
- 下一步：无。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
