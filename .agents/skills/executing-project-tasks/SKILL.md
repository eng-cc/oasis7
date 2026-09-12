---
name: executing-project-tasks
description: Use when a task has written repo truth and implementation should proceed step by step with evidence.
---

# Executing Project Tasks

Canonical lifecycle: [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done).

## When to Use

Use when the task already has written scope in a PRD/design, a handoff, or GitHub-backed task truth. Do not use for unresolved direction, observed unexplained failures, or finishing.

## Procedure

1. Before editing or execution, create a `Plan-Gap Evidence` entry for every ordered execution step. Each entry must record all of: `step_id`, `acceptance_refs`, `dependencies`, `verification_command`, `verification_evidence`, `write_scope`, `out_of_scope`, and `required_role_slices`. Keep each field non-empty and map acceptance/dependencies/scope to current GitHub-backed task truth; state the verification command and expected evidence target before execution, capture the actual result afterward, and name required roles or an explicit reasoned exemption. A missing field, evidence, or mapping fails closed before editing or execution. Keep mutable task planning only in GitHub-backed task truth; repository PRD/design documents remain durable professional authority, not a second task ledger.
2. Record ordered steps and required professional slices in GitHub task issue evidence comments.
3. At task start and after any context handoff, query the compact resume state
   from the canonical worktree. Treat its `next_command` as the only suggested
   continuation and stop for its blockers when `identity_status` is not bound:

```bash
python3 ./scripts/pm/workflow-next.py --repo-root <canonical-worktree> \
  --task-uid <TASK-UID> --json
```

4. For a finite multi-obligation change, before implementing a leaf, read its coordinating Issue record and confirm the approved required set, mapping slots, candidate-selection rule and blocking feedback under the [traceability record contract](../../../doc/engineering/workflow/source-of-truth.md#traceability-record-contract). Ordinary single-task/single-leaf work follows its own task Issue and does not require a second coordinating Issue. The optional `delivery_obligations` binding may be absent; absence does not prove there are no obligations, but is not itself a blocker when the required record and evidence are complete.
5. Implement one bounded step in its declared write scope.
6. Run the step-level verification and inspect the output.
   Route commands expected to emit broad logs or search results through `./scripts/pm/bounded-command-output.py`; inspect the bounded summary and retain the reported full artifact/digest for debugging.
7. Append result, evidence, deviation, and next step to the same task issue.
8. Repeat until scope is implemented and verified, then route to `finishing-a-development-branch`. A leaf may complete truthfully while aggregate obligations remain pending.

If any command, test, or behavior is unexpected, automatically route to `systematic-debugging`, resolve the root cause, and resume the same step. Pause only for canonical `external_wait` or `capability_blocked`, recording resume authority and instruction from the canonical state contract.

Use `./scripts/pm/append-execution-log.sh` when a durable local execution ledger is required. Module verification does not imply integration or release readiness.

## Return Contract

- completed step and changed paths
- fresh verification command/result
- GitHub task issue evidence link
- remaining steps, or canonical blocker with resume instruction

## Guardrails

For an explicitly bound manual loop task, use `scripts/pm/loop.py` with the effective trusted tool root before admission or continuation. Preserve the exact loop binding, immutable contracts, scope and user merge hold. At stable waits return resumable evidence without heartbeat or scheduled continuation; completion never starts another task. See [manual entry authority](../../../doc/engineering/workflow/source-of-truth.md#manual-three-loop-transition). Legacy tasks retain their existing route.

For a finite multi-obligation binding, the coordinating record is read and
validated by the effective traceability helper before leaf admission. Keep the
leaf closeout truthful to its own evidence; an aggregate candidate and its
coordinating record are required before an aggregate completion claim.

Preserve declared write scopes and task truth; do not claim broader readiness than the evidence tier.

## Known Failure Modes

Large unverified batches; parallel planning truth; continuing after unexplained failures; treating module checks as release proof.
