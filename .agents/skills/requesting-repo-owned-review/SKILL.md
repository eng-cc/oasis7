---
name: requesting-repo-owned-review
description: Use when a diff has high claim risk, closes a major slice, or needs a focused repo-owned review before commit or before asking GitHub reviewers to assess it.
---

# Requesting Repo-Owned Review

Use this skill when the work would benefit from one more deliberate review pass before it enters or advances through the GitHub PR path.

## When to Use

Use this skill when:

- a major feature or workflow helper just landed locally
- multiple role slices were just integrated back into one canonical diff
- the next claim is high-risk, such as `tests_passed`, `ready_for_pr`, or a broad behavioral assertion
- you want a focused local review packet before asking GitHub reviewers to spend time on the PR

Do not use this skill when:

- the diff is trivial and already fully covered by the normal verification path
- you are trying to replace GitHub PR review with an internal review ritual
- no concrete review target, risk question, or evidence sink has been defined

## Core Rule

Repo-owned review is a supplement, not a replacement.

It strengthens local confidence and leaves a traceable packet, but the formal review boundary remains:

`prepare-task-pr -> GitHub required checks -> review/approval`

## Workflow

1. Define the review trigger:
   - major feature
   - high-risk integration slice
   - commit-before-claim risk
   - pre-PR packaging of a complex diff
2. Freeze the review target:
   - changed files or path set
   - exact question to answer
   - evidence already available
3. State the expected output contract:
   - `findings`
   - `no_findings`
   - `residual_risk`
4. Write the review request into a formal sink before or while dispatching:
   - `.pm/tasks/<TASK-UID>.execution.md`
   - PR evidence document
   - handoff when another role/subagent is reviewing
5. Run or dispatch the review.
6. Act on the result:
   - fix valid findings
   - record rejected/stale findings with code or doc evidence
   - keep residual risk explicit
7. Only then continue to claim-ready, commit, or PR progression.

## Review Packet Template

```markdown
## YYYY-MM-DD HH:MM:SS CST / <role_name>
- Review Trigger: <major feature | high-risk slice | commit claim risk | pre-PR>
- Review Scope: <paths / diff summary>
- Review Question: <what must this review confirm or challenge>
- Evidence Available: <tests / docs / screenshots / logs>
- Expected Return Contract: <findings | no_findings | residual_risk>
- Formal Sink: <execution log | PR evidence | handoff>
```

## Output Rules

- Findings must be categorized by severity or merge risk.
- `no_findings` still needs a short residual-risk statement when risk is not literally zero.
- If the review is stale or wrong, answer with concrete repo truth instead of silently ignoring it.
- Always separate:
  - local repo-owned review outcome
  - GitHub PR review readiness

## Guardrails

- Do not turn this into a mandatory review after every tiny step.
- Do not claim that repo-owned review makes GitHub review unnecessary.
- Do not leave the review request or outcome as chat-only context.
- Do not resolve GitHub threads based solely on this local review packet.
