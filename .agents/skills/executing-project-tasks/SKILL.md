---
name: executing-project-tasks
description: Use when a task has written repo truth and implementation should proceed step by step with evidence.
---

# Executing Project Tasks

Canonical lifecycle: [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done), and [planning and slice contract](../../../doc/engineering/workflow/source-of-truth.md#52-tpm-planning-and-subagent-dispatch).

## When to Use

Use when the task already has written scope in a PRD/design, a handoff, or GitHub-backed task truth. Do not use for unresolved direction, observed unexplained failures, or finishing.

## Procedure

1. Confirm the bound task's goal, owner, write scope, exclusions, acceptance, and separately authorized dangerous effects. For a write task, satisfy the canonical per-step Plan-Gap Evidence schema and slice-context checklist in task truth before editing; the GitHub task Issue remains the sole mutable plan truth. Do not create a new Issue comment for each local command or intermediate step.
2. Work continuously within the same authorized scope: analyze, reproduce, write tests, implement, and run targeted verification. Normal commits, test repairs, diagnosis, and main advancement do not require a new task, epoch, or implementation permit. Stop before an out-of-scope write, new dangerous effect, changed delivery object, or explicit hold/expiry.
3. At task start and after any context handoff, query the compact resume state
   from the canonical worktree. Treat its `next_command` as the only suggested
   continuation and stop for its blockers when `identity_status` is not bound:

```bash
python3 ./scripts/pm/workflow-next.py --repo-root <canonical-worktree> \
  --task-uid <TASK-UID> --json
```

4. For a finite multi-obligation change, before implementing a leaf, read its coordinating Issue record and confirm the approved required set, mapping slots, candidate-selection rule and blocking feedback under the [traceability record contract](../../../doc/engineering/workflow/source-of-truth.md#traceability-record-contract). Ordinary single-task/single-leaf work follows its own task Issue and does not require a second coordinating Issue. The optional `delivery_obligations` binding may be absent; absence does not prove there are no obligations, but is not itself a blocker when the required record and evidence are complete.
5. Implement within the declared write scope and acceptance target.
6. Run the targeted verification and inspect the output.
   Route commands expected to emit broad logs or search results through `./scripts/pm/bounded-command-output.py`; inspect the bounded summary and retain the reported full artifact/digest for debugging.
7. Record material results, changed decisions, deviations, and blockers in the same task truth; do not create an Issue comment for each local command.
8. Continue until scope is implemented and verified, then route to `finishing-a-development-branch`. A leaf may complete truthfully while aggregate obligations remain pending.

If any command, test, or behavior is unexpected, automatically route to `systematic-debugging`. Diagnosis, isolated tests, and repair may continue only within the existing authorized change. Resume the same step only while the canonical state permits that typed action and its task/checkpoint-bound authority remains valid.

If the canonical state is `failed`, stop and escalate; recovery requires an authorized new evidence epoch or fresh bootstrap, not resumption. If a trusted helper reports `no_safe_repair` or `new_review_epoch_required`, preserve its reason and fail-closed immutable evidence: do not invent a CLI, overwrite or recreate immutable evidence, claim it was restored, expand scope, or resume the old epoch. Stop and escalate for `no_safe_repair`; begin a new epoch only after its authority is recorded. For `external_wait` or `capability_blocked`, record the canonical resume authority and instruction before continuing.

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

Repeated reauthorization for routine HEAD changes; per-command evidence churn; large unverified batches; parallel planning truth; continuing after unexplained failures; treating module checks as release proof.
