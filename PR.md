# Add Rust style guide to repository inspection

- PR URL: pending
- Task UID: task_3fcca9c7548f47d9be1d79a06ffcb59f
- Source Branch: task/engineering-repo-health-rust-style-guide-check
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add `third_party/rust-skills/AGENTS.md` as a read-only Rust style-guide input to the manual repository-health inspection.
- Document style-drift checks for owned Rust code, including Rust 2024/lint defaults, line length, library-code `unwrap()`, and unsafe `SAFETY` comments.
- Keep `third_party` code read-only and route style findings into focused follow-up tasks.

## Verification
- `./scripts/pm/workflow-lint.sh --task-uid task_3fcca9c7548f47d9be1d79a06ffcb59f --phase current`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## Residual Risk
- This PR documents the style-guide inspection input only; it does not add a new automated Rust style gate.
