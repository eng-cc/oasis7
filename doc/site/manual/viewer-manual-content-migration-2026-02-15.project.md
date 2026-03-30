# Viewer 使用手册内容搬迁（2026-02-15）项目管理文档

- 对应设计文档: `doc/site/manual/viewer-manual-content-migration-2026-02-15.design.md`
- 对应需求文档: `doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口文档：`doc/site/manual/site-manual-static-docs.project.md`。
- 本文件仅维护增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增设计文档（`doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确搬迁来源清单（viewer-* + capture 脚本文档）

### 1. 中文基线手册搬迁
- [x] 更新 `doc/world-simulator/viewer/viewer-manual.manual.md`（新增操作章节）
- [x] 清理冲突口径（保持 Web 默认 / native fallback）
- [x] 自检章节结构与命令可复制性

### 2. 站点手册同步
- [x] 更新 `site/doc/cn/viewer-manual.html`
- [x] 更新 `site/doc/en/viewer-manual.html`
- [x] 确认 CN/EN 章节对齐与链接可达

### 3. 验证与收口
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新项目管理文档状态
- [x] 写任务日志（`doc/devlog/2026-02-15.md`）

## 依赖
- 以 `doc/world-simulator/viewer/viewer-manual.manual.md` 为中文基线。
- 站点发布页面位于 `site/doc/cn|en/viewer-manual.html`。

## 状态
- 当前阶段：已完成（任务 0-3 全部完成）
- 最近更新：完成手册语义增量同步（移除过时 `power_storage`，校准 Auto Focus/Auto Select 目标语法，2026-03-07）。
- 下一步：后续继续按 `doc/world-simulator/viewer/viewer-manual.manual.md` 与 viewer 实际实现滚动同步。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
