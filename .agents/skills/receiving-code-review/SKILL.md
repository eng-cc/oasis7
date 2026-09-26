---
name: receiving-code-review
description: Use when a PR receives review comments or when a user asks to handle review feedback.
---

# Receiving Code Review

Follow the canonical [review feedback triage](../../../doc/engineering/workflow/source-of-truth.md#review-feedback-triage), [merge gates](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done) and [PM truth](../../../doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract).

## When to Use

Use this skill when:

- GitHub PR review comments arrive
- the user says to address review comments
- a comment looks unclear, debatable, or possibly stale
- a normal PR's required checks fail during the post-PR watch/fix/merge loop

## Core Workflow

1. Inventory the active comments. Before selecting repairs, include all comments currently available and record an evidence-backed disposition for each; comments arriving after triage require a fresh inventory.
2. Classify each one:
   - correctness bug
   - regression risk
   - missing test / evidence
   - style or preference
   - misunderstanding or stale assumption
   Each structured finding must also declare `triage.classification` (`blocking` or `nonblocking`) and a non-empty evidence basis; missing or unknown triage fails closed.
3. Verify the comment against repo truth before editing. Check it against the current diff, effective contract and actual consumers. Assess impact, confidence, regression risk, scope, benefit and verification cost; `P2` alone does not decide whether to change anything. For an incremental plan, use its bound role obligation: `full_review` assesses the delta, while `impact_confirmation` confirms no material impact within the recorded scope digest. Unknown impact, new required roles, or authority/policy drift escalates to full review.
4. For post-publication GitHub feedback, repair only credible P0 findings caused or exposed by the current PR diff. Treat lower-severity and out-of-scope comments as no-change dispositions with evidence; do not expand the task or create follow-up work without separate authorization. Project `Priority` is not review-finding severity. The stricter structured pre-PR role-finding contract remains unchanged. Batch only compatible accepted P0 repairs that share the current source scope and can use one focused verification and push/CI cycle.
5. Run focused checks for a repair batch; for a no-change decision, record the supporting evidence, rationale, residual risk, responsible role and revisit condition. A nonblocking `non_actionable` disposition must retain that rationale and revisit evidence. Do not create a task for every nit or start follow-up work without authorization. There is no arbitrary cycle cap. After a push changes the source head, prior review returns and plans are context only; current-head CI and the complete required role review must run again, with impacted roles in `full_review` and unaffected roles providing bound `impact_confirmation` unless escalation requires full review.
6. Push only when a code change is needed (including documentation edits); then resolve the thread after the relevant verification. For a stale or incorrect comment with no code change, record an evidence-backed disposition before resolving it. Use the same path for justified nonblocking no-change decisions. Formal role findings still require the canonical immutable return and authorized resolution manifest; a thread disposition does not replace them. This keeps thread resolution separate from merge readiness.
7. Re-check overall PR state separately.
8. For normal PRs, take one fresh batched gate read when otherwise ready to merge. Do not add a grace period, heartbeat, repeated poll, or merge delay solely for an absent GitHub Codex review. Process review material present in that read under the P0/current-change rule, while preserving required checks, mergeability, holds, and any administrative disposition/thread clearance required by live repository policy. `REVIEW_REQUIRED` is informational and does not block by itself. If everything passes, merge and clean up through the finishing branch workflow.

## Oasis7 GitHub Loop

Start with:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

Use it to inventory unresolved threads. After a code fix is pushed, or an evidence-backed no-change disposition is recorded, resolve the intended threads explicitly, then re-check:

- `reviewDecision`
- `mergeStateStatus`
- required checks

Treat `REVIEW_REQUIRED` as a status signal to report, not as merge-blocking by itself. The approval-only admin path needs no additional authorization; follow the fresh live gate in [the canonical policy](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done). Do not wait specifically for GitHub Codex review. Requested changes or unresolved threads may still require evidence-backed administrative clearance, but only current-change P0 findings mandate repair; failed checks, non-mergeable state, active holds, or non-review-approval merge API/branch-protection rejection remain blockers.

If the PR purpose decision is `manual_packaging_ci_hold`, do not convert packaging-job completion into merge readiness by itself. Resume the normal watch/fix/merge path only after the operator/user says the manual packaging CI purpose is complete.

## Response Rules

- Do not auto-agree with every comment.
- For an adopted comment, say what changed and what check passed.
- If the comment is partially valid, address the valid concern proportionately and explain the rest.
- For a stale/incorrect premise or a justified nonblocking no-change decision, explain the evidence and rationale. Follow canonical disposition authority; do not imply the suggestion was implemented or a real blocker was waived.

## Verification Rules

- Behavior fixes need a rerun of the affected check. No-change decisions need evidence sufficient to verify their premise and nonblocking conclusion; run a targeted check when needed to resolve uncertainty.
- Comments about docs still need `./scripts/doc-governance-check.sh`.
- Comments about PM flow still need `./scripts/pm/lint.sh`.

## Known Failure Modes

- Accepting every comment or `P2` label without assessing the current diff, contract, impact and cost.
- Misclassifying a current-change P0 as lower severity or out of scope to bypass repair.
- Resolving a thread before the targeted verification, required code push, or evidence-backed no-change disposition.
- Treating thread resolution as proof that the whole PR is merge-ready.
- Letting a review fix broaden into unrelated cleanup or silently revert sibling/user changes.

## Guardrails

- "Thread resolved" is not the same as "PR ready to merge".
- "Manual packaging CI ran" is not the same as "PR ready to merge".
- Do not widen scope just because the review mentions adjacent cleanup.
- Do not revert user or sibling-task changes unless explicitly requested.
