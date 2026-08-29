---
name: finishing-a-development-branch
description: Use when implementation is verified and the branch must proceed through review, PR, merge, and cleanup.
---

# Finishing a Development Branch

Canonical workflow: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done), [review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [terminal runbook](../../../doc/engineering/workflow/source-of-truth.md#terminal-runbook).

TPM is the coordinator/integrator for this sequence. Gate meanings, retry/disposition rules, merge authorization, and terminal order come only from the canonical links above.

## When to Use

Use after implementation and its required verification are complete, or when a
bound task has a classified non-merge outcome. In particular, `not_planned`
may enter from bootstrap, planning, or execution without implementation
verification and must proceed directly to the canonical non-merge terminal
route below.

## Freeze-Commit Gates

1. Freeze comparison ref and implementation head. Run `git diff --check <Comparison Ref>...<Source Head>`.
2. Create/resume the CI candidate with `./scripts/prepare-task-pr.sh --draft-candidate --create` and obtain a trusted `ci_ready_receipt` for its exact head.
3. Use `requesting-repo-owned-review`; resolve findings against that same head.

## Optional Evidence-Only Commit / PR-Prep Gates

4. If review/evidence helpers produce metadata after the frozen head, allow only an evidence-only commit; any implementation change invalidates the freeze and review.
   If that evidence-only commit changes HEAD, follow the canonical [PR creation
   gate](../../../doc/engineering/workflow/source-of-truth.md#pr-creation-gate):
   re-run final-head verification and review, then issue a new packet
   for the final PR head;
   otherwise do not create the PR. The resulting packet binds the reviewed PR
   head.
5. Record Pre-PR Ready with the adapter:

```bash
./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> \
  --comparison-ref "<Comparison Ref>" --verification-profile <repository-owned-profile> \
  --review-packet-file <canonical-review-packet.json> --ci-ready-receipt <receipt.json>
```

Partial remote state recovers via refresh -> audit -> retry; do not edit cache JSON.
6. Promote the existing draft only through:

```bash
./scripts/prepare-task-pr.sh --promote-draft <receipt.json>
```

Pre-PR local role review packet recorded after immutable verification and before PR creation; its schema is only at the canonical review-packet link.

## Post-PR / Pre-Merge Gates

7. Record the PR purpose decision after PR creation. Manual packaging/release CI may wait for an operator only when task policy says so.
8. Otherwise inspect the current PR gates with one batched read:

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

Post-PR checks/comments/mergeability remain separate gates. All interpretations, retry loops, dispositions and merge authorization come from the canonical gate definitions, not this skill.
9. Merge only with trusted gate evidence and the gate-selected repository path.
   A live `MERGEABLE` result with `REVIEW_REQUIRED` and approval-only `BLOCKED`
   or informational `BEHIND` defaults to admin merge
   when the gate emits `use_admin_merge: true`; do not request separate task or
   user authorization. Any hold or substantive gate blocker still fails closed.
   Do not land locally unless the user explicitly asks for local landing.

## Post-Merge Cleanup

10. From the canonical default worktree, use the terminal runbook's resumable
operator entry:

```bash
./scripts/pm/finalize-task.sh --repo-root <canonical-default-worktree> \
  --task-uid <TASK-UID> --pr <PR-NUMBER> --resume --json
```

The linked canonical terminal runbook remains authoritative for order,
receipts, recovery, and fail-closed behavior; this skill does not maintain a
second copy.

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

Missing trusted runtime attestation is `capability_blocked` for unattended
automation, not for the current human-operated PR path. Never manufacture a
passed packet or downgrade a real blocker to waiting.

## Guardrails

Do not bypass a canonical gate, mutate implementation after freeze without restarting review, or clean up before trusted merge evidence.

## Known Failure Modes

Stale verification; locally fabricated receipts; treating PR creation as completion; cleanup against unbound paths.
