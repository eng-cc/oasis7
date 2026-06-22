# Add code and dependency health to repository inspection

- PR URL: https://github.com/eng-cc/oasis7/pull/564
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
- `./scripts/pm/task-closeout.sh --role tpm --task-uid task_b0a770d8447340f6844e1cff07f99a37 --verify-command './scripts/pm/workflow-lint.sh --task-uid task_b0a770d8447340f6844e1cff07f99a37 --phase current && ./scripts/doc-governance-check.sh && git diff --check' --json`
- `./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command './scripts/pm/workflow-lint.sh --task-uid task_b0a770d8447340f6844e1cff07f99a37 --phase current && ./scripts/doc-governance-check.sh && git diff --check'`
- `./scripts/pm/workflow-lint.sh --task-uid task_b0a770d8447340f6844e1cff07f99a37 --phase pr-ready --allow-unbound`

## Local Role Review
- repository_health_engineer: no findings; scope/spec compliance passed and risk acceptable.
- qa_engineer: found missing pre-PR/claim-ready evidence and incomplete PR verification evidence; fixed before PR creation.

## Residual Risk
- The runbook now references existing report/gate commands; it does not add a new automated dependency-upgrade tool or schedule.
- `ci-rust-governance-report` was not run for this docs-only change; it is documented as an inspection command, not a validation requirement for this PR.
