# Role Handoff Brief

审计轮次: 1

## Meta
- Handoff ID: `HANDOFF-GAME-036-2026-03-22-LIMITED-PREVIEW-LIVEOPS`
- Date: `2026-03-22`
- From Role: `producer_system_designer`
- To Role: `liveops_community`
- Related PRD-ID: `PRD-GAME-010`
- Related Task ID: `TASK-GAME-036`
- Priority: `P0`

## Goal
- 执行 1 轮 controlled builder-facing 的 `limited playable technical preview` builder callout，并把反馈、事故与 claim drift 信号完整回流。

## Why Now
- 当前 unified gate 已 `pass`，但团队还没有验证这层新 claim envelope 在真实外部信号中是否稳定。
- 如果现在不做，制作人仍只能基于内部证据判断，无法判断口径是否会在真实渠道中失控。

## Inputs
- 代码 / 文档入口：`doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`、`doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`、`doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
- 已完成内容：统一 gate 已 `pass`；当前对外口径已收口为 `limited playable technical preview`
- 已知约束：不得说 `closed beta` / `live now` / `play now` / `public launch` / `official integration announced`
- 依赖前置项：现有 feedback/incident 模板与 GitHub CTA 已存在

## Expected Output
- 接收方交付物 1：1 条 controlled builder-facing callout 与固定窗口巡检记录（当前 round-1 主线程：`eng-cc/oasis7#48`）
- 接收方交付物 2：按 `Blocking / Opportunity / Idea` 归档的首批外部信号
- 接收方交付物 3：producer 可读的聚合摘要（来源、频次、owner、next action）
- 需要回写的文档 / 日志：`doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`、`doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`、`doc/devlog/2026-03-22.md`

## Done Definition
- [ ] 完成 1 条 controlled builder-facing callout，并显式使用 `limited playable technical preview` 与 GitHub CTA
- [ ] 至少收集 3 条有效信号，并完成 owner / next action 归档
- [ ] 所有高可见度 claim drift 都在同轮巡检中被纠偏并留痕

## Risks / Blockers
- 风险：真实渠道把当前状态误解为公开可玩、已开 beta 或已正式集成
- 阻断项：若出现未及时纠偏的高可见度误导性声明，则不得继续扩大本轮外放
- 需要升级给谁：`producer_system_designer`、必要时 `qa_engineer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n "limited playable technical preview|candidate-feedback|Blocking|Opportunity|Idea|closed beta|play now|live now|blocked_before_publish" doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md doc/readme/governance/readme-limited-preview-round1-execution-2026-03-27.md doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md doc/devlog/2026-03-27.md`

## Notes
- 接收方确认范围：`按 readme-limited-preview-invite-pack-2026-03-22.md 的 approved main copy / first comment / monitoring windows 执行，不扩大 claim envelope`
- 接收方确认 ETA：`2026-03-27 已通过 GitHub issue 发布 round-1 主线程；接下来 24h 内完成首轮巡检与 signal 归档`
- 接收方新增风险：`Moltbook 在 2026-03-27 首次尝试时返回 ERR_CONNECTION_CLOSED，因此 round-1 主线程已切换为 GitHub issue eng-cc/oasis7#48；当前风险转为“首批有效 builder signals 尚未落地”而非“无法发布 callout”`
