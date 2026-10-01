---
name: default-workflow-bootstrap
description: Use when an oasis7 request has write effects and needs canonical task truth, a worktree, and an owner before routing.
---

# Default Workflow Bootstrap

Canonical lifecycle and authority: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done).

## When to Use

Use for requests that will change repository files, task truth, remote state, or another persistent system. Read-only questions, fact lookup, professional analysis, and side-effect-free checks may proceed directly without creating task truth, a branch, worktree, or review artifact. A professional conclusion still belongs to its matching role; a read-only result is not formal review or release evidence.

Before bootstrapping a task to change developer workflow, CI routing/gates, review/merge policy, or workflow helpers—or before capturing a reflection, creating/promoting task truth, or expanding an existing task to do so—apply the [canonical prior-approval rule](../../../doc/engineering/workflow/source-of-truth.md#workflow-change-approval). This stop precedes task binding and workspace creation. A direct scoped user request counts. Read-only audits/diagnosis use ordinary bootstrap but do not authorize policy changes; CI failures and review findings do not count as approval. Alternate intake or reflection paths do not bypass the stop.

## Procedure

1. Classify whether the requested action has an external or persistent side effect. If none, answer directly or use the matching professional role for judgment, and stop before task creation. If the request mixes a read-only answer with a proposed change, answer the read-only part and wait for actual write authorization before bootstrapping the change.
2. For authorized write work, bind a single Project-backed task, owner, allowed scope, exclusions, acceptance target, and any separately authorized dangerous effects before editing. Reuse a valid canonical task only when identity and scope still match. For a new task, capture the machine-readable result:

```bash
./scripts/new-task-worktree.sh <module> <task> \
  --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref> --json
```

TPM is the default coordinator and continuation owner, not the task outcome
owner. Determine and bind the matching professional role as `owner_role`;
reuse an existing owner only when task truth still validates it. Create a
dedicated worktree unless the user explicitly authorized reuse. Professional
work still requires matching bounded subagent slices.
3. Treat a successful helper JSON result with `pm_task.bootstrap_complete=true`, `status=committed`, `workflow_started=true`, and non-empty `bootstrap_snapshot_path`/`bootstrap_snapshot_digest` as complete bootstrap confirmation. For a reused task or partial result, verify its live identity with `workflow-report --phase start` and `bootstrap-task-snapshot.py validate-or-create`. A new HEAD inside the same goal and scope does not itself renew authorization or create a new epoch.
4. If the helper reports a partial remote creation, preserve the worktree and follow its printed `resume-bootstrap` command. Recovery repeats only the missing journaled step and preserves the same task/request/epoch.
5. Record task authorization and material lifecycle changes in the canonical GitHub task Issue. Do not require an Issue comment for each local command or intermediate step.
6. Once task truth exists, hand off to `repo-owned-workflow-router` via `./.agents/skills/repo-owned-workflow-router/SKILL.md`.

## Required Output for Write-Task Bootstrap

- `## Repository State Impact`
- `## Isolation Decision`
- `## Task Truth`
- `## Bootstrap Snapshot` (path + digest)
- `## Routed Next Phase`

Do not force this bootstrap onto read-only requests. Do not treat professional/domain judgments as TPM-owned conclusions; route them to the matching role when needed without creating task truth solely for that analysis.

Already-bound work continues under the existing task authorization while goal, scope, and dangerous-effect boundary remain stable; do not emit a new bootstrap packet for ordinary commits, tests, diagnosis, repair, or main advancement.

## Guardrails

For an explicitly bound manual loop task, use `scripts/pm/loop.py` with the effective trusted tool root before admission or continuation. Preserve the exact loop binding, immutable contracts, scope and user merge hold. At stable waits return resumable evidence without heartbeat or scheduled continuation; completion never starts another task. See [manual entry authority](../../../doc/engineering/workflow/source-of-truth.md#manual-three-loop-transition). Legacy tasks retain their existing route.

Do not perform a write before task truth is bound. A read-only answer does not require binding.

## Known Failure Modes

Reusing an incompatible task/worktree; treating read-only analysis as write authorization; creating task truth for a side-effect-free answer.
