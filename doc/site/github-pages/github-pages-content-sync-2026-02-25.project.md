# GitHub Pages 内容状态同步（2026-02-25）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-content-sync-2026-02-25.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口统一指向 `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`，本文仅维护增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确输入基线（viewer 手册、world 项目状态、CLI 与 Web API 实现）

### 1. 首页与文档目录同步
- [x] 更新 `site/index.html` 与 `site/en/index.html` 的近期更新与运行口径
- [x] 更新 `site/doc/cn/index.html` 与 `site/doc/en/index.html` 的手册状态摘要
- [x] 校对中英文锚点与入口链接一致性

### 2. 手册正文同步
- [x] 更新 `site/doc/cn/viewer-manual.html`
- [x] 更新 `site/doc/en/viewer-manual.html`
- [x] 补齐默认 LLM/`--no-llm`、legacy 控制面参数下线说明、Web step 控制、通用 target 语法

### 3. 验证与收口
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新本项目管理文档状态
- [x] 写任务日志（`doc/devlog/2026-02-25.md`）

## 依赖
- 继续沿用 `site/` 静态目录与 GitHub Pages 工作流。
- 内容基线以 `doc/world-simulator/viewer/viewer-manual.manual.md` 和已合入代码行为为准。

## 状态
- 当前阶段：已完成（任务 0/1/2/3 全部完成）
- 最近更新：执行统一校验 `env -u RUSTC_WRAPPER cargo check` 并完成文档收口（2026-02-25）
- 下一步：无（本轮同步完成）。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
