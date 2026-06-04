---
name: receiving-code-review
description: Use when a PR receives review comments or when a user asks to handle review feedback. Verifies each comment against code and repo truth, applies minimal valid fixes, and keeps thread resolution separate from merge readiness.
---

# Receiving Code Review

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
3. Verify the comment against repo truth before editing.
4. Apply the smallest fix that resolves the real issue.
5. Re-run the checks that prove the comment is addressed.
6. Push first, then resolve the thread explicitly.
7. Re-check overall PR state separately.
8. For normal PRs, continue watching required checks, requested changes, comments/threads, and mergeability after the fix; `REVIEW_REQUIRED` is informational and does not block by itself. If everything passes, merge and clean up through the finishing branch workflow.

## Oasis7 GitHub Loop

Start with:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

Use it to inventory unresolved threads. After fixes and push, resolve the intended threads explicitly, then re-check:

- `reviewDecision`
- `mergeStateStatus`
- required checks

Treat `REVIEW_REQUIRED` as a status signal to report, not as merge-blocking by itself. Requested changes, actionable comments, unresolved blocking threads, failed checks, non-mergeable state, or merge API/branch-protection rejection remain blockers.

If the PR purpose decision is `manual_packaging_ci_hold`, do not convert packaging-job completion into merge readiness by itself. Resume the normal watch/fix/merge path only after the operator/user says the manual packaging CI purpose is complete.

## Response Rules

- Do not auto-agree with every comment.
- If the comment is valid, say what changed and what check passed.
- If the comment is partially valid, fix the valid part and explain the rest.
- If the comment is stale or incorrect, answer with concrete code or doc evidence.

## Verification Rules

- Comments about behavior need a rerun of the affected check.
- Comments about docs still need `./scripts/doc-governance-check.sh`.
- Comments about PM flow still need `./scripts/pm/lint.sh`.

## Guardrails

- "Thread resolved" is not the same as "PR ready to merge".
- "Manual packaging CI ran" is not the same as "PR ready to merge".
- Do not widen scope just because the review mentions adjacent cleanup.
- Do not revert user or sibling-task changes unless explicitly requested.
