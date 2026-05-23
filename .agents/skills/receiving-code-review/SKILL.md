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

## Oasis7 GitHub Loop

Start with:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

Use it to inventory unresolved threads. After fixes and push, resolve the intended threads explicitly, then re-check:

- `reviewDecision`
- `mergeStateStatus`
- required checks

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
- Do not widen scope just because the review mentions adjacent cleanup.
- Do not revert user or sibling-task changes unless explicitly requested.
