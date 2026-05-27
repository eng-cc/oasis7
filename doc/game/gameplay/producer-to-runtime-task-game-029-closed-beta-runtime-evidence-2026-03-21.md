# Role Handoff Brief

审计轮次: 1

## Meta
- Handoff ID: `HANDOFF-GAME-029-2026-03-21-CLOSED-BETA-RUNTIME`
- Date: `2026-03-21`
- From Role: `producer_system_designer`
- To Role: `runtime_engineer`
- Related PRD-ID: `PRD-GAME-009`
- Related Task ID: `TASK-GAME-029`
- Priority: `P0`

## Goal
- 为 `closed_beta_candidate` 准入准备 runtime 侧最小硬证据包：five-node no-LLM soak、replay/rollback drill 与 longrun release gate。

## Why Now
- 当前技术底座已接近封闭 Beta，但还缺连续、可复跑、候选版本口径的长期在线与恢复证据。
- 若不先补这条线，阶段判断会继续停留在“专题都不错，但整体还不能升阶”。

## Inputs
- 代码 / 文档入口：`doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`、`doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- 已完成内容：权威分层、回放/回滚、反作弊、经济审计、可运维发布阻断规则已建档并有实现切片
- 已知约束：必须以 fresh bundle / 候选版本口径为准，source tree 结果不能替代
- 依赖前置项：`doc/testing/longrun/s10-five-node-real-game-soak.prd.md`

## Expected Output
- 接收方交付物 1：five-node no-LLM soak 候选版本证据
- 接收方交付物 2：replay/rollback drill 候选版本证据
- 接收方交付物 3：给 `qa_engineer` 可直接消费的 runtime release gate 摘要
- 需要回写的文档 / 日志：对应 evidence、`doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`、`doc/devlog/README.md`

## Done Definition
- [ ] five-node no-LLM soak 证据可复跑
- [ ] replay/rollback drill 证据可复跑
- [ ] 阻断项与非阻断项已分级回写

## Risks / Blockers
- 风险：若只跑 source tree，不跑候选 bundle，会高估当前稳定性
- 阻断项：出现 longrun / rollback blocking 失败则必须维持当前阶段
- 需要升级给谁：`producer_system_designer`、`qa_engineer`

## Validation
- 建议测试层级：`test_tier_required` + `test_tier_full`
- 建议验证命令：沿用 `s10-five-node-real-game-soak`、rollback drill 与当前 longrun release gate 命令

## Notes
- 接收方确认范围：`待 runtime_engineer 确认`
- 接收方确认 ETA：`待 runtime_engineer 确认`
- 接收方新增风险：`待 runtime_engineer 回写`
