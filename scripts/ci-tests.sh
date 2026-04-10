#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tier="${1:-full}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-tests.sh [commit|required|full|full-core|full-support]

  commit        Run the lightweight local commit gate used by pre-commit.
  required      Run the explicit heavier required gate for local validation and PR gate.
  full          Run required checks plus all extended feature/integration tests.
  full-core     Run doc/fmt plus the heaviest `oasis7 --tests` full-tier shard.
  full-support  Run the remaining support crates/viewer shard plus `oasis7 --lib --bins`.

Default: full
USAGE
}

if [[ $# -gt 1 ]]; then
  usage
  exit 1
fi

case "$tier" in
  commit|required|full|full-core|full-support) ;;
  *)
    usage
    exit 1
    ;;
esac

run() {
  echo "+ $*"
  "$@"
}

run_with_retries() {
  local max_attempts=$1
  shift
  local attempt=1
  local exit_code=0
  while (( attempt <= max_attempts )); do
    set +e
    "$@"
    exit_code=$?
    set -e
    if [[ "$exit_code" -eq 0 ]]; then
      return 0
    fi
    if (( attempt == max_attempts )); then
      return "$exit_code"
    fi
    echo "retry: attempt $attempt/$max_attempts failed (exit=$exit_code), retrying..." >&2
    attempt=$((attempt + 1))
    sleep 1
  done
}

run_cargo() {
  if [[ "${CI_VERBOSE:-}" == "1" ]]; then
    run env -u RUSTC_WRAPPER cargo "$@" --verbose
  else
    run env -u RUSTC_WRAPPER cargo "$@"
  fi
}

run_oasis7_required_tier_tests() {
  run_cargo test -p oasis7 --tests --features test_tier_required
}

run_oasis7_full_tier_tests() {
  run_cargo test -p oasis7 --tests --features "test_tier_full,wasmtime,viewer_live_integration" -- --skip live_server_accepts_client_and_emits_snapshot_and_event
  run_with_retries 3 \
    run_cargo test -p oasis7 --features "test_tier_full,wasmtime,viewer_live_integration" \
      --test viewer_live_integration live_server_accepts_client_and_emits_snapshot_and_event -- --nocapture
}

run_oasis7_consensus_tests() {
  run_cargo test -p oasis7_consensus --lib
}

run_oasis7_distfs_tests() {
  run_cargo test -p oasis7_distfs --lib
}

run_oasis7_node_tests() {
  run_cargo test -p oasis7_node --lib
}

run_oasis7_net_tests() {
  run_cargo test -p oasis7_net --lib
}

run_oasis7_net_libp2p_tests() {
  run_cargo test -p oasis7_net --features libp2p --lib
}

run_oasis7_llm_baseline_fixture_smoke() {
  run ./scripts/llm-baseline-fixture-smoke.sh
}

run_oasis7_viewer_tests() {
  run_cargo test -p oasis7_viewer
}

run_oasis7_viewer_software_safe_feedback_contract_tests() {
  run node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs
}

run_oasis7_viewer_wasm_check() {
  run_cargo check -p oasis7_viewer --target wasm32-unknown-unknown
}

run_required_gate_checks() {
  run ./scripts/doc-governance-check.sh
  run ./scripts/check-rust-file-size.sh
  run env -u RUSTC_WRAPPER cargo fmt --all -- --check
}

run_commit_gate_checks() {
  run_required_gate_checks
  run_oasis7_consensus_tests
  run_oasis7_distfs_tests
  run_oasis7_viewer_software_safe_feedback_contract_tests
}

run_full_core_tier_tests() {
  run_required_gate_checks
  run_oasis7_full_tier_tests
}

run_full_support_tier_tests() {
  run_oasis7_consensus_tests
  run_oasis7_distfs_tests
  run_oasis7_node_tests
  run_oasis7_net_tests
  run_oasis7_net_libp2p_tests
  run_oasis7_llm_baseline_fixture_smoke
  run_oasis7_viewer_tests
  run_oasis7_viewer_software_safe_feedback_contract_tests
  run_oasis7_viewer_wasm_check
  run_cargo test -p oasis7 --features wasmtime --lib --bins
}

echo "+ ci test tier: $tier"
case "$tier" in
  commit)
    run_commit_gate_checks
    ;;
  required)
    run_required_gate_checks
    run_oasis7_required_tier_tests
    run_oasis7_consensus_tests
    run_oasis7_distfs_tests
    run_oasis7_viewer_tests
    run_oasis7_viewer_software_safe_feedback_contract_tests
    run_oasis7_viewer_wasm_check
    ;;
  full)
    run_full_core_tier_tests
    run_full_support_tier_tests
    ;;
  full-core)
    run_full_core_tier_tests
    ;;
  full-support)
    run_full_support_tier_tests
    ;;
  *)
    usage
    exit 1
    ;;
 esac
