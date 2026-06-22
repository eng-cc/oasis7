# Remove legacy messaging bridge guidance

- PR URL: https://github.com/eng-cc/oasis7/pull/561
- Task UID: task_198cdd132d3e4fda9f5fc9b4f46f412e
- Source Branch: task/engineering-cc-connect-cleanup
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Remove the active legacy messaging bridge instructions from `AGENTS.md`.
- Replace scheduler-specific repository-health runbook guidance with scheduler-neutral reminder text.
- Keep historical `.pm` task evidence intact as audit truth while removing active non-`.pm` guidance.

## Verification
- `NO_ACTIVE_MATCHES` from active-surface grep outside `.pm`, `.git`, and `third_party`.
- `./scripts/doc-governance-check.sh`
- `./scripts/lint-skills.sh`
- `git diff --check`
- `./scripts/pm/workflow-lint.sh --task-uid task_198cdd132d3e4fda9f5fc9b4f46f412e --phase pr-ready`
- `./scripts/pm/workflow-lint.sh --task-uid task_198cdd132d3e4fda9f5fc9b4f46f412e --phase post-pr --allow-unbound`

## Review Follow-Up
- Addressed review thread `PRRT_kwDORHhWec6LRCgY` by replacing the stale root `PR.md` with this task's PR evidence chain.

## Residual Risk
- `.pm` current and historical task evidence intentionally retains removed-tool mentions as task/audit truth.
- Full repo `./scripts/pm/lint.sh` remains noisy from broad `.pm` evidence strictness debt; task-scoped workflow lint and docs/governance checks pass for this cleanup.
