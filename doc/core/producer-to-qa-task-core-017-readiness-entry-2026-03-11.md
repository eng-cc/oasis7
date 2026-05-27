# Role Handoff Brief

审计轮次: 4

## Meta
- Handoff ID: `HANDOFF-CORE-017-2026-03-11-READINESS-ENTRY`
- Date: `2026-03-11`
- From Role: `producer_system_designer`
- To Role: `qa_engineer`
- Related PRD-ID: `PRD-CORE-005`
- Related Task ID: `TASK-CORE-017`
- Priority: `P1`

## Goal
- 复核“发布候选 readiness 统一入口”专题已具备完成态证据链，并确认 `TASK-CORE-018` 可继续实例化首份候选看板。

## Why Now
- `TASK-CORE-016` 已选定该主题为第一优先级；当前需要先把字段、槽位和聚合规则冻结，后续实例化才不会漂移。

## Inputs
- 代码 / 文档入口：`doc/core/project.md`、`doc/core/release-candidate-readiness-entry-2026-03-11.prd.md`
- 已完成内容：统一入口字段、证据槽位、总状态规则与下一任务回写
- 已知约束：本次不实例化具体候选看板
- 依赖前置项：`TASK-CORE-016`

## Expected Output
- 接收方交付物 1：确认 `TASK-CORE-017` 满足 `test_tier_required`
- 接收方交付物 2：如发现字段缺口，仅登记意见，不重开优先级清单任务
- 需要回写的文档 / 日志：`doc/devlog/README.md`

## Done Definition
- [x] 已冻结统一入口字段与状态规则
- [x] 已回写 core 主项目下一任务为 `TASK-CORE-018`
- [x] 已补齐 handoff

## Risks / Blockers
- 风险：若 `TASK-CORE-018` 未及时实例化，该入口仍停留在模板层
- 阻断项：无
- 需要升级给谁：如 QA 认为证据槽位定义不足，升级给 `producer_system_designer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n "gameplay|playability|testing|runtime|core|blocked 即整体 blocked" doc/core/release-candidate-readiness-entry-2026-03-11.prd.md && rg -n "TASK-CORE-018|当前状态: active" doc/core/project.md`

## Notes
- 接收方确认范围：`已确认 readiness 统一入口定义完成，可继续实例化首份候选看板`
- 接收方确认 ETA：`2026-03-11 same-day`
- 接收方新增风险：`无`
