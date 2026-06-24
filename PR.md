# Add versions to internal path dependencies

- Task UID: task_d3474ab9cb2b46f29409b3bd575c8c2d
- PR: https://github.com/eng-cc/oasis7/pull/611
- Source Branch: task/engineering-rust-governance-next-issue-3
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Add `version = "0.1.0"` metadata to cargo-deny report-visible workspace internal path dependencies.
- Clear cargo-deny wildcard warnings from the Rust governance report without changing `Cargo.lock`, external dependency versions, or `deny.toml`.
- Record the task trace and PR evidence in engineering project/task truth.

## Verification
- `env -u RUSTC_WRAPPER cargo metadata --locked --format-version 1`
- `./scripts/ci-rust-governance-report.sh --out-dir output/rust-governance/internal-path-version-ratchet-smoke`
- `! rg -n "warning\\[wildcard\\]" output/rust-governance/internal-path-version-ratchet-smoke/cargo-deny.log`
- `python3 -m json.tool output/rust-governance/internal-path-version-ratchet-smoke/summary.json`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_d3474ab9cb2b46f29409b3bd575c8c2d --phase current`
- `git diff --check`

## Local Role Review
- Passed/handled: `repository_health_engineer`, `qa_engineer`, `runtime_engineer`, `wasm_platform_engineer`, `blockchain_ops_engineer`, and `producer_system_designer`.
- Producer finding on `doc/engineering/project.md` latest-complete drift was addressed.

## Residual Risk
- Duplicate dependency and unsafe usage counts remain report-only debt.
- Optional/report-invisible internal path dependencies remain future consistency debt.
