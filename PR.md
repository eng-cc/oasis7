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

## Local Role Review
- Pending pre-PR local role review.

## Residual Risk
- This PR documents the sampling model only; it does not add a new automated code-style gate or sampling threshold.
- Operators still need judgment when deciding whether repeated findings justify path-level deep reading.
