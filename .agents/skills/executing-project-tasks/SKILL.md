---
name: executing-project-tasks
description: Use when a task has written repo truth and implementation should proceed step by step with evidence.
---

# Executing Project Tasks

Canonical lifecycle: [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done).

## When to Use

Use when the task already has written scope in `prd.md`, `project.md`, a handoff, or GitHub-backed task truth. Do not use for unresolved direction, observed unexplained failures, or finishing.

## Procedure

1. Run a brief plan-gap review before editing: confirm acceptance criteria, dependencies, verification, and out-of-scope items. Do not create a second planning system outside `prd.md` / `project.md` / GitHub-backed task truth.
2. Record ordered steps and required professional slices in GitHub task issue evidence comments.
3. Implement one bounded step in its declared write scope.
4. Run the step-level verification and inspect the output.
5. Append result, evidence, deviation, and next step to the same task issue.
6. Repeat until scope is implemented and verified, then route to `finishing-a-development-branch`.

If any command, test, or behavior is unexpected, automatically route to `systematic-debugging`, resolve the root cause, and resume the same step. Pause only for canonical `external_wait` or `capability_blocked`, recording resume authority and instruction from the canonical state contract.

Use `./scripts/pm/append-execution-log.sh` when a durable local execution ledger is required. Module verification does not imply integration or release readiness.

## Return Contract

- completed step and changed paths
- fresh verification command/result
- GitHub task issue evidence link
- remaining steps, or canonical blocker with resume instruction

## Guardrails

Preserve declared write scopes and task truth; do not claim broader readiness than the evidence tier.

## Known Failure Modes

Large unverified batches; parallel planning truth; continuing after unexplained failures; treating module checks as release proof.
