---
name: default-workflow-bootstrap
description: Use when any oasis7 user request starts and needs standard task worktree, GitHub Project-backed task truth, owner role truth, and routing into the repo-owned workflow surface.
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.

> PM truth: GitHub Issues/Project plus `.pm/github-project-sync/tasks.json` are active task truth; task evidence is recorded as GitHub issue comments.


# Default Workflow Bootstrap

Use this skill as the repo-owned first-touch entrypoint for every user request.

It exists to make oasis7 behave more like a default structured workflow without
importing an external bootstrap or creating a second source of truth.

## When to Use

Use this skill when:

- any new user request starts, including read-only, chat-only, pure fact lookup, professional judgment, implementation, verification, review, or external messaging
- a new task is starting and may change repository files, scripts, docs, tests, config, or other tracked state
- the user said `做`, `继续`, `landing`, or otherwise expects end-to-end execution
- you need to ensure a dedicated task worktree and GitHub Project-backed task truth exist before edits begin
- the next step is still “set up the correct workflow surface” rather than implementation itself

Do not use this skill when:

- the task is already inside a bound task worktree with GitHub Project-backed task truth and an obvious current phase
- you are already executing a documented task and only need `executing-project-tasks`
- you are already at claim-ready / closeout time

Read-only caveat:

- Read-only and chat-only requests do not skip this bootstrap. They still need
  task/worktree truth before TPM answers, gathers evidence, or dispatches slices.
- If a read-only question requires product/design/gameplay/game-visual-interaction/runtime/blockchain-ops/WASM/agent/viewer/QA/repository-health
  or liveops judgment, route to the matching bounded professional slice after
  task/worktree bootstrap.
- Pure fact lookup, path lookup, command-output restatement, or mechanical
  evidence collection can be answered directly by TPM only inside the bound
  task worktree and only if it is not framed as a professional conclusion.

## Core Workflow

1. Treat the request as requiring standard worktree + GitHub Project-backed task truth before substantive handling. Do not first classify the request type to decide whether bootstrap is needed:
   - repository-changing: requires standard worktree + GitHub Project-backed task truth before edits
   - read-only/chat-only pure fact lookup: requires standard worktree + GitHub Project-backed task truth before direct answer
   - read-only/chat-only professional judgment: requires standard worktree + GitHub Project-backed task truth before dispatching the matching professional role slice
2. Verify workflow state in this order:
   - are you already in an isolated task worktree
   - is the current worktree already bound to the target task
   - is there unrelated dirty state that forbids reuse
3. If isolation or task truth is missing, create it:
   - choose `tpm` as the default workflow owner role unless an existing bound task already has a valid owner
   - treat `tpm` ownership as workflow coordination only; professional work still requires matching bounded subagent slices
   - create a dedicated worktree unless the user explicitly authorized reuse
   - bootstrap GitHub Project-backed task truth inside the target worktree
   - read `doc/<module>/prd.md`, `doc/<module>/project.md`, and task execution truth
4. Once task truth exists, hand off to `repo-owned-workflow-router`.
5. Record the bootstrap decision in the task GitHub issue evidence comments.
   - `project.md` and handoff may supplement the task issue
   - they cannot replace the GitHub-backed task evidence sink for task truth
6. Continue into the routed phase rather than stopping at setup.

Already-bound micro-loop caveat:

- The first bootstrap for a request still records the full bootstrap decision.
- Once the request is already inside the bound task/worktree, small objective
  fact lookups, command-output restatements, and tiny follow-ups should use the
  source-of-truth "Learning Intake / Loop Closeout" minimal record instead of
  repeating the full bootstrap packet.
- The minimal record is: question or observation, evidence path or command,
  answer or decision, and whether task truth changed.
- If owner, scope, route, professional slice plan, or PR chain changes, return
  to the full bootstrap/router records.
- Same-thread continuations inside the same bound task should verify the
  binding and record only changed route/evidence instead of repeating the
  heavyweight bootstrap.

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
- GitHub Project task:
- Formal docs:

## Routed Next Phase
- Selected workflow surface:
- Why now:

## Required Writeback
- `prd.md`:
- `project.md`:
- GitHub issue evidence comment (mandatory):
- handoff (optional supplement):

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
  - `.pm/github-project-sync/tasks.json`
  - GitHub task issue evidence comments

## Guardrails

- Do not create a second planning or bootstrap truth outside repo-owned surfaces.
- Do not infer a read-only/chat-only bypass before bootstrap; request-type routing happens only after task truth exists.
- Do not treat project, handoff, signal, memory, chat transcript, or PR evidence as a replacement for GitHub-backed task evidence.
- Do not skip worktree / GitHub Project task creation for any user request unless the user explicitly authorized reuse of a specific task worktree that is already bound to the same task.
- Do not stop after saying which workflow surface should be used; continue into that phase.
- Do not treat this skill as permission to bypass `repo-owned-workflow-router`, verification, or GitHub PR review.
- Do force this bootstrap onto chat-only or read-only requests, even when they do not change repository state.
- Do not treat read-only professional/domain questions as TPM-owned conclusions just because bootstrap has completed; the matching professional slice still owns the conclusion.
- Do not repeat the full bootstrap output for already-bound micro loops when a
  minimal learning-intake record is sufficient.

## Known Failure Modes

- Treating a request as "just a quick question" before task/worktree truth exists; run the bootstrap first, then decide whether the answer is pure evidence or needs a role slice.
- Creating a task from an invalid external `source_ref`; use a repository path as the PM source and record external URLs in GitHub issue evidence instead.
- Reusing a dirty or unrelated worktree because it is convenient; reuse needs explicit user authorization and matching task truth.
- Stopping after worktree creation; this skill's job includes routing into the next workflow surface and recording the decision.

## Verification

- Minimum verification commands after changing this surface:
  - `./scripts/pm/workflow-behavior-eval.sh`
  - `./scripts/lint-skills.sh`
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- Expected result:
  - bootstrap surface is discoverable in `AGENTS.md` and `.agents/skills/README.md`
  - workflow behavior eval proves all user requests route through repo-owned task truth
