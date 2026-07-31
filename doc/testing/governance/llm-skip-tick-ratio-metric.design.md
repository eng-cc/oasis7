# oasis7: LLM 跳过 Tick 占比指标设计

- 对应需求文档: `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md`
- 可变任务状态与历史: GitHub task issue evidence comments

## 1. 设计定位
定义测试治理与发布门禁指标专题设计，统一度量口径、阻断策略与审计回写。

## 2. 设计结构
- 指标定义层：明确治理指标、统计口径与采集边界。
- 门禁对齐层：把指标与 release gate / policy / determinism guard 对齐。
- 异常阻断层：为越界指标和违规状态建立阻断规则。
- 审计回写层：把治理结论沉淀到日志、发布与追踪文档。

## 3. 关键接口 / 入口
- 治理指标入口
- release gate / policy 对齐点
- 异常阻断阈值
- 治理审计记录

## 4. 约束与边界
- 治理指标必须与发布口径一致。
- 阻断规则应明确且可解释。
- 不在本专题扩展新的治理组织流程。

## 5. 设计演进计划
- 先固定指标与统计口径。
- 再对齐门禁策略。
- 最后固化阻断与审计。
