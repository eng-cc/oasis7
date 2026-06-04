---
name: requesting-repo-owned-review
description: Use when a branch is about to create a PR and must collect fresh involved-role subagent review before GitHub PR creation.
---

# Requesting Repo-Owned Review

Use this skill when the work is about to enter the GitHub PR path.

The review is no longer optional before PR creation. TPM must create or dispatch
fresh local subagents for every involved relevant professional role, integrate
their review findings, and record the required evidence packet before
`prepare-task-pr.sh --create`.

## When to Use

Use this skill when:

- a branch is about to create a PR
- a major feature or workflow helper just landed locally
- multiple role slices were just integrated back into one canonical diff
- the next claim is `ready_for_pr`, `tests_passed`, or a broad behavioral assertion

Do not use this skill when:

- there is no PR creation path in the current task
- you are trying to replace GitHub PR required checks, requested-changes handling, comment closeout, or mergeability with only an internal review ritual
- no concrete review target, risk question, or evidence sink has been defined

## Core Rule

Pre-PR local role review is required before PR creation, but it is not a
replacement for GitHub required checks, requested-changes handling, comment
closeout, mergeability, or the repository/GitHub merge path. `REVIEW_REQUIRED`
is informational and is not a blocking item by itself. A review-approval-only
`mergeStateStatus=BLOCKED` may use an explicitly authorized repository admin
merge path, but this does not weaken checks, requested changes, comments, or
mergeability gates.

The formal path is:

`local involved-role subagent review -> prepare-task-pr -> GitHub required checks -> comment/requested-changes closeout -> mergeability -> merge`

## Workflow

1. Define the involved roles:
   - infer from changed paths, role ownership, task slice history, and user-facing claim
   - include `qa_engineer` when the PR claim depends on verification or release readiness
   - include `liveops_community` when external messaging, incidents, player promises, or channel runbooks are touched
2. Freeze the review target:
   - changed files or path set
   - exact question to answer
   - evidence already available
3. Spawn or dispatch a fresh subagent for each involved role.
4. State the expected output contract:
   - `findings`
   - `no_findings`
   - `residual_risk`
5. Write the review request into a formal sink before or while dispatching:
   - `.pm/tasks/<TASK-UID>.execution.md`
   - PR evidence document
   - handoff when another role/subagent is reviewing
6. Act on the result:
   - fix valid findings
   - record rejected/stale findings with code or doc evidence
   - keep residual risk explicit
7. Record the passed evidence packet only after all valid findings are resolved.
8. Only then continue to PR creation.

## Review Packet Template

```markdown
## YYYY-MM-DD HH:MM:SS CST / <role_name>
- Review Trigger: pre-PR local role review
- Review Scope: <paths / diff summary>
- Review Roles: <comma-separated roles>
- Review Question: <what must this review confirm or challenge>
- Evidence Available: <tests / docs / screenshots / logs>
- Expected Return Contract: <findings | no_findings | residual_risk>
- Formal Sink: <execution log | PR evidence | handoff>
```

## Passed Evidence Packet

Record this packet in `.pm/tasks/<TASK-UID>.execution.md` after integrating the
role reviews and addressing findings:

```markdown
- Pre-PR Local Role Review: passed
- Task UID: <task_uid>
- Source Worktree: <absolute path>
- Source Branch: <branch>
- Source Head: <reviewed git sha; must be current source head or an ancestor whose later changes are only the task review evidence files>
- Comparison Ref: <base ref>
- Reviewed Changed Paths: <semicolon-separated paths or diff summary ref>
- Role Selection Basis: <changed paths + task slice history + explicit includes/skips>
- Review Roles: <comma-separated roles>
- Review Evidence: <per-role section or handoff refs>
- Review Findings Disposition: <addressed | no_findings>
- Finding Disposition Evidence: <fix refs or rejected/stale evidence refs>
- Residual Risk: <text>
```

## Output Rules

- Findings must be categorized by severity or merge risk.
- `no_findings` still needs a short residual-risk statement when risk is not literally zero.
- If the review is stale or wrong, answer with concrete repo truth instead of silently ignoring it.
- Always separate:
  - local repo-owned review outcome
  - GitHub PR review readiness

## Guardrails

- Do not leave PR creation without a passed pre-PR local role review packet.
- Do not claim that repo-owned review makes GitHub review unnecessary.
- Do not leave the review request or outcome as chat-only context.
- Do not resolve GitHub threads based solely on this local review packet.
