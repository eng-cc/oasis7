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

run_cargo() {
  if [[ "${CI_VERBOSE:-}" == "1" ]]; then
    run env -u RUSTC_WRAPPER cargo "$@" --verbose
  else
    run env -u RUSTC_WRAPPER cargo "$@"
  fi
}

should_run_ci_required_component() {
  local raw_value="${1:-}"
  [[ -z "$raw_value" || "$raw_value" == "1" || "$raw_value" == "true" ]]
}

run_required_component() {
  local label="$1"
  local raw_value="$2"
  shift 2

  if should_run_ci_required_component "$raw_value"; then
    "$@"
  else
    echo "skip: ${label} disabled by CI scope planner"
  fi
}

run_oasis7_required_tier_tests() {
  run_cargo test -p oasis7 --tests --features test_tier_required
}

run_oasis7_full_tier_tests() {
  run_cargo test -p oasis7 --tests --features "test_tier_full,wasmtime,viewer_live_integration"
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

run_oasis7_viewer_software_safe_feedback_contract_tests() {
  run node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs
  run ./scripts/copy-viewer-web-dist.test.sh
  run ./scripts/agent-browser-viewer-dist-freshness-test.sh
  run npm --prefix crates/oasis7_viewer run test:ui
}

run_oasis7_viewer_software_safe_build() {
  run ./scripts/build-viewer-software-safe.sh
}

run_oasis7_client_launcher_web_build() {
  run mkdir -p output/release/web-launcher-dist
  (
    cd crates/oasis7_client_launcher
    run env -u NO_COLOR trunk build --release --dist ../../output/release/web-launcher-dist
  )
}

run_required_gate_checks() {
  run ./scripts/doc-governance-check.sh
  run ./scripts/check-windows-paths.sh
  run bash ./scripts/check-script-executable-bits.sh
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
  run_oasis7_viewer_software_safe_feedback_contract_tests
  run_oasis7_viewer_software_safe_build
  run_cargo test -p oasis7 --features wasmtime --lib --bins
}

echo "+ ci test tier: $tier"
case "$tier" in
  commit)
    run_commit_gate_checks
    ;;
  required)
    run_required_gate_checks
    run_required_component "oasis7 required tests" "${OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:-}" run_oasis7_required_tier_tests
    run_required_component "oasis7_consensus tests" "${OASIS7_CI_RUN_CONSENSUS_TESTS:-}" run_oasis7_consensus_tests
    run_required_component "oasis7_distfs tests" "${OASIS7_CI_RUN_DISTFS_TESTS:-}" run_oasis7_distfs_tests
    run_required_component "oasis7_node tests" "${OASIS7_CI_RUN_OASIS7_NODE_TESTS:-false}" run_oasis7_node_tests
    run_required_component "oasis7_net tests" "${OASIS7_CI_RUN_OASIS7_NET_TESTS:-false}" run_oasis7_net_tests
    run_required_component "oasis7_net libp2p tests" "${OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS:-false}" run_oasis7_net_libp2p_tests
    run_required_component "viewer software-safe contract" "${OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS:-}" run_oasis7_viewer_software_safe_feedback_contract_tests
    run_required_component "viewer software-safe build" "${OASIS7_CI_RUN_VIEWER_WASM_CHECK:-}" run_oasis7_viewer_software_safe_build
    run_required_component "launcher web build" "${OASIS7_CI_RUN_LAUNCHER_WEB_BUILD:-false}" run_oasis7_client_launcher_web_build
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
