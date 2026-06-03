# Subagent Slice Card

> 用途：在 `tpm` 派工前，先把每个专业角色 subagent slice 固化为单卡，确保 owner / task / worktree / PR 主链一致，并必须回写 `.pm/tasks/<TASK-UID>.execution.md`。

## Required Fields（固定字段）
- role:
- slice type:
- model configuration: compatibility marker; fill the intended/actual fields below.
- intended model configuration: `gpt-5.5-medium` by default; record reason for any requested override.
- actual dispatched model/reasoning: selected model/reasoning when the tool permits and reports it; otherwise `inherited/unverified` plus connector/tool limitation, including cases where selection was requested but actual dispatch cannot be verified.
- context delivery mode: full-thread/full-history fork by default; explicit context packet is delivery supplement/fallback only, with reason recorded.
- mandatory context packet: compatibility marker; fill the mandatory context checklist/packet below.
- mandatory context checklist/packet:
  - identity and authority: assigned role + `.agents/roles/<role>.md` + owner role + TPM integration owner
  - workflow governance: `AGENTS.md` + `doc/engineering/workflow/source-of-truth.md` + selected workflow skill(s)
  - task truth: `.pm/tasks/<TASK-UID>.yaml` + `.pm/tasks/<TASK-UID>.execution.md` + canonical worktree/branch/base ref + PR link/status if present
  - user intent: request summary + current TODO + non-goals + done/verification expectations
  - scoped repo context: relevant `prd.md` / `project.md` / handoff + changed paths + current diff/evidence summary + constraints
  - collaboration boundary: sibling slices + write-scope conflicts + integration order + allowed commands + return contract + formal sink
- write scope:
- return contract:
- validation command:
- formal sink: `.pm/tasks/<TASK-UID>.execution.md`（mandatory；其他 sink 只能补充）
- integration order:
- context exemption: none, or explicit reason for narrow read-only explorer slice only

## Disjoint Scope Checklist（并行写入必填）
- [ ] 本 slice 的 write scope 与其他并行 slice 完全不重叠（文件/目录/模块级）。
- [ ] 若存在共享文件，已降级为串行集成，且在 integration order 中标注先后。
- [ ] 已声明本 slice 不得回退或覆盖其他 subagent 已落地改动。
- [ ] 已声明冲突处理策略（rebase / manual merge / owner 手工集成）。
- [ ] 已定义完成信号（patch / findings / evidence / review）与回传路径，避免并行漂移到新真值。

## Example (copy/paste)
- role: producer_system_designer
- slice type: implementation
- model configuration: see intended/actual fields
- intended model configuration: `gpt-5.5-medium`
- actual dispatched model/reasoning: `gpt-5.5-medium`, or `inherited/unverified` with reason if the connector cannot select/report the model or actual dispatch cannot be verified
- context delivery mode: full-thread/full-history fork
- mandatory context packet: see mandatory context checklist/packet
- mandatory context checklist/packet: `AGENTS.md` + `.agents/roles/producer_system_designer.md` + `doc/engineering/workflow/source-of-truth.md` + `doc/<module>/project.md` task `<task slug>` + `.pm/tasks/<TASK-UID>.yaml` + `.pm/tasks/<TASK-UID>.execution.md` + current branch/diff summary
- write scope: `crates/foo/**`（disjoint）
- return contract: patch + test evidence
- validation command: `./scripts/cargo-dev.sh test -p foo`
- formal sink: `.pm/tasks/<TASK-UID>.execution.md`（mandatory）
- integration order: 2/3（after runtime slice, before qa slice）
- context exemption: none

## Notes
- 一个 slice 一张卡；多角色并行时，必须逐张卡校验 disjoint scope checklist。
- slice card 链接/引用必须回写 `.pm/tasks/<TASK-UID>.execution.md` 对应条目；未写入 task execution log 的派工不视为有效派工。
- 除窄范围只读 explorer 且写明豁免原因外，`AGENTS.md`、对应 role card、workflow source-of-truth、当前 `.pm` task yaml/execution log 必须包含在 context packet 中。
