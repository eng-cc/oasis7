# Subagent Slice Card

> 用途：在 `tpm` 派工前，先把每个专业角色 subagent slice 固化为单卡，确保 owner / task / worktree / PR 主链一致，并必须回写 GitHub task issue evidence comments。

## Required Fields（固定字段）
- role:
- slice type:
- model configuration: fill the intended/actual fields below.
- intended model configuration: `inherit current parent selection` by default; record reason for any explicit model/reasoning request.
- actual dispatched model/reasoning: selected model/reasoning when the tool permits and reports it; otherwise `inherited/unverified` plus connector/tool limitation, including cases where selection was requested but actual dispatch cannot be verified.
- context delivery mode: minimal HEAD-bound task packet by default; full-thread/full-history is escalation-only, with reason recorded.
- task packet identity: task UID + canonical worktree + base ref + current/frozen HEAD + packet producer/time.
- role activation: named-role selector evidence when adapter-backed, otherwise `message-assigned; adapter inactive on this surface`.
- mandatory context checklist: fill the mandatory context checklist below.
- mandatory context checklist:
  - identity and authority: assigned role + `.agents/roles/<role>.md` + owner role + TPM integration owner
  - workflow governance: `AGENTS.md` + `doc/engineering/workflow/source-of-truth.md` + selected workflow skill(s)
  - task truth: GitHub task issue + GitHub Project item/status + `.pm/github-project-sync/tasks.json` mapping record + canonical worktree/branch/base ref + PR link/status if present
  - user intent: request summary + current TODO + non-goals + done/verification expectations
  - scoped repo context: relevant `prd.md` / `project.md` / handoff + changed paths + current diff/evidence summary + constraints
    - gameplay-sensitive slices must explicitly include `doc/game/prd.md`, `doc/game/project.md`, `doc/game/gameplay/README.md`, relevant topic PRD/project docs, and fresh playability/QA evidence expectations or an explicit non-applicability reason
  - collaboration boundary: sibling slices + write-scope conflicts + integration order + allowed commands + return contract + formal sink
- write scope:
- return contract:
- validation command:
- formal sink: GitHub task issue evidence comments（mandatory；其他 sink 只能补充）
- integration order:
- context exemption: none, or explicit reason for narrow read-only explorer slice only

## Disjoint Scope Checklist（并行写入必填）
- [ ] 本 slice 的 write scope 与其他并行 slice 完全不重叠（文件/目录/模块级）。
- [ ] 若存在共享文件，已降级为串行集成，且在 integration order 中标注先后。
- [ ] 已声明本 slice 不得回退或覆盖其他 subagent 已落地改动。
- [ ] 已声明冲突处理策略（rebase / manual merge / owner 手工集成）。
- [ ] 已定义完成信号（patch / findings / evidence / review）与回传路径，避免并行漂移到新真值。

## Example (copy/paste)
- role: gameplay_designer
- slice type: implementation
- model configuration: see intended/actual fields
- intended model configuration: `inherit current parent selection`
- actual dispatched model/reasoning: `inherited/unverified` because this dispatch surface cannot report the inherited runtime
- context delivery mode: minimal HEAD-bound task packet
- task packet identity: `<task_uid>` + `<canonical_worktree>` + `<base_ref>` + `<head_sha>` + `<producer/time>`
- role activation: `message-assigned; adapter inactive on this surface`
- mandatory context checklist: `AGENTS.md` + `.agents/roles/gameplay_designer.md` + `doc/engineering/workflow/source-of-truth.md` + `doc/game/prd.md` + `doc/game/project.md` + relevant `doc/world-simulator/**` and playability evidence + GitHub task issue evidence + `.pm/github-project-sync/tasks.json` mapping record + current branch/diff summary
- write scope: `crates/foo/**`（disjoint）
- return contract: patch + test evidence
- validation command: `./scripts/cargo-dev.sh test -p foo`
- formal sink: GitHub task issue evidence comments（mandatory）
- integration order: 2/3（after runtime slice, before qa slice）
- context exemption: none

## Notes
- 使用 `./scripts/pm/subagent-task-packet.py create|validate` 生成并验证不可覆盖的 packet；派工记录写入 packet 路径与 digest。
- 一个 slice 一张卡；多角色并行时，必须逐张卡校验 disjoint scope checklist。
- slice card 链接/引用必须回写 GitHub task issue evidence comments 对应条目；未写入 GitHub-backed task evidence 的派工不视为有效派工。
- 除窄范围只读 explorer 且写明豁免原因外，最小 task packet 必须用精确路径/链接提供 `AGENTS.md`、对应 role card、workflow source-of-truth、当前 GitHub-backed task truth 与 issue evidence；不要把这些长文本重复嵌入 packet。full-history 只用于有记录原因的升级。
- 标准角色名以 `.agents/roles/*.md` 为准；当前包含 `producer_system_designer`、`gameplay_designer`、`game_visual_interaction_designer`、`runtime_engineer`、`blockchain_ops_engineer`、`wasm_platform_engineer`、`agent_engineer`、`viewer_engineer`、`qa_engineer`、`repository_health_engineer`、`liveops_community`。
