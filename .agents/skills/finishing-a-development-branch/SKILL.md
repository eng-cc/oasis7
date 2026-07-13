---
name: finishing-a-development-branch
description: Use when implementation is verified and the branch must proceed through review, PR, merge, and cleanup.
---

# Finishing a Development Branch

Canonical workflow: [capability](../../../doc/engineering/workflow/source-of-truth.md#capability-status), [ownership](../../../doc/engineering/workflow/source-of-truth.md#lifecycle-ownership), [state machine](../../../doc/engineering/workflow/source-of-truth.md#canonical-state-machine), [states](../../../doc/engineering/workflow/source-of-truth.md#workflow-states), [gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done), [review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [terminal runbook](../../../doc/engineering/workflow/source-of-truth.md#terminal-runbook).

TPM is the coordinator/integrator for this sequence. Gate meanings, retry/disposition rules, merge authorization, and terminal order come only from the canonical links above.

## When to Use

Use after implementation and its required verification are complete.

## Freeze-Commit Gates

1. Run fresh verification at the required tier and inspect all output.
2. Freeze comparison ref and implementation head. Run `git diff --check <Comparison Ref>...<Source Head>`.
3. Use `requesting-repo-owned-review`; resolve findings and obtain the canonical human-operated review packet.

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
  --review-packet-file <canonical-review-packet.json>
```

Partial remote state recovers via refresh -> audit -> retry; do not edit cache JSON.
6. Create the PR only through:

```bash
./scripts/prepare-task-pr.sh --create
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
9. Merge only with trusted gate evidence and the repository-approved merge path. Do not land locally unless the user explicitly asks for local landing.

## Post-Merge Cleanup

10. Follow the linked canonical terminal runbook; this skill does not maintain a
second copy of its order or commands.

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
