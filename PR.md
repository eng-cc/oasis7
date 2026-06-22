# Add code and dependency health to repository inspection

- PR URL: pending
- Task UID: task_b0a770d8447340f6844e1cff07f99a37
- Source Branch: task/engineering-repo-health-code-dependency-inspection
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Extend the manual repository-health inspection runbook with code-health and dependency-health checks.
- Point the checklist at the existing Rust governance report and required-gate surfaces.
- Record that dependency upgrades should become focused follow-up tasks instead of being performed inside the inspection task.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_b0a770d8447340f6844e1cff07f99a37 --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## Residual Risk
- The runbook now references existing report/gate commands; it does not add a new automated dependency-upgrade tool or schedule.
