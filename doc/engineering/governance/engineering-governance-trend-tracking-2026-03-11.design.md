# oasis7: 工程门禁趋势跟踪（2026-03-11）设计

- 对应需求文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`
- 对应项目管理文档: `doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
定义 engineering 门禁趋势统计结构，统一样本字段、公式、阈值、baseline 报告与当前任务证据回流方式。

## 2. 设计结构
- 样本层：从 `doc/engineering/project.md`、GitHub task issue evidence comments、正式 evidence 与 `scripts/doc-governance-check.sh` 演进记录抽取 `sample_id`、样本基数、残留违规数、发现/关闭日期、回归数。
- 指标层：计算 `违规率`、`平均修复时长`、`回归率` 三项指标。
- 报告层：输出 baseline Markdown，包含窗口、样本明细、红黄绿结论、风险与建议动作。
- 交接层：通过 GitHub task issue evidence 与 pre-PR local role review evidence 固化审查入口。

## 3. 关键接口 / 入口
- 专题 PRD：`doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.prd.md`
- 专题 Project：`doc/engineering/governance/engineering-governance-trend-tracking-2026-03-11.project.md`
- baseline 报告：`doc/engineering/evidence/engineering-governance-trend-baseline-2026-03-11.md`
- 审查证据：GitHub task issue evidence comments 与 pre-PR local role review packet。

## 4. 约束与边界
- 只统计仓内可追溯的 engineering 门禁治理样本，不补写不可核验的历史任务。
- baseline 允许小样本，但必须显式写出样本数、窗口与 small-sample 风险。
- 本轮不引入自动化聚合脚本。

## 5. 设计演进计划
- 先冻结样本字段、指标公式与阈值。
- 再发布首份 engineering baseline。
- 后续在 `TASK-ENGINEERING-004` 中将季度审查模板与趋势基线对齐。
