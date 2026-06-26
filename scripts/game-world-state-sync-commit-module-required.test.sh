#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/oasis7-gwsc-module-required-test.XXXXXX")
trap 'rm -rf "$tmp_root"' EXIT

run() {
  echo "+ $*"
  "$@"
}

run ./scripts/game-world-state-sync-commit-module-required.sh \
  --dry-run \
  --out-dir "$tmp_root/full"

summary=$(find "$tmp_root/full" -type f -name summary.json | sort | tail -n 1)

jq -e '
  .claim_boundary == "module_required"
  and (.does_not_claim | index("multi-node world sync")) != null
  and (.does_not_claim | index("release_full")) != null
  and (.does_not_claim | index("public_testnet ready")) != null
  and .overall_status == "dry_run"
  and .dry_run == true
  and .totals.step_count == 7
  and .totals.cargo_step_count == 6
  and .totals.matrix_step_count == 1
  and .totals.dry_run_count == 7
  and .totals.skipped_count == 0
  and [.steps[].base_command] == [
    "env -u RUSTC_WRAPPER cargo test -p oasis7 --tests --features test_tier_required",
    "env -u RUSTC_WRAPPER cargo test -p oasis7_node",
    "env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib",
    "env -u RUSTC_WRAPPER cargo test -p oasis7_net --features libp2p --lib",
    "env -u RUSTC_WRAPPER cargo test -p oasis7_consensus --lib",
    "env -u RUSTC_WRAPPER cargo test -p oasis7_distfs --lib",
    "./scripts/p2p-mixed-topology-matrix.sh --tier required"
  ]
  and any(.steps[]; .step_id == "p2p_mixed_topology_required" and (.command | contains("--dry-run")) and (.command | contains("--out-dir")))
' "$summary" >/dev/null

run ./scripts/game-world-state-sync-commit-module-required.sh \
  --dry-run \
  --skip-cargo \
  --out-dir "$tmp_root/skip-cargo"

skip_cargo_summary=$(find "$tmp_root/skip-cargo" -type f -name summary.json | sort | tail -n 1)

jq -e '
  .claim_boundary == "module_required"
  and .overall_status == "dry_run"
  and .skips.cargo == true
  and .skips.matrix == false
  and .totals.step_count == 7
  and .totals.skipped_count == 6
  and .totals.dry_run_count == 1
  and all(.steps[] | select(.kind == "cargo"); .status == "skipped")
  and any(.steps[]; .kind == "matrix" and .status == "dry_run")
' "$skip_cargo_summary" >/dev/null

run ./scripts/game-world-state-sync-commit-module-required.sh \
  --dry-run \
  --skip-matrix \
  --out-dir "$tmp_root/skip-matrix"

skip_matrix_summary=$(find "$tmp_root/skip-matrix" -type f -name summary.json | sort | tail -n 1)

jq -e '
  .claim_boundary == "module_required"
  and .overall_status == "dry_run"
  and .skips.cargo == false
  and .skips.matrix == true
  and .totals.step_count == 7
  and .totals.skipped_count == 1
  and .totals.dry_run_count == 6
  and any(.steps[]; .kind == "matrix" and .status == "skipped")
  and all(.steps[] | select(.kind == "cargo"); .status == "dry_run")
' "$skip_matrix_summary" >/dev/null

echo "GWSC module_required wrapper smoke checks passed"
