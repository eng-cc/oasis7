# GitHub Pages 视觉细节打磨 V2（2026-02-12）项目管理文档

- 对应设计文档: `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护本专题增量任务，不重复主项目文档任务编排。

## 任务拆解

### 0. 文档与基线
- [x] 新增设计文档（`doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md`）
- [x] 新增项目管理文档（本文件）
- [x] 明确本轮优化目标（移动端密度 / Hero+证据条层级 / 路线图状态编码）

### 1. 页面与样式实现
- [x] 移动端文本密度优化（字号/间距/代码块高度与滚动策略）
- [x] Hero 指标区层级强化（关键数值与标签对比）
- [x] 证据条层级强化（普通项与“近期更新”高亮分层）
- [x] 路线图状态图标与 active 轻量动态（含 reduced-motion 降级）
- [x] 中英文页面结构标记同步

### 2. 验证与收口
- [x] 截图回归（CN/EN 桌面 + 移动）
- [x] 执行 `env -u RUSTC_WRAPPER cargo check`
- [x] 更新项目管理文档状态
- [x] 写入当日开发日志（`doc/devlog/README.md`）

## 依赖
- 依赖现有 GitHub Pages 双语页面结构与样式体系。
- 不涉及后端、接口协议与第三方依赖变更。

## 状态
- 当前阶段：已完成
- 最近更新：完成 V2 视觉打磨（移动端密度压缩、Hero/证据条层级强化、路线图状态编码与轻量动态）并通过截图回归与 `cargo check`（2026-02-12）
- 下一步：结合后续传播需求评估是否增加“小屏极简版”信息层。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
