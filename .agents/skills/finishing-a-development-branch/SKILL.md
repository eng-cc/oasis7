---
name: finishing-a-development-branch
description: Use when implementation is verified and the branch must proceed through review, PR, merge, and cleanup.
---

# Finishing a Development Branch

Canonical workflow: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done), [review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [terminal runbook](../../../doc/engineering/workflow/source-of-truth.md#terminal-runbook).

TPM is the coordinator/integrator for this sequence. Gate meanings, retry/disposition rules, merge authorization, and terminal order come only from the canonical links above.

## When to Use

Use after verified implementation or a classified non-merge outcome; `not_planned` may enter this route from bootstrap, planning, or execution without implementation
verification.

Read the bound task's compact resume state from the canonical worktree; follow only `next_command`, and stop on unbound identity or any blocker:

```bash
python3 ./scripts/pm/workflow-next.py --repo-root <canonical-worktree> --task-uid <TASK-UID> --json
```

For a finite multi-obligation change, before a leaf enters finish, read its coordinating Issue record and confirm the approved required set, mapping slots, candidate-selection rule and blocking feedback under the [traceability record contract](../../../doc/engineering/workflow/source-of-truth.md#traceability-record-contract). Ordinary single-task/single-leaf work follows its own task Issue and does not require a second coordinating Issue. Before claiming aggregate or overall completion, compare that frozen record with actual Task/contract/evidence references, aggregate source/integration/tested-tree candidate, results and feedback clearance. An omitted optional `delivery_obligations` binding is not itself a blocker when the required record and evidence are complete; only missing proof for a declared obligation keeps the aggregate claim pending. That does not invalidate a bounded truthful leaf completion or create a new state or ledger. Preserve classified non-merge outcomes and report the aggregate blocker with its resume condition.

## Freeze-Commit Gates

1. Freeze comparison ref and implementation head. Run `git diff --check <Comparison Ref>...<Source Head>`.
2. Create/resume the CI candidate with `./scripts/prepare-task-pr.sh --draft-candidate --create`; the helper records and reads back canonical task/head/base identity on the bound issue before push or PR creation.
3. After the draft's exact-head `required-gate` succeeds, produce the trusted receipt explicitly:

```bash
python3 ./scripts/pm/ci-ready-receipt.py \
  --repository <owner/repo> --task-uid <TASK-UID> \
  --task-issue-number <issue-number> --pr-number <pr-number> \
  --check-name required-gate --check-app-id <required-check-app-id> \
  --planner-digest auto --json > <ci_ready_receipt.json>
```

Resolve `<required-check-app-id>` from the active repository rules returned by `pr-lifecycle-gate.py`; do not infer it from whichever same-name check finished most recently. The producer validates the live PR, exact HEAD, ruleset-bound required check, base, and planner artifact before emitting the receipt. `prepare-task-pr.sh --draft-candidate --create` does not create this artifact.
4. Use `requesting-repo-owned-review`; resolve findings against that same head.

## Optional Evidence-Only Commit / PR-Prep Gates

5. Allow only evidence-only commits after freeze; implementation changes invalidate freeze/review.
   If one changes HEAD, follow the canonical [PR creation gate](../../../doc/engineering/workflow/source-of-truth.md#pr-creation-gate),
   repeat final-head verification/review, issue a new packet; otherwise do not create the PR. The packet binds the reviewed PR head.
6. Record Pre-PR Ready with the adapter:

```bash
./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> \
  --comparison-ref "<Comparison Ref>" --verification-profile <repository-owned-profile> \
  --review-packet-file <canonical-review-packet.json> --ci-ready-receipt <receipt.json>
```

Partial remote state recovers via refresh -> audit -> retry; do not edit cache JSON.
7. Promote the existing draft only through:

```bash
./scripts/prepare-task-pr.sh --promote-draft <ci_ready_receipt.json>
```

Pre-PR local role review packet recorded after immutable verification, after frozen-head draft-candidate creation and trusted exact-head CI, before draft promotion; its schema is only at the canonical review-packet link.

## Post-PR / Pre-Merge Gates

8. Record the PR purpose decision after PR creation. Manual packaging/release CI may wait for an operator only when task policy says so.
9. Otherwise inspect the current PR gates with one batched read:

```bash
./scripts/pm/pr-lifecycle-gate.py <pr-number> --task-uid <task_uid> --json
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

For a stable long-running required check or `required-gate` wait, follow the
[canonical stable-wait rule](../../../doc/engineering/workflow/source-of-truth.md#stable-required-gate-wait).

On a non-Codex surface, use the finite fallback:

```bash
./scripts/pm/pr-watch-loop.sh <pr-number> --task-uid <task_uid>
```

Post-PR checks/comments/mergeability remain separate gates. All interpretations, retry loops, dispositions and merge authorization come from canonical gate definitions, not this skill.
9. Before merge, follow the canonical [terminal-readiness preflight](../../../doc/engineering/workflow/source-of-truth.md#terminal-readiness-preflight) from the canonical default worktree. It must return `status: ready`, an empty `blockers` array, and the exact executable `next_command`; any identity mismatch must be repaired and reverified before merge:

```bash
./scripts/pm/finalize-task.sh --repo-root <canonical-default-worktree> --task-uid <TASK-UID> --pr <PR-NUMBER> --preflight --json
```

10. Merge only with trusted gate evidence and the gate-selected repository path.
   A live `MERGEABLE` result with `REVIEW_REQUIRED` and approval-only `BLOCKED`
   or informational `BEHIND` defaults to admin merge
   when the gate emits `use_admin_merge: true`; do not request separate task or
   user authorization. Any hold or substantive gate blocker still fails closed.
   Do not land locally unless the user explicitly asks for local landing.

## Post-Merge Cleanup

11. From the canonical default worktree, use the terminal runbook's resumable operator entry:

```bash
./scripts/pm/finalize-task.sh --repo-root <canonical-default-worktree> \
  --task-uid <TASK-UID> --pr <PR-NUMBER> --resume --json
```

The canonical terminal runbook controls order, receipts, recovery and fail-closed behavior.

For a classified non-merge outcome, follow the [canonical terminal runbook](../../../doc/engineering/workflow/source-of-truth.md#terminal-runbook):

```bash
python3 ./scripts/pm/non-merge-finalize.py \
  --repo-root <canonical-default-worktree> --task-uid <TASK-UID> \
  --reason <reason> --evidence-file <path> --json
```

## Return Contract

- frozen comparison range and fresh verification
- canonical review and gate evidence links
- PR URL and merged receipt, or canonical blocker with resume instruction
- main-sync and cleanup result

Missing trusted runtime attestation is `capability_blocked` for unattended automation, not human-operated PRs. Never manufacture passed evidence or downgrade a blocker.
## Guardrails

For an explicitly bound manual loop task, use `scripts/pm/loop.py` with the effective trusted tool root before admission or continuation. Preserve the exact loop binding, immutable contracts, scope and user merge hold. At stable waits return resumable evidence without heartbeat or scheduled continuation; completion never starts another task. See [manual entry authority](../../../doc/engineering/workflow/source-of-truth.md#manual-three-loop-transition). Legacy tasks retain their existing route.
Do not bypass a canonical gate, mutate implementation after freeze without restarting review, or clean up before trusted merge evidence. Hosted loop CI proves repository scope/content only; promotion and merge require fresh effective-helper admission using the existing local `gh` login, without exporting credentials or accepting caller-signed substitute receipts.
## Known Failure Modes

Stale verification; locally fabricated receipts; treating PR creation as completion; cleanup against unbound paths.
