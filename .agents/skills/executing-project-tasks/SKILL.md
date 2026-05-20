---
name: executing-project-tasks
description: Use when a task already has repo truth such as `prd.md`, `project.md`, a handoff, or a `.pm` task, and the next job is to execute it step by step with plan-gap review, per-step verification, and explicit blocker handling.
---

# Executing Project Tasks

## When to Use

Use this skill when:

- the task already has written scope in `prd.md`, `project.md`, a handoff, or `.pm/tasks/<TASK-UID>.yaml`
- implementation should start now, but the execution path still needs a quick gap review
- the work is large enough that step-level verification matters before final closeout

Do not use this skill when:

- the task still lacks a real plan and first needs `prd.md` / `project.md` updates
- the user explicitly asked for brainstorming, architecture exploration, or plan-only output
- you are already in final closeout mode and should switch to `finishing-a-development-branch`

## Core Workflow

1. Read the canonical execution inputs:
   - `doc/<module>/prd.md`
   - `doc/<module>/project.md`
   - relevant handoff, if present
   - `.pm/tasks/<TASK-UID>.yaml`
   - `.pm/tasks/<TASK-UID>.execution.md`, if present
2. Run a brief plan-gap review before editing:
   - is the affected surface explicit enough to start
   - does each requirement or acceptance point map to a task step
   - does each meaningful step have a verification command or observable expected result
   - are names, PRD-IDs, task slug, and key paths consistent
3. If the gap review fails, update the existing repo truth first instead of inventing a side plan.
4. Execute one atomic step at a time.
5. After each meaningful step, run the named verification for that step and inspect the result directly.
6. Record important blockers, behavior changes, and verification outcomes in the task execution log or formal docs when they affect task truth.
7. When scope is implemented and verified, switch to `finishing-a-development-branch` for closeout, commit, PR prep, review handling, and cleanup.

## Oasis7-Specific Surfaces

- Planning / execution truth:
  - `AGENTS.md`
  - `doc/<module>/prd.md`
  - `doc/<module>/project.md`
  - `.pm/tasks/<TASK-UID>.yaml`
  - `.pm/tasks/<TASK-UID>.execution.md`
- Planning helpers:
  - `./.agents/roles/templates/handoff-brief.md`
  - `./.agents/roles/templates/handoff-detailed.md`
  - `./.agents/roles/templates/planning-self-checklist.md`
- Verification / closure helpers:
  - `./scripts/pm/claim-ready.sh`
  - `./scripts/pm/task-closeout.sh`
  - `./scripts/prepare-task-pr.sh`

## Blocker Rules

Stop and report the blocker instead of guessing when:

- the written plan is missing required affected paths, verification, or acceptance mapping
- the task has drifted beyond the documented scope
- instructions conflict across `prd.md`, `project.md`, handoff, or `.pm` truth
- the same verification keeps failing and the failure is no longer producing new information

When blocked, lead with:

- what step you were executing
- what verification ran
- what failed or was unclear
- what doc or decision needs to change before continuing

## Guardrails

- Do not create a second planning system outside `prd.md` / `project.md` / `.pm`.
- Do not batch many unverified steps and only test at the very end if step-level verification was available.
- Do not continue through ambiguity by silently choosing one interpretation.
- Do not treat this skill as a replacement for final closeout; use `finishing-a-development-branch` when implementation is done.

## Verification

- Minimum repo-owned checks before calling the execution surface updated:
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- Also run at least one representative step-level verification tied to the task you executed.
