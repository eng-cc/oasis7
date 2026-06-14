## Summary

- Remove the unused `oasis7` crate `self_tests` feature.
- Make the provider local bridge `Action` import test-only.
- Record simulation cleanup audit task evidence and engineering project trace.

## Verification

- `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_provider_local_bridge`
- `env -u RUSTC_WRAPPER cargo check -p oasis7 --tests --features "test_tier_full,wasmtime,viewer_live_integration"`
- `rg -n "self_tests" crates/oasis7 Cargo.toml scripts doc .github --glob '!target/**' --glob '!third_party/**'`
- `./scripts/pm/workflow-lint.sh --task-uid task_ef969e0f0f5b4b9f8fdb7349e7a015dc --phase pr-ready`
- `git diff --check`

## PR Evidence

- PR URL: https://github.com/eng-cc/oasis7/pull/469
- task_uid: task_ef969e0f0f5b4b9f8fdb7349e7a015dc
- Pre-PR Local Role Review: passed for `repository_health_engineer` and `qa_engineer`.
- PR Purpose Decision: normal_pr_ci_watch
- Residual risk: external/downstream use of removed `oasis7` `self_tests` feature is not proven absent beyond repo-owned searches; repo-wide `./scripts/pm/lint.sh` still has unrelated historical task-log structure debt outside this task.
