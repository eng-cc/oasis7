---
name: tpm-production-supervisor
description: Use when assessing or developing the blocked production-supervisor target for a bound oasis7 task spanning multiple lifecycle phases.
---

# TPM Production Supervisor

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the
> single normative workflow spec.  GitHub task issue evidence comments remain
> the formal task-truth sink.

## When to Use

Use to evaluate or implement the production-supervisor target. It is not a
production-ready entrypoint: current capability status is `blocked`.

## Operational Contract

1. Treat `scripts/pm/tpm-workflow-supervisor.py` as an honest blocked prototype;
   never reinterpret reducer `action_required` or `external_wait` as completion.
2. Mechanical actions must come from `scripts/pm/tpm-action-builder.py` and be
   validated by `scripts/pm/tpm-live-validator-registry.py`.
3. Consume a `tpm-collaboration-action/v1` with the Codex collaboration tool.
   Return only `tpm-collaboration-return/v1` evidence bound to the dispatch ack,
   actual agent, attempt and artifact digest. Echoed plans are invalid.
4. Do not treat `scripts/pm/tpm-wake-owner.py` JSON mutation as delivery. A real
   runtime scheduler/thread-wakeup owner is still required.
5. Route watch outcomes as pending→watch, actionable→fix, ready→merge. A changed
   head invalidates review/gate evidence and loops through verify, review, push,
   and watch.

## Legitimate Stops

Only `completed` or a persisted `external_wait` for user/manual hold, actual
human approval, GitHub auth/transient exhaustion, or temporary readback failure
from an available trusted producer may yield control. When a trusted dispatch producer
exists but a temporary readback or delivery failure occurs,
the state is `external_wait`. The wake owner must remain installed
for waits.

## Guardrails

- Never import `scripts/pm/fixtures` from production adapters or validators.
- Never accept a caller-provided generic validator or self-signed receipt.
- A missing dispatch producer or attestation capability is
  `capability_blocked`; it is not a legitimate wait.
- TPM coordinates typed professional actions; the matching role owns the
  professional conclusion.
- A created PR is not done. Continue through merge receipt, task done, main sync
  and safe cleanup.

## Verification

Run `python3 scripts/pm/tpm-production-supervisor.test.py`, then the workflow
behavior evaluator, skill lint, doc governance, PM lint, and `git diff --check`.

## Known Failure Modes

- Calling the reducer directly and returning `action_required` to the user.
- Treating a collaboration plan or echoed payload as an agent return.
- Polling a PR linearly after a head-changing fix instead of invalidating the
  old epoch and looping through fresh verification and review.
- Letting a wait outlive its wake-owner lease or cancelling the owner before
  merged receipt, task done, main sync and safe cleanup.
