# Switch repository health inspection to manual trigger

- PR URL: pending
- Task UID: task_ef87a7a54b764b13ae8be86dd6f54a77
- Source Branch: task/engineering-repo-health-manual-inspection-trigger
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Replace the repository-health scheduled inspection runbook with a manual-trigger runbook.
- Remove the `cc-connect` / cron scheduler entry point from the repository-health inspection flow.
- Update engineering README, project trace, and task evidence to point at the manual inspection path.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_ef87a7a54b764b13ae8be86dd6f54a77 --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command './scripts/pm/workflow-lint.sh --task-uid task_ef87a7a54b764b13ae8be86dd6f54a77 --phase current && ./scripts/doc-governance-check.sh && git diff --check'`

## Local Role Review
- repository_health_engineer found stale cadence wording in the manual runbook; fixed by replacing it with blocking-inspection-finding wording.
- qa_engineer found missing claim-ready evidence and stale `PR.md` content; fixed before PR creation.

## Residual Risk
- Global `AGENTS.md` still documents cc-connect/cron usage generally. This PR only removes the repository-health inspection automatic trigger requested by the user.
- Full repo `./scripts/pm/lint.sh` still reports unrelated historical execution-log debt after the current-task evidence is fixed and task-local gates pass.
