# Viewer 使用手册内容搬迁（2026-02-15）设计文档

- 对应设计文档: `doc/site/manual/viewer-manual-content-migration-2026-02-15.design.md`
- 对应项目管理文档: `doc/site/manual/viewer-manual-content-migration-2026-02-15.project.md`

审计轮次: 5

## ROUND-002 主从口径
- 主入口文档：`doc/site/manual/site-manual-static-docs.prd.md`。
- 本文件仅维护增量专题内容。

- 对应标准执行入口: `doc/site/manual/viewer-manual-content-migration-2026-02-15.project.md`

## 目标
- 将分散在 `doc/world-simulator/viewer/viewer-*` 中与当前 `software_safe` Web 主入口相关的“用户可操作内容”并入 Viewer 使用手册。
- 形成单一入口：`doc/world-simulator/viewer/viewer-manual.manual.md`（中文基线）与 `site/doc/cn|en/viewer-manual.html`（站点发布版）。
- 保持现有“Web 默认、`software_safe` 单入口”的闭环策略不变。

## 范围
- 范围内
  - 把以下能力并入手册：
    - 自动步骤（auto select）
    - 右侧面板模块显隐与本地缓存
    - 选中详情面板能力
    - 快速定位 Agent
    - 2D 全览图缩放分层
    - 文本可选中/复制面板
    - UI 语言切换
    - 当前 Web 主入口相关的排障说明
  - 同步 `doc/world-simulator/viewer/viewer-manual.manual.md` 与 `site/doc/cn|en/viewer-manual.html`。
- 范围外
  - 迁移 `.project.md`、`devlog`、runtime 架构设计文档。
  - 改动 viewer 协议或功能实现代码。

## 接口/数据
- 输入文档
  - `doc/world-simulator/viewer/viewer-manual.manual.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `doc/world-simulator/viewer/viewer-right-panel-module-visibility.prd.md`
  - `doc/world-simulator/viewer/viewer-selection-details.prd.md`
  - `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md`
  - `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md`
  - `doc/world-simulator/viewer/viewer-copyable-text.prd.md`
  - `doc/world-simulator/viewer/viewer-i18n.prd.md`
- 输出文件
  - `doc/world-simulator/viewer/viewer-manual.manual.md`
  - `site/doc/cn/viewer-manual.html`
  - `site/doc/en/viewer-manual.html`

## 里程碑
- M1：文档与任务拆解。
- M2：完成中文基线手册合并。
- M3：完成站点 CN/EN 手册同步。
- M4：验证收口（`cargo check`、项目文档、devlog）。

## 风险
- 风险：中英文手册内容漂移。
  - 缓解：同任务内成对更新 `site/doc/cn|en`。
- 风险：搬迁后出现口径冲突（历史文档过时）。
  - 缓解：以当前已上线行为为准，不直接搬旧开关语义。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
