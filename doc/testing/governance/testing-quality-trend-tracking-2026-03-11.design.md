# oasis7: 测试质量趋势跟踪（2026-03-11）设计

- 对应需求文档: `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md`
- 对应项目管理文档: `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.project.md`

审计轮次: 4

## 1. 设计定位
定义 testing 模块的质量趋势统计结构，统一样本字段、计算公式、阈值分级与 baseline 发布方式。

## 2. 设计结构
- 样本层：从 evidence / project / devlog 抽取 `sample_id`、首次结论、最终结论、发现/关闭日期。
- 指标层：计算 `首次通过率`、`阶段内逃逸率`、`平均修复时长`。
- 报告层：输出 baseline Markdown，附窗口、样本明细、风险结论与下一步建议；`producer_system_designer` 等消费角色直接读取该报告，具体阶段评审交接留在 GitHub task issue evidence comments。

## 3. 关键接口 / 入口
- 专题 PRD：`doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md`
- 专题 Project：`doc/testing/governance/testing-quality-trend-tracking-2026-03-11.project.md`
- baseline 报告：`doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`

## 4. 约束与边界
- 只统计仓内可追溯的测试/复验样本，不补写不可验证的历史数据。
- baseline 允许小样本，但必须显式写出样本数与统计窗口。
- 线上生产逃逸率不在本轮定义范围内。

## 5. 设计演进计划
- 先冻结样本字段与阈值。
- 再发布首份 baseline。
- 后续按周续写趋势并纳入阶段评审。
