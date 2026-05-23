#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT_DIR"

env -u RUSTC_WRAPPER cargo test -p oasis7 \
  viewer::runtime_live::tests::snapshot_progress::compat_snapshot_surfaces_control_feeling_contract_fields_from_gameplay_feedback \
  -- --nocapture

env -u RUSTC_WRAPPER cargo test -p oasis7 \
  viewer::runtime_live::tests::snapshot_progress::compat_snapshot_keeps_post_onboarding_no_progress_after_confirmed_progress \
  -- --nocapture

env -u RUSTC_WRAPPER cargo test -p oasis7 \
  viewer::runtime_live::tests::snapshot_progress::compat_snapshot_blocks_first_session_when_chain_sync_is_unavailable \
  -- --nocapture

env -u RUSTC_WRAPPER cargo test -p oasis7 \
  simulator::tests::persist::snapshot_player_gameplay_execution_state_backfills_from_legacy_fields \
  -- --nocapture

node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs

(
  cd crates/oasis7_viewer
  npm run test:ui -- software_safe_src/main.test.jsx
)

./scripts/build-viewer-software-safe.sh
./scripts/doc-governance-check.sh
git diff --check
