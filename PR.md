# PR Evidence

## Harden WASM Rust governance surfaces

- Harden WASM executor compiled-cache use by verifying request bytes against `wasm_hash` before any memory or disk cache hit.
- Validate path-sensitive WASM artifact/module identifiers before filesystem joins, and map invalid store keys into module-change validation errors.
- Strengthen prepared subscription and regex cache behavior with manifest-aware subscription keys plus prepared regex cache/metrics coverage.

Verification:
- `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_store artifact_paths_reject_path_shaped_wasm_hashes`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor --features wasmtime wasm_executor_rejects_wasm_hash_bytes_mismatch_before_cache_hit`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_executor --features wasmtime wasm_executor_epoch_watchdog_preempts_infinite_loop`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_router router_metrics_track_prepared_regex_compile_count`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 prepared_subscription_cache_key_tracks_manifest_identity`
- `env -u RUSTC_WRAPPER cargo test --manifest-path tools/wasm_build_suite/Cargo.toml minimal_template_rejects_path_shaped_module_id`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime observability`
- `./scripts/pm/workflow-lint.sh --task-uid task_61ae4b646e994b3e91bff3141f7ef818 --phase pr-ready`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

Evidence:
- task_uid: task_61ae4b646e994b3e91bff3141f7ef818
- PR URL: https://github.com/eng-cc/oasis7/pull/483
- Source worktree: `/Users/scc/ccwork/worktrees/oasis7-engineering-wasm-rust-governance-review`
- Branch: `task/engineering-wasm-rust-governance-review`
- Pre-PR Local Role Review: passed
- Review roles: `wasm_platform_engineer`, `runtime_engineer`, `repository_health_engineer`, `qa_engineer`
- Residual risk: targeted local tests cover the changed hardening surfaces; broader package/workspace checks, GitHub required checks, comments, requested changes, review threads, and mergeability remain required after PR creation.

## Harden Launcher Provider Configuration Contracts

PR URL: https://github.com/eng-cc/oasis7/pull/482

Task: .pm/tasks/task_169255fb26a2410a9c9edfaa839fc466.yaml
Task UID: task_169255fb26a2410a9c9edfaa839fc466

Summary:
- Add tested web launcher agent-provider schema/config/args coverage.
- Make provider-backed web config fail closed for invalid provider subfields, transport URL policy, invalid DNS hostnames, and invalid agent_decision_source.
- Redact provider auth tokens from state responses while preserving child-process forwarding.
- Clarify trusted_local_only as an internal local-playtest escape hatch and share HTTP base URL parser coverage.
