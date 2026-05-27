# Role Handoff Brief

审计轮次: 1

## Meta
- Handoff ID: `HANDOFF-GAME-032-2026-03-21-CLOSED-BETA-LIVEOPS`
- Date: `2026-03-21`
- From Role: `producer_system_designer`
- To Role: `liveops_community`
- Related PRD-ID: `PRD-GAME-009`
- Related Task ID: `TASK-GAME-032`
- Priority: `P1`

## Goal
- 为封闭 Beta 候选阶段准备对外 runbook、招募/反馈/事故回流模板与禁语清单，但在 producer 放行前继续保持 `technical preview` 口径。

## Why Now
- 当前运营准备已从“有推广方案”进入“有持续 SOP”，但仍缺一份明确服务于封闭 Beta 候选的口径包与回流包。

## Inputs
- 代码 / 文档入口：`doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`、`doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
- 已完成内容：Moltbook 渠道 runbook、首周模板、评论分级与 GitHub 回流路径已建档
- 已知约束：当前不能对外说 `closed beta` / `play now` / `live now`
- 依赖前置项：现有 readme governance 文档与 `technical preview` 口径

## Expected Output
- 接收方交付物 1：封闭 Beta 候选 runbook / FAQ / 招募与反馈模板
- 接收方交付物 2：禁语清单与升级条件
- 接收方交付物 3：事故摘要与用户反馈回流模板
- 需要回写的文档 / 日志：`doc/readme/*`、必要时 `doc/playability_test_result/*`、`doc/devlog/README.md`

## Done Definition
- [ ] runbook 已覆盖招募、反馈、事故回流
- [ ] 禁语清单与可用口径已明确
- [ ] 在 producer 放行前仍保持 `technical preview` 口径

## Risks / Blockers
- 风险：提前放宽对外说法会制造超出当前成熟度的承诺
- 阻断项：若无事故回流模板与反馈升级路径，则不得扩大对外承接
- 需要升级给谁：`producer_system_designer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n "technical preview|not playable yet|closed beta|play now|live now" doc/readme/governance doc/readme`

## Notes
- 接收方确认范围：`待 liveops_community 确认`
- 接收方确认 ETA：`待 liveops_community 确认`
- 接收方新增风险：`待 liveops_community 回写`
