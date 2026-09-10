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

1. Inventory the active comments.
2. Classify each one:
   - correctness bug
   - regression risk
   - missing test / evidence
   - style or preference
   - misunderstanding or stale assumption
3. Verify the comment against the current diff, effective contract and actual consumers. Assess impact, confidence, regression risk, scope, benefit and verification cost; `P2` alone does not decide whether to change anything.
4. Fix confirmed correctness defects, material regressions and actual contract violations before merge. For incorrect/stale premises, preferences or nonblocking improvements, apply the canonical triage rule and choose a minimal fix or an evidence-backed no-change decision. Cost or scope cannot excuse a real blocker.
5. Run focused checks for a fix; for no change, record the supporting evidence, rationale and residual risk. Name a responsible role and revisit condition for material follow-up; do not create a task for every nit or start follow-up work without authorization.
6. Push only when a code change is needed (including documentation edits); then resolve the thread after the relevant verification. For a stale or incorrect comment with no code change, record an evidence-backed disposition before resolving it. Use the same path for justified nonblocking no-change decisions. Formal role findings still require the canonical immutable return and authorized resolution manifest; a thread disposition does not replace them.
7. Re-check overall PR state separately.
8. For normal PRs, continue watching required checks, requested changes, comments/threads, and mergeability after the fix; `REVIEW_REQUIRED` is informational and does not block by itself. If everything passes, merge and clean up through the finishing branch workflow.

## Oasis7 GitHub Loop

Start with:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

Use it to inventory unresolved threads. After a code fix is pushed, or an evidence-backed no-change disposition is recorded, resolve the intended threads explicitly, then re-check:

- `reviewDecision`
- `mergeStateStatus`
- required checks

Treat `REVIEW_REQUIRED` as a status signal to report, not as merge-blocking by itself. The approval-only admin path needs no additional authorization; follow the fresh live gate in [the canonical policy](../../../doc/engineering/workflow/source-of-truth.md#ready-and-done). Requested changes, actionable comments, unresolved blocking threads, failed checks, non-mergeable state, or non-review-approval merge API/branch-protection rejection remain blockers.

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
- Using low benefit, scope or `non_actionable` to hide a confirmed merge blocker.
- Resolving a thread before the targeted verification, required code push, or evidence-backed no-change disposition.
- Treating thread resolution as proof that the whole PR is merge-ready.
- Letting a review fix broaden into unrelated cleanup or silently revert sibling/user changes.

## Guardrails

- "Thread resolved" is not the same as "PR ready to merge".
- "Manual packaging CI ran" is not the same as "PR ready to merge".
- Do not widen scope just because the review mentions adjacent cleanup.
- Do not revert user or sibling-task changes unless explicitly requested.
