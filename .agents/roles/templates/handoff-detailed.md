# Role Handoff Detailed

## Meta
- Handoff ID:
- Date:
- From Role: `producer_system_designer | runtime_engineer | wasm_platform_engineer | agent_engineer | viewer_engineer | qa_engineer | liveops_community`
- To Role: `producer_system_designer | runtime_engineer | wasm_platform_engineer | agent_engineer | viewer_engineer | qa_engineer | liveops_community`
- Related Module:
- Related PRD-ID:
- Related Task UID:
- Priority: `P0` / `P1` / `P2`
- Expected ETA:

> 角色名只能使用 `.agents/roles/*.md` 中已存在的标准角色名，禁止自造别名。

## Objective
- 目标描述：
- 成功标准：
- 非目标：

## Current State
- 当前实现 / 文档状态：
- 已确认事实：
- 待确认假设：
- 当前失败信号 / 用户反馈：

## Scope
- In Scope:
- Out of Scope:

## Inputs
- 关键文件：
- 关键命令：
- 上游依赖：
- 现有测试 / 证据：

## Brainstorming / Option Framing
- Need Bounded Brainstorming:
- Scope Split / Decomposition:
- Option 1:
- Option 2:
- Option 3 (If Needed):
- Recommended Direction:
- Visual Companion Need / Non-Need:

## Behavior-First Test Plan
- Behavior Contract:
- Target Test Files / Test Surface:
- RED Verification Command:
- Expected RED Failure:
- GREEN / Safety Command:
- Skip Reason If Not Applicable:

## Subagent Contract
- Slice Type: `analysis | implementation | verification | supplemental_review | liveops_feedback`
- Write Scope:
- Read-Only Dependencies:
- Return Contract:
- Formal Sink / Writeback Surface:
- Integration Owner:
- Integration Order:

## Requested Work
- 工作项 1：
- 工作项 2：
- 工作项 3：

## Atomic Steps
| Step | Action | Validation Command | Expected Result | Actual Result | Blocker / Next Action |
| --- | --- | --- | --- | --- | --- |
| 1 |  |  |  |  |  |
| 2 |  |  |  |  |  |
| 3 |  |  |  |  |  |
| 4 |  |  |  |  |  |

> 若同一验证连续失败两次且没有新信息，必须在 `Blocker / Next Action` 明确写出失败签名和需要补的文档/决策/输入，而不是继续猜测实现。

## Expected Outputs
- 代码改动：
- 文档回写：
- 测试记录：
- task execution log：
- subagent 回传物类型：

## Done Definition
- [ ] 输出满足目标与成功标准
- [ ] 影响面已核对上游 / 下游角色
- [ ] 若任务存在方向/范围不确定性，bounded brainstorming 的方案对比与推荐方向已写清
- [ ] behavior-first RED plan 已执行，或 skip 原因已写清
- [ ] write scope / return contract / integration order 已被遵守
- [ ] 对应 `prd.md` / `project.md` 已回写
- [ ] 对应 `.pm/tasks/<TASK-UID>.execution.md` 已记录
- [ ] required/full 测试证据已补齐

## Risks / Decisions
- 已知风险：
- 待拍板事项：
- 建议决策：

## Validation Plan
- 测试层级：`test_tier_required` / `test_tier_full`
- 验证命令：
- Claim-Ready Command (If This Slice Supports Completion Claim):
- 预期结果：
- 回归影响范围：

## Handoff Acknowledgement
- 接收方确认范围：
- 接收方确认 ETA：
- 接收方新增风险：
