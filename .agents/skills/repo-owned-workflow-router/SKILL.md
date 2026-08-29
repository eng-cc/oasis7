---
name: repo-owned-workflow-router
description: Use when a bound oasis7 task needs the next repo-owned workflow phase selected.
---

# Repo-Owned Workflow Router

Canonical authority and lifecycle: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done).

The production supervisor is a target runtime executor and is currently `capability_blocked`; TPM remains coordinator/integrator.

## When to Use

Use after bootstrap has bound canonical task truth and the next workflow phase is not yet selected.

## Routing

Read-only/chat-only requests enter this router after `default-workflow-bootstrap` has established task truth.

1. Clear execution truth -> `executing-project-tasks`.
2. Ambiguous, option-heavy, or materially visual scope -> optional `bounded-brainstorming`; do not route there when implementation-ready.
3. Behavior change with a stable narrow harness -> `tdd-test-writer` before execution.
4. Observed failure -> `systematic-debugging` before speculative fixes.
5. Review feedback -> `receiving-code-review`.
6. A classified non-merge outcome, including `not_planned` during bootstrap,
   planning, or execution -> `finishing-a-development-branch` and its canonical
   non-merge terminal route; implementation verification is not a prerequisite.
7. Implementation verified -> `finishing-a-development-branch`.
8. Already-bound read-only professional/domain judgment -> matching bounded role slice. Pure fact lookup needs no professional slice.

Do not treat specialist domain skills as mandatory default workflow phases. Select them only when their trigger matches.

## Route Output

- selected phase/skill and reason
- skipped optional phases and reason
- verification tier
- GitHub issue evidence comment (mandatory)

## Subagent Slice Plan (If Needed)

- role:
- slice type:
- model configuration: `inherit current parent selection` by default; record observed runtime or `adapter inactive on this surface`
- context delivery mode: minimal HEAD-bound task packet by default; record a concrete escalation reason before using full history
- task packet identity: task UID, canonical worktree, base ref, current/frozen HEAD, producer/time
- mandatory context checklist:
  - identity and authority:
  - workflow governance:
  - task truth:
  - user intent:
  - scoped repo context:
  - collaboration boundary:
- write scope:
- return contract:
- formal sink / writeback surface: GitHub issue evidence comment (mandatory)
- integration owner: TPM
- integration order:
- context exemption: none, or reason no professional slice is required

Record the slice contract in the GitHub issue evidence comment sink before dispatch. Unbound read-only professional questions are invalid under the always-bootstrap workflow; read-only professional/domain judgments must already be task-bound.

Do not dispatch implementation, verification, review, or specialist subagents without `AGENTS.md`, the assigned role card, workflow source-of-truth, current GitHub-backed task truth, and scoped repo context recorded in the mandatory context checklist.

## Guardrails

Do not route around canonical task truth or invent a specialist phase.

## Known Failure Modes

Routing before bootstrap; mandatory brainstorming; speculative fixing before debugging; incomplete slice contracts.
