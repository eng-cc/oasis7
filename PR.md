## Summary

- Refresh the simulation cleanup audit evidence for the current task.
- Reclassify the old world-simulator PRD review checklist as a historical 2026-03-05 snapshot instead of current active truth.
- Record follow-up reflection signals for the larger `oasis7_init_demo` and `oasis7_llm_agent_demo*` retirement candidates, leaving direct code deletion out of this governance-only patch.

## Verification

- `./scripts/pm/workflow-lint.sh --task-uid task_41b18b1a7fef4d7b95e5d51aac64974f --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## PR Evidence

- task_uid: task_41b18b1a7fef4d7b95e5d51aac64974f
- Source worktree: `/Users/scc/ccwork/worktrees/oasis7-engineering-simulation-cleanup-audit`
- Branch: `task/engineering-simulation-cleanup-audit`
- Repository-health conclusion: no active simulation core code/docs are safe to delete immediately; only governance/evidence cleanup and future retirement follow-ups are in scope.
- Residual risk: demo/bin retirement still needs runtime, agent, and QA replacement evidence before any code deletion. Repo-wide `./scripts/pm/lint.sh` was run and still fails on unrelated historical task-log structure debt; task-local workflow lint passes for this task.
