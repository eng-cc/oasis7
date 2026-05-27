# Role Handoff Brief

审计轮次: 4

## Meta
- Handoff ID: `HANDOFF-CORE-023-2026-03-11-DOC-README`
- Date: `2026-03-11`
- From Role: `producer_system_designer`
- To Role: `qa_engineer`
- Related PRD-ID: `PRD-CORE-007/008`
- Related Task ID: `TASK-CORE-023`
- Priority: `P2`

## Goal
- 复核 `doc/README.md` 已同步当前公开阅读路径与更新时间，确保工程总入口不再沿用旧顺序。

## Why Now
- repo / site / release communication 都已更新；总入口若不跟上，会让新协作者从旧入口理解项目状态。

## Inputs
- 代码 / 文档入口：`doc/README.md`、`README.md`、`site/index.html`
- 已完成内容：总入口已回写根 README / site 作为新的前置阅读入口
- 已知约束：不重写模块矩阵
- 依赖前置项：`TASK-README-010`、`TASK-SITE-010`

## Expected Output
- 接收方交付物 1：确认 `doc/README.md` 满足 `test_tier_required`
- 接收方交付物 2：如有路径遗漏，仅登记入口缺口
- 需要回写的文档 / 日志：`doc/devlog/README.md`

## Done Definition
- [x] 更新时间已同步到 `2026-03-11`
- [x] 快速阅读路径已纳入根 README / site 入口
- [x] core 主项目已追踪该任务

## Risks / Blockers
- 风险：若未来继续新增公开入口，`doc/README.md` 仍需定期跟进
- 阻断项：无
- 需要升级给谁：`producer_system_designer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n '更新时间：2026-03-11|README.md|site/index.html|doc/core/prd.md' doc/README.md && rg -n 'TASK-CORE-023|当前状态: completed|下一任务: 无' doc/core/project.md`

## Notes
- 接收方确认范围：`已确认工程总入口阅读路径已追平当前公开入口状态`
- 接收方确认 ETA：`same-day`
- 接收方新增风险：`无`
