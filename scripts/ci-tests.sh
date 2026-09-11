#!/usr/bin/env bash
set -euo pipefail

driver_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$driver_dir/.." && pwd)

tier="${1:-}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-tests.sh [commit|required|full|full-core|full-support] [--repo-root PATH]

  commit        Run the lightweight local commit gate used by pre-commit.
  required      Run the explicit heavier required gate for local validation and PR gate.
  full          Run required checks plus all extended feature/integration tests.
  full-core     Run doc/fmt plus the heaviest `oasis7 --tests` full-tier shard.
  full-support  Run the remaining support crates/viewer shard plus `oasis7 --lib --bins`.

Default: none (explicit tier required)
USAGE
}

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

if [[ $# -ne 1 && !( $# -eq 3 && "$2" == "--repo-root" && -n "$3" ) ]]; then
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

if [[ $# -eq 3 ]]; then
  repo_root=$(cd "$3" && pwd)
fi
cd "$repo_root"
# Keep sourced driver definitions with the driver, even when the tested tree differs.
source "$driver_dir/viewer-dependency-preflight.sh"

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

run_scenario_regression_tests() {
  run_cargo test -p oasis7 --test oasis7_init_demo --features test_tier_full oasis7_init_demo_runs_
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
  run ./scripts/check-rustsec-ignore-baseline.sh
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

run_packaging_contract_tests() {
  run bash ./scripts/native-packaging-contract.test.sh
  run bash ./scripts/packaging-artifact-size-contract.test.sh
  run bash ./scripts/copy-viewer-web-dist.test.sh
}

run_operational_contract_tests() {
  run_packaging_contract_tests
  run python3 ./scripts/pm/ci-ready-receipt.test.py
  run python3 ./scripts/pm/review-plan.test.py
  run python3 ./scripts/pm/subagent-task-packet.test.py
  run python3 ./scripts/pm/bootstrap-task-snapshot.test.py
  run python3 ./scripts/pm/integration-ci.test.py
  run python3 ./scripts/pm/integration-selection-regression.test.py
  run python3 ./scripts/pm/workflow-bootstrap-fallback.test.py
  run python3 ./scripts/pm/loop-policy.test.py
  run python3 ./scripts/pm/loop-contracts.test.py
  run python3 ./scripts/pm/loop-traceability.test.py
  run python3 ./scripts/pm/loop_terminal.test.py
  run python3 ./scripts/pm/loop.test.py
  run python3 ./scripts/pm/loop-gate.test.py
  run python3 ./scripts/pm/loop-ci.test.py
  run python3 ./scripts/pm/loop-ci-content.test.py
  run python3 ./scripts/pm/pr-lifecycle-loop.test.py
  run python3 ./scripts/pm/loop-ingress.test.py
  run env PYTHONDONTWRITEBYTECODE=1 python3 ./scripts/pm/loop-publication.integration.test.py
  run python3 ./scripts/pm/loop-recovery.test.py
  run python3 ./scripts/pm/github-project-loop.test.py
  run python3 ./scripts/pm/github-project-admission.test.py
  run python3 ./scripts/pm/loop-bootstrap.test.py
  run env PYTHONDONTWRITEBYTECODE=1 python3 ./scripts/pm/loop-bootstrap.integration.test.py
  run ./scripts/ci-required-scope-audit-contract.test.sh
  run ./scripts/game-world-state-sync-commit-module-required.test.sh
  run ./scripts/state-sync-closure-evidence-template.test.sh
  run ./scripts/s10-five-node-game-soak-summary.test.sh
  run ./scripts/release-gate-bash-preflight.test.sh
  run bash ./scripts/p2p-public-testnet-local-observer-sync.test.sh
  run bash ./scripts/build-game-launcher-bundle-ops-default.test.sh
  run bash ./scripts/build-game-launcher-bundle-macos-bash3.test.sh
  run bash ./scripts/testnet-packages-linux-bundle-bootstrap-contract.test.sh
  run bash ./scripts/testnet-packages-windows-governed-closure.test.sh
  run_provider_remote_https_smoke
  run bash ./scripts/p2p-public-testnet-bootstrap-fresh-validator-host.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-health.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-order.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-rollback-contract.test.sh
  run bash ./scripts/p2p-observer-checkpoint-closure-probe.test.sh
  run python3 ./scripts/p2p-observer-checkpoint-closure-probe-safety.test.py
}

run_provider_bridge_live_gate() {
  run ./scripts/provider-remote-https/provider-bridge-live-gate.sh
}

run_newapi_bridge_service_accounting_tests() {
  run env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture
}

run_oasis7_viewer_software_safe_feedback_contract_tests() {
  run ./scripts/viewer-dependency-preflight.test.sh
  viewer_dependency_preflight "$repo_root" test
  run npm --prefix crates/oasis7_viewer run test:frontend-structure
  run npm --prefix crates/oasis7_viewer run test:feedback-contract
  run node crates/oasis7_viewer/scripts/gameplay-attraction-scenario.test.mjs
  run ./scripts/copy-viewer-web-dist.test.sh
  run ./scripts/agent-browser-viewer-dist-freshness-test.sh
  run ./scripts/bundle-freshness-lib.test.sh
  run npm --prefix crates/oasis7_viewer run test:ui
}

run_oasis7_viewer_software_safe_build() {
  run ./scripts/build-viewer-software-safe.sh
}

run_pixel_world_bridge_lib_tests() {
  run_cargo test -p pixel_world_bridge --lib
}

run_pixel_world_bridge_wasm_check() {
  run_cargo check -p pixel_world_bridge --target wasm32-unknown-unknown
}

run_oasis7_viewer_performance_smoke_report_only() {
  run ./scripts/viewer-performance-report-only.sh
}

run_hosted_account_local_smoke() {
  run bash ./scripts/hosted-account-staging-smoke.sh --mode local
}

run_oasis7_client_launcher_web_build() {
  run ./scripts/worktree-harness-lifecycle.test.sh
  run ./scripts/worktree-harness-lifecycle-races.test.sh
  run ./scripts/worktree-harness-contract.test.sh
  run ./scripts/launcher-help-contract.test.sh
  run mkdir -p output/release/web-launcher-dist
  (
    cd crates/oasis7_client_launcher
    run env -u NO_COLOR trunk build --release --dist ../../output/release/web-launcher-dist
  )
}

run_codex_agent_config_validation() {
  run ./scripts/pm/validate-codex-agent-config.test.sh
  run ./scripts/pm/codex-role-fit-task-binding.test.sh
}

run_compile_metrics_contract_tests() {
  run bash ./scripts/ci-compile-metrics-contract.test.sh
}

run_site_contract_tests() {
  run ./scripts/site-link-check.sh
  run ./scripts/site-homepage-claim-check.sh
  run ./scripts/site-manual-sync-check.sh
  run ./scripts/site-download-check.sh
  run bash ./scripts/site-checks-contract.test.sh
}

product_doc_range() {
  local base_oid="${OASIS7_PRODUCT_DOC_BASE:-}"
  local head_oid="${OASIS7_PRODUCT_DOC_HEAD:-}"
  if [[ -n "$base_oid" || -n "$head_oid" ]]; then
    [[ -n "$base_oid" && -n "$head_oid" ]] || {
      echo "product-doc-content: explicit base/head must be supplied together" >&2
      return 1
    }
    printf '%s\n%s\n' "$base_oid" "$head_oid"
    return 0
  fi
  if [[ -n "${GITHUB_EVENT_PATH:-}" ]]; then
    if [[ ! -f "$GITHUB_EVENT_PATH" ]]; then
      echo "product-doc-content: CI event path is missing or unreadable" >&2
      return 1
    fi
    python3 - "$GITHUB_EVENT_PATH" "${GITHUB_EVENT_NAME:-}" "${GITHUB_SHA:-}" <<'PY'
import json
import re
import sys

event_path, event_name, default_head = sys.argv[1:]
try:
    with open(event_path, encoding="utf-8") as handle:
        payload = json.load(handle)
except (OSError, ValueError) as exc:
    raise SystemExit(f"product-doc-content: malformed CI event: {exc}")
if event_name == "pull_request":
    base = ((payload.get("pull_request") or {}).get("base") or {}).get("sha")
    head = ((payload.get("pull_request") or {}).get("head") or {}).get("sha")
elif event_name == "push":
    base = payload.get("before")
    head = payload.get("after")
elif event_name == "workflow_dispatch":
    inputs = payload.get("inputs") or {}
    base = inputs.get("integration_base")
    head = inputs.get("expected_head")
else:
    raise SystemExit(f"product-doc-content: unsupported CI event range: {event_name or '<empty>'}")
if not isinstance(base, str) or not isinstance(head, str) or not base or not head:
    raise SystemExit("product-doc-content: CI event did not provide both base/head OIDs")
if not re.fullmatch(r"[0-9a-fA-F]{40}", base) or not re.fullmatch(r"[0-9a-fA-F]{40}", head):
    raise SystemExit("product-doc-content: CI event base/head must be full 40-character OIDs")
print(base)
print(head)
PY
    return $?
  fi
  if [[ "${CI:-}" == "true" || "${GITHUB_ACTIONS:-}" == "true" ]]; then
    echo "product-doc-content: CI requires explicit base/head OIDs or a valid event payload" >&2
    return 1
  fi
}

run_product_doc_governance_check() {
  local range
  range="$(product_doc_range)" || return 1
  if [[ -n "$range" ]]; then
    local base_oid head_oid
    base_oid="$(printf '%s\n' "$range" | sed -n '1p')"
    head_oid="$(printf '%s\n' "$range" | sed -n '2p')"
    [[ -n "$base_oid" && -n "$head_oid" ]] || {
      echo "product-doc-content: event did not provide explicit base/head" >&2
      return 1
    }
    OASIS7_PRODUCT_DOC_BASE="$base_oid" OASIS7_PRODUCT_DOC_HEAD="$head_oid" \
      run ./scripts/doc-governance-check.sh
  else
    run ./scripts/doc-governance-check.sh
  fi
}

run_standalone_tool_lockfiles_checks() {
  run bash ./scripts/check-standalone-tool-lockfiles.test.sh
  run ./scripts/check-standalone-tool-lockfiles.sh
}

run_required_gate_checks() {
  run_product_doc_governance_check
  run python3 ./scripts/product-doc-governance-check.test.py
  run python3 ./scripts/product-doc-content-check.test.py
  run bash ./scripts/product-doc-content-callers.test.sh
  run ./scripts/lint-skills.sh
  run ./scripts/check-windows-paths.sh
  run bash ./scripts/check-script-executable-bits.sh
  run bash ./scripts/cargo-dev-windows-toolchain.test.sh
  run bash ./scripts/doc-governance-check.test.sh
  run bash ./scripts/testing-manual-active-contract.test.sh
  run bash ./scripts/ci-tests-argument-contract.test.sh
  run bash ./scripts/ci-tests-full-superset-contract.test.sh
  run bash ./scripts/ci-tests-pixel-world-required-contract.test.sh
  run bash ./scripts/ci-tests-codex-agent-config-required-contract.test.sh
  run_required_component "compile metrics contract" "${OASIS7_CI_RUN_COMPILE_METRICS_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_compile_metrics_contract_tests
  run bash ./scripts/release-packages-trunk-cache-contract.test.sh
  run bash ./scripts/rust-required-gate-apt-contract.test.sh
  run bash ./scripts/viewer-performance-report-only-contract.test.sh
  run bash ./scripts/pm/find-python-with-module.test.sh
  run_required_component "standalone tool lockfiles" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run_standalone_tool_lockfiles_checks
  run bash ./scripts/check-launcher-p2p-dependency-surface.test.sh
  run ./scripts/plan-rust-required-scope.test.sh
  run ./scripts/rust-required-gate-compile-command-contract.test.sh
  run bash ./scripts/rust-full-tier-trunk-prerequisite-contract.test.sh
  run ./scripts/unified-world-code-terminology-scan.test.sh
  run_required_component "operational contracts" "${OASIS7_CI_RUN_OPERATIONAL_CONTRACTS:-}" "disabled_by_scope_planner" run_operational_contract_tests
  run_required_component "provider bridge live gate" "${OASIS7_CI_RUN_PROVIDER_LIVE_GATE:-false}" "explicit_opt_in_not_enabled" run_provider_bridge_live_gate
  run_required_component "cargo-dev library contract" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run ./scripts/cargo-dev-lib.test.sh
  run_required_component "newapi bridge Rust baseline" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run_newapi_bridge_service_accounting_tests
  run ./scripts/check-rust-file-size.test.sh
  run ./scripts/check-rust-file-size.sh
  run_required_component "cargo fmt" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run env -u RUSTC_WRAPPER cargo fmt --all -- --check
  run_required_component "RustSec advisory check" "${OASIS7_CI_RUN_RUST_BASELINE:-}" "disabled_by_scope_planner" run_rustsec_advisory_check
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

run_full_required_superset() {
  run_required_gate_checks
  run_site_contract_tests
  run_oasis7_required_tier_tests
  run_scenario_regression_tests
  run_oasis7_consensus_tests
  run_oasis7_distfs_tests
  run_oasis7_node_tests
  run_oasis7_net_tests
  run_oasis7_net_libp2p_tests
  run_oasis7_viewer_software_safe_feedback_contract_tests
  run_oasis7_viewer_software_safe_build
  run_pixel_world_bridge_lib_tests
  run_pixel_world_bridge_wasm_check
  run_oasis7_client_launcher_web_build
  run_oasis7_workspace_support_crate_tests
}

echo "+ ci test tier: $tier"
case "$tier" in
  commit)
    run_commit_gate_checks
    ;;
  required)
    run_required_gate_checks
    run_required_component "oasis7 required tests" "${OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:-}" "disabled_by_scope_planner" run_oasis7_required_tier_tests
    run_required_component "scenario regression" "${OASIS7_CI_RUN_SCENARIO_REGRESSION:-}" "disabled_by_scope_planner" run_scenario_regression_tests
    run_required_component "oasis7_consensus tests" "${OASIS7_CI_RUN_CONSENSUS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_consensus_tests
    run_required_component "oasis7_distfs tests" "${OASIS7_CI_RUN_DISTFS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_distfs_tests
    run_required_component "oasis7_node tests" "${OASIS7_CI_RUN_OASIS7_NODE_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_node_tests
    run_required_component "oasis7_net tests" "${OASIS7_CI_RUN_OASIS7_NET_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_tests
    run_required_component "oasis7_net libp2p tests" "${OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_libp2p_tests
    run_required_component "viewer software-safe contract" "${OASIS7_CI_RUN_VIEWER_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_oasis7_viewer_software_safe_feedback_contract_tests
    run_required_component "viewer software-safe build" "${OASIS7_CI_RUN_VIEWER_WASM_CHECK:-}" "disabled_by_scope_planner" run_oasis7_viewer_software_safe_build
    run_required_component "pixel world bridge lib tests" "${OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_LIB_TESTS:-}" "disabled_by_scope_planner" run_pixel_world_bridge_lib_tests
    run_required_component "pixel world bridge wasm check" "${OASIS7_CI_RUN_PIXEL_WORLD_BRIDGE_WASM_CHECK:-}" "disabled_by_scope_planner" run_pixel_world_bridge_wasm_check
    run_required_component "viewer performance smoke (report-only)" "${OASIS7_CI_RUN_VIEWER_PERF_SMOKE:-false}" "report_only_scope_not_selected" run_oasis7_viewer_performance_smoke_report_only
    run_required_component "hosted account local smoke" "${OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE:-false}" "not_in_local_required_baseline_or_scope_disabled" run_hosted_account_local_smoke
    run_required_component "launcher web build" "${OASIS7_CI_RUN_LAUNCHER_WEB_BUILD:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_client_launcher_web_build
    run_required_component "workspace support crate tests" "${OASIS7_CI_RUN_WORKSPACE_SUPPORT_CRATE_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_workspace_support_crate_tests
    run_required_component "Codex agent-config validation" "${OASIS7_CI_RUN_CODEX_AGENT_CONFIG_VALIDATION:-}" "disabled_by_scope_planner" run_codex_agent_config_validation
    run_required_component "site quality contracts" "${OASIS7_CI_RUN_SITE_CONTRACT_TESTS:-}" "disabled_by_scope_planner" run_site_contract_tests
    run_required_component "oasis7 required clippy" "${OASIS7_CI_RUN_OASIS7_REQUIRED_TESTS:-}" "disabled_by_scope_planner" run_oasis7_required_tier_clippy
    run_required_component "oasis7_consensus clippy" "${OASIS7_CI_RUN_CONSENSUS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_consensus_clippy
    run_required_component "oasis7_distfs clippy" "${OASIS7_CI_RUN_DISTFS_TESTS:-}" "disabled_by_scope_planner" run_oasis7_distfs_clippy
    run_required_component "oasis7_node clippy" "${OASIS7_CI_RUN_OASIS7_NODE_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_node_clippy
    run_required_component "oasis7_net clippy" "${OASIS7_CI_RUN_OASIS7_NET_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_clippy
    run_required_component "oasis7_net libp2p clippy" "${OASIS7_CI_RUN_OASIS7_NET_LIBP2P_TESTS:-false}" "not_in_local_required_baseline_or_scope_disabled" run_oasis7_net_libp2p_clippy
    ;;
  full)
    run_full_required_superset
    run_oasis7_full_tier_tests
    run_oasis7_llm_baseline_fixture_smoke
    run_cargo test -p oasis7 --features wasmtime --lib --bins
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
