# GitHub Pages 对外展示站点（项目管理文档）

- 对应设计文档: `doc/site/github-pages/github-pages-showcase.design.md`
- 对应需求文档: `doc/site/github-pages/github-pages-showcase.prd.md`

审计轮次: 5

## 审计备注
- 主项目入口：`doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md`
- 本文仅维护本专题增量任务，不重复主项目文档任务编排。

## 任务拆解
- [x] 输出设计文档（`doc/site/github-pages/github-pages-showcase.prd.md`）
- [x] 输出项目管理文档（本文件）
- [x] 明确首版信息架构与对外文案（首页各模块）
- [x] 新增站点目录与静态资源骨架（`site/`）
- [x] 实现首页 UI（响应式布局 + 中英文展示策略）
- [x] 准备首版展示素材（矢量示意图 `site/assets/images/world-loop.svg`）
- [x] 新增 GitHub Pages 工作流（`.github/workflows/pages.yml`）
- [x] 在 `README.md` 增加 Pages 入口
- [x] 本地自检链接与资源路径（项目页子路径）
- [x] 更新任务日志（`doc/devlog/README.md`）
- [x] 运行测试（`env -u RUSTC_WRAPPER cargo check`）
- [x] 更新方案文档：补充中英文双版目标/范围/验收
- [x] 新增英文首页（`site/en/index.html`）
- [x] 新增中英文切换入口（CN/EN）
- [x] 本地预览双页并截图核对（agent-browser）
- [x] 同步更新任务日志（`doc/devlog/README.md`）
- [x] 新增首访按浏览器语言自动跳转（EN）
- [x] 新增手动语言选择记忆（停止后续自动跳转）
- [x] agent-browser 验证自动跳转与手动选择记忆
- [x] 调整语言标识为右上角弱化样式（低视觉权重）
- [x] 改为最右侧地球图标按钮 + 语言下拉

## 依赖
- GitHub 仓库 Settings 中启用 Pages（Source: GitHub Actions）。
- 现有 CI 工作流与 `pages.yml` 并行运行，互不影响。
- 展示素材需要来自当前可运行场景（viewer 截图或录屏）。

## 状态
- 当前阶段：已完成（双语增强）
- 最近更新：完成最右侧地球图标语言按钮（含下拉）并通过 agent-browser 截图确认（2026-02-10）
- 下一步：可按需补充双语 FAQ 与演示素材（真实 viewer 截图/GIF）。

## 迁移记录（2026-03-03）
- 已按 `TASK-ENGINEERING-014-D1 (PRD-ENGINEERING-006)` 从 legacy 命名迁移为 `.prd.md/.project.md`。
- 保留原任务拆解、依赖与状态语义，不改变既有结论。
