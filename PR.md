## Summary

- Add explicit `version = "0.1.0"` metadata to the two remaining optional internal path dependencies visible in all-features workspace metadata.
- Update engineering project tracking and task evidence for `rust-governance-all-features-internal-path-version-ratchet`.
- Keep `Cargo.lock`, external dependency versions, and `deny.toml` unchanged.

Task UID: `task_3567a753dcca412f8794bd5b613642d9`

## Verification

- `env -u RUSTC_WRAPPER cargo metadata --locked --format-version 1`
- `env -u RUSTC_WRAPPER cargo metadata --locked --format-version 1 --all-features`
- all-features internal path `req="*"` metadata query returned no rows
- `cargo deny check --metadata-path /tmp/oasis7-next-issue-4-all-features-metadata-after-rerun.json`
- `test -s /tmp/oasis7-next-issue-4-cargo-deny-all-features-rerun.log && ! rg -n "warning\\[wildcard\\]" /tmp/oasis7-next-issue-4-cargo-deny-all-features-rerun.log`
- `./scripts/ci-rust-governance-report.sh --out-dir output/rust-governance/all-features-internal-path-version-ratchet-smoke`
- `python3 -m json.tool output/rust-governance/all-features-internal-path-version-ratchet-smoke/summary.json >/tmp/oasis7-next-issue-4-summary.pretty.json`
- `./scripts/doc-governance-check.sh`
- `./scripts/pm/workflow-lint.sh --task-uid task_3567a753dcca412f8794bd5b613642d9 --phase current`
- `git diff --check`

## Local Role Review

- `repository_health_engineer`: no findings
- `qa_engineer`: no findings
- `runtime_engineer`: no findings; runtime replay/recovery/checkpoint/long-run checks not applicable to manifest metadata-only change
- `wasm_platform_engineer`: no findings; ABI/manifest/hash/determinism checks not applicable to manifest metadata-only change
- `blockchain_ops_engineer`: no findings; no deployment change
- `producer_system_designer`: no findings
