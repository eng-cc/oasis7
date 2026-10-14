# Site 使用手册静态化（CN/EN）设计文档

- 对应设计文档: [Site 静态手册共享设计](../design.md#static-manual-design)；原同名 design 仅保留兼容入口。
- 执行记录: Git、PR、实际 CI 和评审记录；Issue 与 Project 按需使用

审计轮次: 5

## ROUND-002 主从口径
- 本文件为 manual 主文档；已完成的 Viewer 手册搬迁增量已回填到产品模式、canonical Viewer 手册和本专题镜像治理中，不再保留独立日期型子文档。

- 对应标准执行入口: `doc/site/manual/site-manual-static-docs.prd.md`

## 目标
- 在 `site/doc/cn` 与 `site/doc/en` 下建立可直接发布到 GitHub Pages 的手册站框架。
- 将现有 Viewer 用户手册整理为站内可访问内容，形成“首页 -> 文档目录 -> 手册正文”的闭环。
- 保持当前纯静态部署模式（无需新增构建系统）。

<a id="static-manual-requirements"></a>
## 范围
- 范围内
  - 新增文档目录页（CN/EN）与基础布局框架。
  - 新增手册正文页（CN/EN）并接入站点语言切换。
  - 在现有中英文首页增加“使用手册”入口。
  - 补充文档页样式与最小交互脚本。
- 范围外
  - 引入 SSG（VitePress/MkDocs）或服务端渲染。
  - 一次性迁移全部 `doc/` 技术文档。
  - 改动主站现有信息架构与交互主流程。

## 接口/数据
- 页面目录
  - `site/doc/cn/index.html`
  - `site/doc/en/index.html`
  - `site/doc/cn/viewer-manual.html`
  - `site/doc/en/viewer-manual.html`
- 样式/交互
  - `site/assets/styles.css`
  - `site/assets/app.js`
- 入口页面
  - `site/index.html`
  - `site/en/index.html`
- 内容来源
  - `doc/world-simulator/viewer/viewer-manual.manual.md`（中文原稿）

## 里程碑

以下为原静态化迁移阶段的历史规划，不要求再次迁移或恢复 Project／daily devlog。

- M1：需求、设计与原任务拆解。
- M2：CN/EN 目录页与导航基础能力。
- M3：两种语言的正文接入与互链。
- M4：原迁移的链接、构建与记录收尾；其命令和结果只代表原窗口。

当前维护按共享设计同步原稿、双语镜像与受影响入口，使用现行测试手册的适用检查；
Git、PR、实际 CI 与评审承载本次交付事实，不以旧计划或旧 cargo 记录证明当前通过。
原阶段说明可从本文件合并前 Git 版本追溯。

## 风险
- 风险：CN/EN 内容漂移。
  - 缓解：同任务内成对维护 `cn/en` 页面，并在目录页统一挂载。
- 风险：手册内容更新后站内版本过时。
  - 缓解：明确 `doc/world-simulator/viewer/viewer-manual.manual.md` 为内容基线，后续按任务滚动同步。
- 风险：文档页样式影响首页样式。
  - 缓解：新增样式尽量限定在 `.docs-*` 命名空间内。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
