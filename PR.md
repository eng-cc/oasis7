# Add repository health code sampling guidance

- Task UID: task_4fb500e6782e4eac916f6846e01542af
- Source Branch: task/engineering-repo-health-file-coverage-guidance
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add a `Code Evidence Sampling Model` section to the manual repository-health inspection runbook.
- Clarify that code-style and code-health conclusions need code evidence, but the default inspection model is automated full-repository scans plus high-risk code sampling, not manual all-file reading.
- Document when to escalate from sampling to path-level deep reading.
- Record the task trace in `doc/engineering/project.md`.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_4fb500e6782e4eac916f6846e01542af --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `./scripts/pm/task-closeout.sh --role tpm --task-uid task_4fb500e6782e4eac916f6846e01542af --verify-command './scripts/pm/workflow-lint.sh --task-uid task_4fb500e6782e4eac916f6846e01542af --phase current && ./scripts/doc-governance-check.sh && git diff --check' --json`
- `./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command './scripts/pm/workflow-lint.sh --task-uid task_4fb500e6782e4eac916f6846e01542af --phase current && ./scripts/doc-governance-check.sh && git diff --check'`
- `./scripts/pm/workflow-lint.sh --task-uid task_4fb500e6782e4eac916f6846e01542af --phase pr-ready --allow-unbound`
- `./scripts/prepare-task-pr.sh --body-file PR.md --title "Add repository health code sampling guidance" --json`

## Local Role Review
- repository_health_engineer: no findings; scope/spec compliance passed and role quality/risk acceptable.
- qa_engineer: found stale review-package base, pending local-review evidence, and missing PR-readiness evidence; addressed before PR creation by rebasing/regenerating the package and adding claim-ready/pr-ready/preflight evidence.

## Residual Risk
- This PR documents the sampling model only; it does not add a new automated code-style gate or sampling threshold.
- Operators still need judgment when deciding whether repeated findings justify path-level deep reading.
