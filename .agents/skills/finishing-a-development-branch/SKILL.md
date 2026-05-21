---
name: finishing-a-development-branch
description: Use when implementation is done and the work needs to be closed out, committed, prepared for PR, and eventually cleaned up. Follows the oasis7 default path: task closeout, commit, PR preflight/create, review handling, merge, and worktree cleanup.
---

# Finishing a Development Branch

## When to Use

Use this skill when code and docs are already updated and you are moving into branch closure:

- close the task
- verify the final diff
- commit
- prepare or create the PR
- handle review comments
- clean up after merge

## Default Oasis7 Path

1. Confirm the task has its own worktree and `.pm` task.
2. Run the final local checks for the changed surface.
3. Close the task:

```bash
./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID>
```

4. Commit exactly this task slice.
5. Run PR preflight / create:

```bash
./scripts/prepare-task-pr.sh --create
```

6. If review comments arrive, use:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

7. After merge, sync local `main` and remove the task worktree / branch.

## Required Checks Before Commit

- worktree diff matches task scope
- task execution log updated
- relevant formal docs updated
- local verification rerun for the affected surface

## Post-Merge Cleanup

- fast-forward local `main`
- remove the task worktree
- delete the task branch after leaving that worktree

## Guardrails

- Do not land locally unless the user explicitly asks for local landing.
- Do not skip `.pm` closeout just because the execution log is updated.
- Do not claim "done" while the branch still lacks required verification or PR creation.
- Do not treat review-thread resolution as merge readiness.
