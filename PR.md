# Harden public testnet readiness and rollout docs

- Task UID: task_bdb48338fac544849d8c681e9a7dd441
- Source Branch: task/world-runtime-testnet-health-status
- Base Branch: main
- Purpose: normal_pr_ci_watch

## Summary
- Treat noisy external public-testnet replication peer churn as diagnostic-only when the validator/request path and network head are healthy, while keeping request-path failures blocking.
- Update validator rebuild and observer reseed scripts to clear/read the current replication/runtime state paths.
- Record the five-node testnet rollout, catch-up, and operator inventory docs, including Windows artifact scope and reseed recovery rules.

## Verification
- `env -u RUSTC_WRAPPER cargo fmt --check`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime observability_transport_tests -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime observability_tests -- --nocapture`
- `bash -n scripts/p2p-public-testnet-rebuild-validators.sh scripts/p2p-public-testnet-local-observer-sync.sh scripts/p2p-public-testnet-rebuild-validators.test.sh scripts/p2p-public-testnet-local-observer-sync.test.sh`
- `bash scripts/p2p-public-testnet-local-observer-sync.test.sh`
- `bash scripts/p2p-public-testnet-rebuild-validators.test.sh`
- `./scripts/check-rust-file-size.sh`
- `./scripts/doc-governance-check.sh`
- `git diff --check`
- `./scripts/pm/workflow-lint.sh --task-uid task_bdb48338fac544849d8c681e9a7dd441 --phase current`
- `./scripts/pm/workflow-lint.sh --task-uid task_bdb48338fac544849d8c681e9a7dd441 --phase pr-ready --allow-unbound`

## Local Role Review
- Passed: `blockchain_ops_engineer`, `runtime_engineer`, `qa_engineer`, and `repository_health_engineer` findings are fixed or dispositioned in `.pm/tasks/task_bdb48338fac544849d8c681e9a7dd441.execution.md`.

## Residual Risk
- Windows observer reset/reseed is documented as SOP but is not yet a fully reusable repo-owned PowerShell script.
- Repo-wide `.pm` lint still has unrelated historical task-log debt; current task workflow lint passes.
