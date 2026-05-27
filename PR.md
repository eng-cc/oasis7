## Summary

- Add a required-scope planner regression for unclassified Rust crate paths.
- Expand `full-support` to directly test workspace support crates and `oasis7_client_launcher`.
- Fix test-only drift exposed by the expanded coverage shard.

## Verification

- `./scripts/ci-tests.sh required`
- `./scripts/ci-tests.sh full-support`
- `git diff --check`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/lint.sh`

## PM

- task_uid: task_ce44b8a269824fbcb718febd2140c425
