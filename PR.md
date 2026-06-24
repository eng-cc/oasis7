## Summary

- Add explicit `version = "0.1.0"` metadata to 37 builtin WASM module `oasis7_wasm_sdk` internal path dependencies.
- Update engineering project tracking and task evidence for `builtin-wasm-module-internal-path-version-ratchet`.
- Keep Rust source, feature wiring, `Cargo.lock`, external dependency versions, `deny.toml`, and workspace membership unchanged.

Task UID: `task_f1990219b9bd41ed99db93c13a64c0c5`

## Verification

- path-only builtin `oasis7_wasm_sdk` dependency count is `0`
- versioned builtin `oasis7_wasm_sdk` dependency count is `37`
- representative M1/M4/M5 cargo metadata reports `oasis7_wasm_sdk` as `^0.1.0` with `wire`
- `./scripts/ci-rust-governance-report.sh --out-dir output/rust-governance/builtin-wasm-internal-path-version-ratchet-claim`
- `python3 -m json.tool output/rust-governance/builtin-wasm-internal-path-version-ratchet-claim/summary.json >/tmp/oasis7-next-issue-5-claim-summary.pretty.json`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_f1990219b9bd41ed99db93c13a64c0c5 --phase current`
- `git diff --check`

## Local Role Review

- `repository_health_engineer`: no findings
- `wasm_platform_engineer`: no findings; ABI/manifest/hash/determinism checks not applicable to manifest metadata-only change
- `qa_engineer`: no findings
- `producer_system_designer`: no findings
