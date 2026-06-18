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

run_cargo_clippy() {
  local lint_flags=(
    -D clippy::correctness
    -D clippy::suspicious
  )
  if [[ "${CI_VERBOSE:-}" == "1" ]]; then
    run env -u RUSTC_WRAPPER cargo clippy --verbose "$@" -- "${lint_flags[@]}"
  else
    run env -u RUSTC_WRAPPER cargo clippy "$@" -- "${lint_flags[@]}"
  fi
}

should_run_ci_required_component() {
  local raw_value="${1:-}"
  [[ -z "$raw_value" || "$raw_value" == "1" || "$raw_value" == "true" ]]
}

run_required_component() {
  local label="$1"
  local raw_value="$2"
  local skip_reason="${3:-disabled_by_scope_planner}"
  if [[ $# -gt 2 ]]; then
    shift 3
  else
    shift 2
  fi

  if should_run_ci_required_component "$raw_value"; then
    "$@"
  else
    echo "skip: ${label} reason=${skip_reason} claim_boundary=not_covered_by_this_required_run"
  fi
}

run_oasis7_required_tier_tests() {
  run_cargo test -p oasis7 --tests --features test_tier_required
}

run_oasis7_required_tier_clippy() {
  run_cargo_clippy -p oasis7 --tests --features test_tier_required
}

run_oasis7_full_tier_tests() {
  run_cargo test -p oasis7 --tests --features "test_tier_full,wasmtime,viewer_live_integration"
}

run_oasis7_consensus_tests() {
  run_cargo test -p oasis7_consensus --lib
}

run_oasis7_consensus_clippy() {
  run_cargo_clippy -p oasis7_consensus --lib
}

run_oasis7_distfs_tests() {
  run_cargo test -p oasis7_distfs --lib
}

run_oasis7_distfs_clippy() {
  run_cargo_clippy -p oasis7_distfs --lib
}

run_oasis7_node_tests() {
  run_cargo test -p oasis7_node --lib
}

run_oasis7_node_clippy() {
  run_cargo_clippy -p oasis7_node --lib
}

run_oasis7_net_tests() {
  run_cargo test -p oasis7_net --lib
}

run_oasis7_net_clippy() {
  run_cargo_clippy -p oasis7_net --lib
}

run_oasis7_net_libp2p_tests() {
  run_cargo test -p oasis7_net --features libp2p --lib
}

run_oasis7_net_libp2p_clippy() {
  run_cargo_clippy -p oasis7_net --features libp2p --lib
}

run_oasis7_workspace_support_crate_tests() {
  run_cargo test \
    -p oasis7_launcher_ui \
    -p oasis7_proto \
    -p oasis7_wasm_abi \
    -p oasis7_wasm_build \
    -p oasis7_wasm_router \
    -p oasis7_wasm_sdk \
    -p oasis7_wasm_store \
    -p pixel_world_bridge \
    --lib
  run_cargo test -p oasis7_wasm_executor --features wasmtime --lib
  run_cargo test -p oasis7_client_launcher --bin oasis7_client_launcher
}

run_rustsec_advisory_check() {
  run ./scripts/ensure-cargo-deny.sh
  run cargo deny check advisories
}

run_oasis7_llm_baseline_fixture_smoke() {
  run ./scripts/llm-baseline-fixture-smoke.sh
}

run_provider_remote_https_smoke() {
  run ./scripts/run-local-letai-game-test.test.sh
  run ./scripts/provider-remote-https/letai-provider-cli.test.sh
  run ./scripts/provider-remote-https/provider-bridge-contract-smoke.test.sh
}

run_provider_bridge_live_gate() {
  run ./scripts/provider-remote-https/provider-bridge-live-gate.sh
}

run_newapi_bridge_service_accounting_tests() {
  run env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture
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

run_oasis7_viewer_performance_smoke_report_only() {
  local status=0
  run ./scripts/viewer-performance-probe.sh --profile smoke || status=$?
  if [[ "$status" -ne 0 ]]; then
    echo "warning: viewer performance smoke failed (report-only)"
    return 0
  fi
}

run_hosted_account_local_smoke() {
  run bash ./scripts/hosted-account-staging-smoke.sh --mode local
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
  run ./scripts/lint-skills.sh
  run ./scripts/check-windows-paths.sh
  run bash ./scripts/check-script-executable-bits.sh
  run ./scripts/cargo-dev-lib.test.sh
  run ./scripts/plan-rust-required-scope.test.sh
  run ./scripts/unified-world-code-terminology-scan.test.sh
  run ./scripts/release-gate-bash-preflight.test.sh
  run_provider_remote_https_smoke
  run_required_component "provider bridge live gate" "${OASIS7_CI_RUN_PROVIDER_LIVE_GATE:-false}" "explicit_opt_in_not_enabled" run_provider_bridge_live_gate
  run_newapi_bridge_service_accounting_tests
  run ./scripts/check-rust-file-size.sh
  run env -u RUSTC_WRAPPER cargo fmt --all -- --check
  run_rustsec_advisory_check
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
  run_oasis7_workspace_support_crate_tests
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
    run_required_component "oasis7 required tests" "${OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:-}" "disabled_by_scope_planner" run_oasis7_required_tier_tests
    run_required_component "oasis7_consensus tests" "${OASIS7_CI_RUN_CONSENSUS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_consensus_tests
    run_required_component "oasis7_distfs tests" "${OASIS7_CI_RUN_DISTFS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_distfs_tests
    run_required_component "oasis7_node tests" "${OASIS7_CI_RUN_OASIS7_NODE_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_node_tests
    run_required_component "oasis7_net tests" "${OASIS7_CI_RUN_OASIS7_NET_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_tests
    run_required_component "oasis7_net libp2p tests" "${OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_libp2p_tests
    run_required_component "viewer software-safe contract" "${OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_oasis7_viewer_software_safe_feedback_contract_tests
    run_required_component "viewer software-safe build" "${OASIS7_CI_RUN_VIEWER_WASM_CHECK:-}" "disabled_by_scope_planner" run_oasis7_viewer_software_safe_build
    run_required_component "viewer performance smoke (report-only)" "${OASIS7_CI_RUN_VIEWER_PERF_SMOKE:-false}" "report_only_scope_not_selected" run_oasis7_viewer_performance_smoke_report_only
    run_required_component "hosted account local smoke" "${OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE:-false}" "not_in_local_required_baseline_or_scope_disabled" run_hosted_account_local_smoke
    run_required_component "launcher web build" "${OASIS7_CI_RUN_LAUNCHER_WEB_BUILD:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_client_launcher_web_build
    run_required_component "oasis7 required clippy" "${OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:-}" "disabled_by_scope_planner" run_oasis7_required_tier_clippy
    run_required_component "oasis7_consensus clippy" "${OASIS7_CI_RUN_CONSENSUS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_consensus_clippy
    run_required_component "oasis7_distfs clippy" "${OASIS7_CI_RUN_DISTFS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_distfs_clippy
    run_required_component "oasis7_node clippy" "${OASIS7_CI_RUN_OASIS7_NODE_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_node_clippy
    run_required_component "oasis7_net clippy" "${OASIS7_CI_RUN_OASIS7_NET_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_clippy
    run_required_component "oasis7_net libp2p clippy" "${OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_libp2p_clippy
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
