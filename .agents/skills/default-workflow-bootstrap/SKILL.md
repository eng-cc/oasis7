---
name: default-workflow-bootstrap
description: Use at the start of any new repository-changing task to ensure standard task worktree truth exists, then hand off to the correct repo-owned workflow surface.
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.


# Default Workflow Bootstrap

Use this skill as the repo-owned first-touch entrypoint for new work that will
change repository state.

It exists to make oasis7 behave more like a default structured workflow without
importing an external bootstrap or creating a second source of truth.

## When to Use

Use this skill when:

- a new task is starting and will change repository files, scripts, docs, tests, config, or other tracked state
- the user said `做`, `继续`, `landing`, or otherwise expects end-to-end execution
- you need to ensure a dedicated task worktree and `.pm` task exist before edits begin
- the next step is still “set up the correct workflow surface” rather than implementation itself

Do not use this skill when:

- the task is already inside a bound task worktree with `.pm` task truth and an obvious current phase
- you are already executing a documented task and only need `executing-project-tasks`
- you are already at claim-ready / closeout time
- the user only asks for chat-only explanation or read-only inspection that will not change repository state

## Core Workflow

1. Determine whether the request changes repository state:
   - repository-changing: requires standard worktree + `.pm` task truth before edits
   - read-only/chat-only: may be handled directly without repository writeback
2. For repository-changing work, verify workflow state in this order:
   - are you already in an isolated task worktree
   - is the current worktree already bound to the target task
   - is there unrelated dirty state that forbids reuse
3. If isolation or task truth is missing, create it:
   - choose `tpm` as the default owner role unless an existing bound task already has a valid owner
   - create a dedicated worktree unless the user explicitly authorized reuse
   - bootstrap `.pm` task inside the target worktree
   - read `doc/<module>/prd.md`, `doc/<module>/project.md`, and task execution truth
4. Once task truth exists, hand off to `repo-owned-workflow-router`.
5. Record the bootstrap decision in a formal sink:
   - `project.md`
   - handoff
   - `.pm/tasks/<TASK-UID>.execution.md`
6. Continue into the routed phase rather than stopping at setup.

## Output Contract

When this skill is used, the bootstrap decision should be renderable in this shape:

```markdown
WORKFLOW BOOTSTRAP DECIDED

## Repository State Impact
- Changes repository state:
- Why:

## Isolation Decision
- Current workspace state:
- Reuse allowed:
- Worktree action:

## Task Truth
- Owner role:
- `.pm` task:
- Formal docs:

## Routed Next Phase
- Selected workflow surface:
- Why now:

## Required Writeback
- `prd.md`:
- `project.md`:
- `.pm` execution log / handoff:

## Next Action
- exact next step:
```

## Oasis7-Specific Surfaces

- Worktree bootstrap:
  - `./scripts/new-task-worktree.sh`
- PM bootstrap / lifecycle:
  - `./scripts/pm/new-task.sh`
  - `./scripts/pm/workflow-report.sh`
  - `./scripts/pm/task-closeout.sh`
- Core workflow routing:
  - `./.agents/skills/repo-owned-workflow-router/SKILL.md`
- Formal truth:
  - `AGENTS.md`
  - `doc/<module>/prd.md`
  - `doc/<module>/project.md`
  - `.pm/tasks/<TASK-UID>.yaml`
  - `.pm/tasks/<TASK-UID>.execution.md`

## Guardrails

- Do not create a second planning or bootstrap truth outside repo-owned surfaces.
- Do not skip worktree / `.pm` task creation for repository-changing work unless the user explicitly authorized reuse of a specific task worktree.
- Do not stop after saying which workflow surface should be used; continue into that phase.
- Do not treat this skill as permission to bypass `repo-owned-workflow-router`, verification, or GitHub PR review.
- Do not force this bootstrap onto chat-only or read-only requests that do not change repository state.

## Verification

- Minimum verification commands after changing this surface:
  - `./scripts/pm/workflow-behavior-eval.sh`
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- Expected result:
  - bootstrap surface is discoverable in `AGENTS.md` and `.agents/skills/README.md`
  - workflow behavior eval proves repository-changing work routes through repo-owned task truth
