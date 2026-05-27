# Role Handoff Brief

审计轮次: 6

## Meta
- Handoff ID: `HANDOFF-README-014-2026-03-19-LIVEOPS-TO-PRODUCER`
- Date: `2026-03-19`
- From Role: `liveops_community`
- To Role: `producer_system_designer`
- Related PRD-ID: `PRD-README-010 / PRD-README-MOLT-001/002/003`
- Related Task ID: `TASK-README-014`
- Priority: `P1`

## Goal
- 请 `producer_system_designer` 审核 Moltbook 推广方案，确认所有对外表述仍然停留在当前技术预览与三访问面 claim envelope 之内。

## Why Now
- Moltbook 当前公开定位与 oasis7 的 agent-native 叙事高度相关，适合先做渠道方案冻结；但这类外部推广直接触碰产品承诺边界，必须先做 owner 审核。

## Inputs
- 代码 / 文档入口：`doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
- 已完成内容：已整理平台现状、内容支柱、30 天节奏、禁宣称项、CTA 和反馈回流机制
- 已知约束：当前仅形成推广方案，不代表已批准真实发帖文案或平台集成承诺
- 依赖前置项：`TASK-README-014`

## Expected Output
- 接收方交付物 1：确认方案可作为 Moltbook 首轮运营动作和贴文草案的边界文件
- 接收方交付物 2：如发现越界承诺，仅回写裁剪意见和禁宣称补充
- 需要回写的文档 / 日志：`doc/devlog/README.md`

## Done Definition
- [x] 已明确 Moltbook 当前公开机制与 oasis7 对应定位
- [x] 已明确禁宣称项与安全替代表述
- [x] 已明确评论升级路径与 owner 审核链

## Risks / Blockers
- 风险：若真实发帖直接跳过本方案，可能再次把技术预览写成正式发布承诺
- 阻断项：无
- 需要升级给谁：`liveops_community`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`rg -n '技术预览|禁宣称项|30 天执行节奏|反馈回流|producer_system_designer' doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md doc/readme/governance/liveops-to-producer-task-readme-014-moltbook-promotion-plan-2026-03-19.md`

## Notes
- 接收方确认范围：`仅确认推广方案边界，不等于逐条批准具体 Moltbook 发帖文案`
- 接收方确认 ETA：`2026-03-19 same-day`
- 接收方新增风险：`如 Moltbook 平台机制变化，执行前需重新复核主页 / developers / help 页面`
