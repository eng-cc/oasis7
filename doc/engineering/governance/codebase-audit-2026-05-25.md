# Codebase Audit Tasks (2026-05-25)

## Scope
- Reviewed representative code and docs in networking tests, viewer UI module, launcher tooling, and engineering project tracking.
- Goal: identify concrete, small follow-up tasks for quality improvement.

## Proposed Tasks

### 1) Fix one spelling typo
- **Finding**: `followup` is used as a single word in task labels; team conventions elsewhere commonly use `follow-up`.
- **Example**: `pr-108-vendor-egui-review-comment-followup` in `doc/engineering/project.md`.
- **Proposed task**: normalize wording to `follow-up` for readability and consistency across project/task labels.
- **Risk**: low (doc-only rename/update).

### 2) Fix one code error (panic-prone request parsing)
- **Finding**: test helper HTTP parser assumes request headers are UTF-8 and panics on decode failure via `expect("request header should be utf-8")`.
- **Example**: `crates/oasis7/src/bin/oasis7_provider_local_bridge.rs` request parsing paths.
- **Proposed task**: replace panic path with fallible handling (`from_utf8` branch) and return a structured parse error or safe fallback for malformed headers.
- **Risk**: medium (touches request parsing behavior; should include regression tests).

### 3) Correct one code comment / documentation inconsistency
- **Finding**: world-bounds UI copy says bounds are derived from `snapshot.config.space`; implementation in the same module also falls back to first location anchor when selected object is absent, which can be interpreted as mixed derivation sources.
- **Example**: `crates/oasis7_viewer/software_safe_src/viewer_world_scale_module.js`.
- **Proposed task**: clarify wording in inline text/comments to distinguish **world bounds source** vs **anchor selection fallback**, preventing reader confusion.
- **Risk**: low (copy/comment alignment).

### 4) Improve one test
- **Finding**: discovery peer-record tests verify dial eligibility via map membership only, but do not assert idempotency details after upgrade transitions.
- **Example**: `process_discovered_peer_record_keeps_single_source_bootstrap_peer_dial_eligible` in `crates/oasis7_net/src/libp2p_net/tests/discovery_peer_record_tests.rs`.
- **Proposed task**: strengthen assertions to verify transition invariants (no duplicate scheduling side effects, expected source upgrade preserved, and stable dial eligibility across repeated updates).
- **Risk**: low-to-medium (test-only; may expose latent logic edge cases).

## Suggested execution order
1. Task 1 (spelling)
2. Task 3 (comment/doc consistency)
3. Task 4 (test strengthening)
4. Task 2 (panic-path hardening + regression tests)
