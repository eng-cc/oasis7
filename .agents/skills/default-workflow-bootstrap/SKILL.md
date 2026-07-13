---
name: default-workflow-bootstrap
description: Use when any oasis7 user request starts and must bind the canonical worktree, GitHub-backed task truth, and owner before routing.
---

# Default Workflow Bootstrap

Canonical lifecycle and authority: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done).

## When to Use

Use for every request, including read-only/chat-only work. Do not repeat bootstrap for a micro-loop already bound to the same task; record the minimal `Learning Intake / Loop Closeout` entry defined by the canonical source.

## Procedure

1. Classify only enough to select isolation:
   - repository-changing: requires standard worktree + GitHub Project-backed task truth before edits
   - read-only/chat-only pure fact lookup: requires standard worktree + GitHub Project-backed task truth before direct answer
   - read-only/chat-only professional judgment: requires the same binding, then a matching professional slice
2. Reuse a valid canonical task worktree only when identity matches; otherwise create one:

```bash
./scripts/new-task-worktree.sh <module> <task> \
  --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref>
```

TPM is the default coordinator and continuation owner, not the task outcome
owner. Determine and bind the matching professional role as `owner_role`;
reuse an existing owner only when task truth still validates it. Create a
dedicated worktree unless the user explicitly authorized reuse. Professional
work still requires matching bounded subagent slices.
3. Confirm task UID, issue/Project binding, owner, repository, branch, worktree, request identity, and acceptance target using `./scripts/pm/workflow-report.sh --phase start --role tpm`.
4. Record the bootstrap result in a GitHub issue evidence comment (mandatory). Fallback evidence cannot replace the GitHub-backed task evidence sink for task truth.
5. Once task truth exists, hand off to `repo-owned-workflow-router` via `./.agents/skills/repo-owned-workflow-router/SKILL.md`.

## Required Output

- `## Repository State Impact`
- `## Isolation Decision`
- `## Task Truth`
- `## Routed Next Phase`

Do force this bootstrap onto chat-only or read-only requests, even when they do not change repository state. Do not treat read-only professional/domain questions as TPM-owned conclusions; read-only/chat-only professional judgment routes to the matching role after binding.

Already-bound micro-loop caveat: use the canonical `Learning Intake / Loop Closeout` minimum record: question or observation, evidence path or command, answer or decision, and follow-up disposition. Do not emit another full bootstrap packet.

## Guardrails

Do not edit, answer substantively, or dispatch before binding task truth.

## Known Failure Modes

Reusing the main worktree; treating read-only as an exemption; leaving evidence outside the task issue.
