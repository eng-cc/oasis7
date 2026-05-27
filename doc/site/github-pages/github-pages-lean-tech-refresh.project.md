# GitHub Pages 精简科技感升级（四期）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-lean-tech-refresh.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-lean-tech-refresh.prd.md`
审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护增量任务。

## 任务拆解

### 0. 文档与基线
- [x] 新增四期设计文档（`doc/site/github-pages/github-pages-lean-tech-refresh.prd.md`）
- [x] 新增四期项目管理文档（本文件）
- [x] 明确验收目标（信息密度/视觉层级/双语一致）

### 1. 信息架构瘦身（双语同构）
- [x] 收敛导航入口（仅保留关键锚点）
- [x] 重写 Hero：一句话价值 + 关键指标 + 主次 CTA
- [x] 删除低收益冗余模块并压缩段落长度
- [x] 同步改造英文页结构与文案节奏

### 2. 视觉系统升级
- [x] 调整字体与色板，强化科技辨识度
- [x] 重新定义 section/card 层级，弱化“全局同强度高亮”
- [x] 强化关键动作区（CTA/证据/路径）视觉优先级
- [x] 优化移动端版式密度与滚动负担

### 3. 验证与收口
- [x] 静态结构自检（关键锚点与模块可达）
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新项目管理文档状态
- [x] 写入当日开发日志（`doc/devlog/README.md`）

## 依赖
- 继续沿用 `site/` 静态目录与 GitHub Pages 工作流。
- 复用现有截图与 SVG 素材，不新增重资源依赖。

## 状态
- 当前阶段：已完成
- 最近更新：完成双语首页结构瘦身、视觉升级与 `cargo check` 校验（2026-02-12）
- 下一步：可按传播目标继续增加“对比前后”专题页或短演示 GIF。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
