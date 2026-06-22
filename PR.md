# Add repository health inspection runbook

- PR URL: https://github.com/eng-cc/oasis7/pull/559
- Task UID: task_2d354c73f1d04e25a328ad4c25c2a1a9
- Source Branch: task/engineering-repo-health-scheduled-inspection
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add a repository-health scheduled inspection runbook for weekly cc-connect reminders and quarterly review.
- Link the runbook from engineering README and record the `.pm` task trace in engineering project.
- Keep the workflow human-triaged instead of adding a new GitHub Actions hard gate.

## Verification
- `./scripts/doc-governance-check.sh`
- `./scripts/lint-skills.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_2d354c73f1d04e25a328ad4c25c2a1a9 --phase pr-ready --allow-unbound`
- `git diff --check`

## Review Follow-Up
- Addressed PR review thread `PRRT_kwDORHhWec6LNu03` by replacing `rtk bash scripts/...` checklist entries with direct repo script commands.
- Resolved the review thread after pushing the fix.

## Residual Risk
- `cc-connect` was not available in the local shell, so this PR documents the operator cron command rather than creating the cron.
- Full repo `./scripts/pm/lint.sh` still reports unrelated historical execution-log debt; repository_health_engineer and qa_engineer both reviewed this as non-blocking for this scoped docs/governance PR.
