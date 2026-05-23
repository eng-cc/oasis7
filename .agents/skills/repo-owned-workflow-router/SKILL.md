---
name: repo-owned-workflow-router
description: Use at the start of any non-trivial task, or whenever you need to decide which repo-owned workflow skill applies next. Routes the task through bounded brainstorming, behavior-first TDD, execution, verification, and closeout without replacing oasis7 root truth.
---

# Repo-Owned Workflow Router

Use this skill to decide which repo-owned workflow surface should drive the next phase of work.

## When to Use

Use this skill when:

- a new non-trivial task is starting
- you are unsure which local workflow skill should apply next
- the task needs to move across multiple phases, such as ideation -> implementation -> verification -> closeout
- the user wants the whole workflow chained together rather than treated as isolated skills

Do not use this skill when:

- the task is trivial and can be completed directly without workflow branching
- you are already clearly inside one terminal phase and the next step is obvious

## Required Rules

1. This skill is a router, not an external bootstrap.
2. It must not replace `AGENTS.md`, `.pm`, `prd.md`, `project.md`, task execution logs, or GitHub PR review.
3. It only chooses and orders repo-owned workflow skills.
4. If the task truth changes, route decisions must be written back into formal docs or the execution log.
5. Use the narrowest applicable workflow surface; do not force every phase if it is not needed.
6. If the route implies multi-role or subagent-driven execution, the route output must also include a minimal slice contract: role, slice type, write scope, return contract, formal sink, and integration owner/order.

## Routing Order

Check the task in this order:

1. `bounded-brainstorming`
   - Use when direction is still fuzzy, scope is too large, or the problem is inherently option-heavy or visual.
2. `tdd-test-writer`
   - Use when the task changes automatable behavior and has a stable automated test surface.
3. `executing-project-tasks`
   - Use when `prd.md` / `project.md` / handoff / `.pm` truth is ready and implementation should proceed step by step.
4. `verification-before-completion`
   - Use when the work is close to a claim such as “done”, “tests pass”, or “ready for PR”.
5. `finishing-a-development-branch`
   - Use when implementation and required verification are complete and the task should close out, commit, and move into PR handling.

## Routing Questions

Ask and answer these in order:

1. Is the direction already clear enough to implement?
2. Does the task need scope decomposition or 2-3 option comparison first?
3. Will the task change product/runtime/interaction behavior with a stable test surface?
4. Is the task already backed by sufficient repo truth to execute?
5. Is the next risk “implementation correctness” or “claim correctness”?
6. Is the task actually at closeout rather than execution?

## Expected Output

```markdown
WORKFLOW ROUTE DECIDED

## Task Phase
- Current phase:
- Why:

## Selected Workflow Skills
1. [skill name] - [why now]
2. [skill name] - [why next]
3. [skill name] - [if needed]

## Skipped Workflow Skills
- [skill name] - [reason skipped]

## Required Writeback
- `prd.md`:
- `project.md`:
- handoff / `.pm` execution log:

## Subagent Slice Plan (If Needed)
- role:
- slice type:
- write scope:
- return contract:
- formal sink / writeback surface:
- integration owner:
- integration order:

## Next Action
- exact next step:
```

## Guardrails

- Do not route into `bounded-brainstorming` if the task is already implementation-ready.
- Do not route into `tdd-test-writer` for pure docs/governance work or when no stable harness exists.
- Do not route into `executing-project-tasks` if the plan truth is still missing key scope or validation details.
- Do not skip `verification-before-completion` when you are about to make a completion claim.
- Do not use this router as a replacement for closeout; switch to `finishing-a-development-branch` when the task is done.
