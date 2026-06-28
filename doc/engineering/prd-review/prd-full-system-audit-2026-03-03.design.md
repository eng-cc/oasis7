# 全量 PRD 体系审读与对齐设计（2026-03-03）

- 对应需求文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md`
- 对应项目管理文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md`

## 1. 设计定位
定义全量 PRD 审读的执行面、清单结构、偏差回写链路与周度增量机制，确保审读结果可追溯、可复核。

## 2. 设计结构
- 历史留痕层：2026-03 审读 snapshot 已退役为历史审计语义；当前逐篇审读追溯通过 round review logs 与模块入口/索引进入。
- 审读层：按模块审查文档与实现、索引、替代链的一致性。
- 修复层：对偏差执行回写、合并、重定向与引用修复。
- 运营层：周度增量巡检承接新增或变更 PRD。

## 3. 关键接口 / 入口
- 历史审读证据：core review round logs 与已删除 review snapshot 的非路径历史条目。
- 模块入口：`doc/*/prd.md`、`doc/*/project.md`、`doc/*/prd.index.md`
- 治理门禁：`scripts/doc-governance-check.sh`

## 4. 约束与边界
- 历史审读必须可追溯到 round logs 或已删除 review snapshot 的历史条目，不能重新恢复为当前活跃旧清单入口。
- 偏差回写遵循“代码为准、文档收口”的治理策略。
- 历史替代链和引用断链必须在当前批次闭环。

## 5. 设计演进计划
- Phase-0：2026-03 阶段建立全量审读留痕与首批审读。
- Phase-1：按模块推进并修偏。
- Phase-2：进入周度增量巡检。
