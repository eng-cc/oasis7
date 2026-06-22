# Add Rust style guide to repository inspection

- PR URL: https://github.com/eng-cc/oasis7/pull/566
- Task UID: task_3fcca9c7548f47d9be1d79a06ffcb59f
- Source Branch: task/engineering-repo-health-rust-style-guide-check
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add `third_party/rust-skills/AGENTS.md` as a read-only Rust style-guide input to the manual repository-health inspection.
- Document style-drift checks for owned Rust code, including Rust 2024/lint defaults, line length, library-code `unwrap()`, and unsafe `SAFETY` comments.
- Document the fresh-worktree submodule initialization step needed before reading the Rust style guide.
- Keep `third_party` code read-only and route style findings into focused follow-up tasks.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `./scripts/pm/task-closeout.sh --role tpm --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --verify-command './scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase current && ./scripts/doc-governance-check.sh && git diff --check' --json`
- `./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command './scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase current && ./scripts/doc-governance-check.sh && git diff --check'`
- `./scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase pr-ready --allow-unbound`
- Post-review fix: `./scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase current`; `./scripts/doc-governance-check.sh`; `git diff --check`

## Local Role Review
- repository_health_engineer: no findings; scope/spec compliance passed and risk acceptable.
- qa_engineer: found missing pre-PR/claim-ready evidence and incomplete PR verification evidence; fixed before PR creation.
- Codex PR review: valid P2 finding about documenting submodule initialization; fixed by adding the `git submodule update --init -- third_party/rust-skills` prerequisite and missing-input evidence path.

## Residual Risk
- This PR documents the style-guide inspection input only; it does not add a new automated Rust style gate.
- The runbook does not define quantitative sampling thresholds for style drift; operators use judgment during manual inspection.
