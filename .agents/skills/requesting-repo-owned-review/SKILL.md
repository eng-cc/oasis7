---
name: requesting-repo-owned-review
description: Use when a frozen implementation candidate needs the required independent repository-owned review.
---

# Requesting Repo-Owned Review

Canonical contract: [pre-PR review packet](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-review-packet), [handoff and v2 resolution](../../../doc/engineering/workflow/source-of-truth.md#review-resolution-handoff-promotion), [freeze](../../../doc/engineering/workflow/source-of-truth.md#freeze-gate), and [Pre-PR Ready](../../../doc/engineering/workflow/source-of-truth.md#pre-pr-ready-gate).

Use after implementation freeze and before Pre-PR Ready. Review is independent
evidence; it does not replace live CI, GitHub checks, holds, requested changes,
threads, or mergeability.

## When to Use

Use this skill for a frozen implementation candidate before Pre-PR Ready. It
selects and dispatches the required independent roles, validates their exact
returns, and uses the repository-owned closeout facade to produce the review
packet.

## Procedure

1. Freeze the candidate head and comparison identity. Run the smallest real
   boundary check that applies; this is early feedback, not final verification.
2. Classify the changed scope with `./scripts/pm/review-role-selector.py`:
   - mechanical documentation: repository health;
   - workflow, authorization, CI, or publication semantics: repository health
     and QA;
   - one domain's semantics: repository health and the matching domain role,
     adding QA when verification changes;
   - external messaging: repository health and LiveOps/community, adding QA
     when verification changes.
   Unknown or mixed impact keeps conservative roles. Preserve changed-path
   inference as the safety floor; explicit labels cannot reduce required roles.
3. Build one digest-bound impact projection and the default v2 source plan from
   the frozen write scope, consumed contracts, affected paths, tests, and
   required roles. Plan, selector, admission, CI, and closeout must bind the
   same projection. Unknown closure widens review; CI status cannot certify
   impact closure.
4. Generate one minimal packet for every required role and publish/read back
   the complete dispatch comment before dispatching any reviewer:

   ```sh
   ./scripts/pm/review-batch-epoch.py --root . dispatch --plan "$PLAN"
   ```

   Require a unique exact live marker, complete comment pagination, canonical
   task/PR identity, server readback, and current repository-admin permission.
   Before each dispatch, require `subagent-task-packet.py review-admission` to
   admit that exact packet. Return evidence must bind its admitted packet digest.
   Use minimal task packets by default; full history needs a recorded escalation
   reason.
5. Collect every planned structured return. Require scope/risk verdicts,
   findings triage and evidence basis, and residual risk. Blocking findings need
   a repair or evidence-backed rejection. Never invent a disposition or turn a
   blocker into `non_actionable`. Keep reviewer scopes cross-author: no reviewer
   may approve code they authored. If the existing one-slice-per-role plan
   cannot express independent coverage, stop and report the review blocker.
6. Run the closeout facade once the returns are complete:

   ```sh
   ./scripts/pm/review-closeout.sh --task-uid "$TASK_UID" \
     --review-plan "$PLAN" --role-returns "$ROLE_RETURNS" --complete
   ```

   The facade reuses or creates the exact plan-bound v2 handoff. When every
   return says `no_findings`, it creates the allowed empty v2 resolution. For
   finding-bearing returns, pass `--finding-resolution <manifest>` only after
   every finding has an explicit authorized disposition. The facade publishes
   or uniquely reuses the exact semantic resolution comment, reads it back,
   validates live identity/admin authority, then promotes, collects, and emits
   the canonical packet through existing v2 validators. An uncertain POST stays
   pending until exact live reconciliation; duplicate or conflicting markers
   block. Repeating a completed unchanged epoch is a no-op. A changed HEAD keeps
   old artifacts intact and needs a new applicable review epoch.
7. Before Pre-PR Ready or promotion, join the role-complete review identity with
   the applicable current PR CI or strict integration identity for the same
   candidate. Pending, stale, wrong-head/ref/app, failed, uncertain, or unreadable
   evidence blocks readiness.

## Guardrails

- Do not redispatch a complete unchanged epoch.
- Do not use a local fixture or chat-only result as live task evidence.
- Do not treat a review result as a substitute for final CI or GitHub checks.
- A legacy v1 plan is compatibility-only; new plan-owned preflight promotion
  uses the handoff/v2 and resolution/v2 contracts.

## Known Failure Modes

- A missing or ambiguous live dispatch comment, unreadable identity, or packet
  admission failure blocks dispatch; do not reconstruct or invent that evidence.
- A blocking finding without an authorized explicit disposition blocks
  completion; the facade does not choose findings outcomes.
- A changed head, applicability identity, or authority requires the applicable
  new review epoch; do not rewrite an earlier plan or return.
- An uncertain append remains pending until exact live reconciliation; do not
  blindly retry or report success from a local journal alone.

## Return Contract

- frozen comparison range and candidate head
- required roles, scoped independent returns, findings dispositions, and
  residual risks
- canonical packet evidence, or the blocker and supported resume step
