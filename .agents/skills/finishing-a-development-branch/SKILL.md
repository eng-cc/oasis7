---
name: finishing-a-development-branch
description: "Use when implementation is done and the work needs to be closed out, committed, prepared for PR, and eventually cleaned up. Follows the oasis7 default path: task closeout, commit, pre-PR local role review, PR preflight/create, review handling, merge, and worktree cleanup."
---

> Workflow authority: `doc/engineering/workflow/source-of-truth.md` is the single normative workflow spec. Keep this skill as short operational guidance only; if behavior changes, update source-of-truth first, then sync this file.


# Finishing a Development Branch

## When to Use

Use this skill when code and docs are already updated and you are moving into branch closure:

- close the task
- verify the final diff
- commit
- collect fresh local involved-role subagent review
- prepare or create the PR
- handle review comments
- clean up after merge

## Default Oasis7 Path

1. Confirm the task has its own worktree and `.pm` task.
2. Run the final local checks for the changed surface.
3. Close the task:

```bash
./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"
```

4. Commit exactly this task slice.
5. Dispatch fresh local subagent review for every involved relevant role, address valid findings, and record the passed evidence packet:

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

6. Run PR preflight / create:

```bash
./scripts/prepare-task-pr.sh --create
```

7. If review comments arrive, use:

```bash
./scripts/pr-review-thread-closeout.sh --unresolved-only
```

8. After merge, sync local `main` and remove the task worktree / branch.

## Required Checks Before Commit

- worktree diff matches task scope
- task execution log updated
- relevant formal docs updated
- local verification rerun for the affected surface
- pre-PR local role review packet recorded when the next step is PR creation

## Post-Merge Cleanup

- fast-forward local `main`
- remove the task worktree
- delete the task branch after leaving that worktree

## Guardrails

- Do not land locally unless the user explicitly asks for local landing.
- Do not skip `.pm` closeout just because the execution log is updated.
- Do not claim "done" while the branch still lacks required verification or PR creation.
- Do not treat review-thread resolution as merge readiness.
