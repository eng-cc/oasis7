#!/usr/bin/env bash
set -euo pipefail

driver_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$driver_dir/.." && pwd)

tier="${1:-}"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ci-tests.sh [commit|required|full|full-core|full-support] [--repo-root PATH] [--group NAME]

  commit        Run the lightweight local commit gate used by pre-commit.
  required      Run the explicit heavier required gate for local validation and PR gate.
  full          Run required checks plus all extended feature/integration tests.
  full-core     Run doc/fmt plus the heaviest `oasis7 --tests` full-tier shard.
  full-support  Run the remaining support crates/viewer shard plus `oasis7 --lib --bins`.

Requires Python 3.11 or newer with the standard-library tomllib module.
The runner discovers a supported interpreter on PATH.

Default: none (explicit tier required)
USAGE
}

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

case "$tier" in
  commit|required|full|full-core|full-support) ;;
  *)
    usage
    exit 1
    ;;
esac

shift
group=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo-root)
      [[ $# -ge 2 && -n "$2" ]] || { usage; exit 1; }
      repo_root=$(cd "$2" && pwd)
      shift 2
      ;;
    --group)
      [[ $# -ge 2 && -n "$2" ]] || { usage; exit 1; }
      group="$2"
      shift 2
      ;;
    *)
      usage
      exit 1
      ;;
  esac
done

ci_python="$(bash "$driver_dir/find-python-with-module.sh" tomllib)"
cd "$repo_root"
# Keep sourced driver definitions with the driver, even when the tested tree differs.
source "$driver_dir/viewer-dependency-preflight.sh"

run() {
  if [[ "$1" == python3 ]]; then
    shift
    set -- "$ci_python" "$@"
  fi
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
    -D warnings
    -D clippy::correctness
    -D clippy::suspicious
  )
  if [[ "${CI_VERBOSE:-}" == "1" ]]; then
    run env -u RUSTC_WRAPPER cargo clippy --verbose "$@" -- "${lint_flags[@]}"
  else
    run env -u RUSTC_WRAPPER cargo clippy "$@" -- "${lint_flags[@]}"
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
    -p oasis7_client_api \
    -p oasis7_local_signer \
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
  # Native tool tests build the real template with the installed stable WASM target.
  OASIS7_WASM_BUILD_STD=0 run_cargo test -p wasm_build_suite -p wasm_module_observe
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

run_doc_checker_contract_tests() {
  run python3 ./scripts/document-corpus-inventory-check.test.py
  run python3 ./scripts/doc-evidence-inventory-check.test.py
  run python3 ./scripts/product-doc-governance-check.test.py
  run python3 ./scripts/product-doc-content-check.test.py
  run_system_design_traceability_tests
  run bash ./scripts/product-doc-content-callers.test.sh
  run bash ./scripts/doc-governance-check.test.sh
  run bash ./scripts/find-python-with-module.test.sh
}

run_cargo_tooling_baseline_contract_tests() {
  run bash ./scripts/cargo-dev-windows-toolchain.test.sh
  run bash ./scripts/cargo-dev-worktree-isolation.test.sh
  run bash ./scripts/new-task-worktree.test.sh
  run bash ./scripts/check-launcher-p2p-dependency-surface.test.sh
}

run_cargo_tooling_contract_tests() {
  run_cargo_tooling_baseline_contract_tests
  run bash ./scripts/cargo-dev-lib.test.sh
  run bash ./scripts/check-standalone-tool-lockfiles.test.sh
  run ./scripts/check-standalone-tool-lockfiles.sh
}

run_workflow_governance_baseline_contract_tests() {
  run python3 ./scripts/plan-rust-required-scope.test.py
  run python3 ./scripts/ci-required-result.test.py
  run python3 ./scripts/ci-workflow.test.py
  run bash ./scripts/rust-full-tier-trunk-prerequisite-contract.test.sh
  run python3 ./scripts/resource-cleanup-executor.test.py
  run python3 ./scripts/pr-review-threads.test.py
}

run_workflow_governance_operational_contract_tests() {
  run python3 ./scripts/security/codeql-plan.test.py
  run python3 ./scripts/security/codeql-workflow.test.py
  run python3 ./scripts/security/codeql-health.test.py
  run python3 ./scripts/security/codeql-upload-association.test.py
  run python3 ./scripts/security/codeql-acceptance.test.py
}

run_workflow_governance_contract_tests() {
  run_workflow_governance_baseline_contract_tests
  run_workflow_governance_operational_contract_tests
}

run_packaging_artifact_contract_tests() {
  run bash ./scripts/native-packaging-contract.test.sh
  run bash ./scripts/packaging-artifact-size-contract.test.sh
  run bash ./scripts/package-workflow-cache-reuse-contract.test.sh
  run bash ./scripts/copy-viewer-web-dist.test.sh
}

run_packaging_contract_tests() {
  run_packaging_artifact_contract_tests
  run bash ./scripts/release-packages-trunk-cache-contract.test.sh

}

run_operational_identity_contract_tests() {
  run python3 ./scripts/p2p-public-testnet-full-network-clean-room.test.py
  run python3 ./scripts/p2p-public-testnet-full-network-clean-room-adapter.test.py
  run python3 ./scripts/p2p-public-testnet-identity-v2-signing-tool.test.py
  run python3 ./scripts/p2p-public-testnet-identity-v2-cli-bridge.test.py
  run python3 ./scripts/p2p-public-testnet-identity-v2-evidence-aggregate.test.py
  run python3 ./scripts/p2p-public-testnet-peer-registry.test.py
}

run_operational_node_contract_tests() {
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
  run bash ./scripts/p2p-public-testnet-service-readback.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-health.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-order.test.sh
  run bash ./scripts/p2p-public-testnet-package-node-upgrade-rollback-contract.test.sh
  run bash ./scripts/p2p-observer-checkpoint-closure-probe.test.sh
  run python3 ./scripts/p2p-observer-checkpoint-closure-probe-safety.test.py
}

run_operational_contract_tests() {
  run_operational_identity_contract_tests
  run_operational_node_contract_tests
}


run_all_required_gate_capability_contract_tests() {
  run_doc_checker_contract_tests
  run_cargo_tooling_contract_tests
  run_workflow_governance_contract_tests
  run_packaging_contract_tests
  run_operational_contract_tests

  run_codex_agent_config_validation
  run_compile_metrics_contract_tests
  run bash ./scripts/viewer-performance-report-only-contract.test.sh
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
  run python3 scripts/tests/test_industrial_starter_evidence.py
  run ./scripts/copy-viewer-web-dist.test.sh
  run ./scripts/agent-browser-viewer-dist-freshness-test.sh
  run bash ./scripts/agent-browser-lifecycle-contract.test.sh
  run bash ./scripts/viewer-prompt-control-regression.test.sh
  run bash ./scripts/viewer-prompt-control-race.test.sh
  run bash ./scripts/viewer-prompt-control-race-identity.test.sh
  run node --test crates/oasis7_viewer/scripts/agent-browser-visual-runner-lifecycle.test.mjs
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
  run python3 ./scripts/validate-codex-agent-config.py
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

run_system_design_traceability_tests() {
  run python3 ./scripts/system-design-traceability-check.test.py
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
      run ./scripts/doc-governance-check.sh --full-corpus
  else
    run ./scripts/doc-governance-check.sh --full-corpus
  fi
}

run_standalone_tool_lockfiles_checks() {
  run bash ./scripts/check-standalone-tool-lockfiles.test.sh
  run ./scripts/check-standalone-tool-lockfiles.sh
}



run_required_gate_checks() {
  run ./scripts/unified-world-code-terminology-scan.test.sh
  run_product_doc_governance_check
  run ./scripts/lint-skills.sh
  run ./scripts/check-windows-paths.sh
  run bash ./scripts/check-script-executable-bits.sh
  if [[ -n "${OASIS7_PRODUCT_DOC_BASE:-}" && -n "${OASIS7_PRODUCT_DOC_HEAD:-}" ]]; then
    run git diff --check "$OASIS7_PRODUCT_DOC_BASE" "$OASIS7_PRODUCT_DOC_HEAD"
  else
    run git diff --check
  fi
  run python3 ./scripts/validate-codex-agent-config.py
}



run_commit_gate_checks() {
  run_required_gate_checks
  run_oasis7_consensus_tests
  run_oasis7_distfs_tests
  run_oasis7_viewer_software_safe_feedback_contract_tests
}

run_full_core_tier_tests() {
  run_required_gate_checks
  run_all_required_gate_capability_contract_tests
  run_oasis7_full_tier_tests
}

run_full_support_tier_tests() {
  run_all_required_gate_capability_contract_tests
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
  run bash ./scripts/testnet-packages-macos-arm64-contract.test.sh
  run_required_gate_checks
  run_all_required_gate_capability_contract_tests
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

run_rust_baseline() {
  run env -u RUSTC_WRAPPER cargo fmt --all -- --check
  run_rustsec_advisory_check
  run ./scripts/check-rust-file-size.test.sh
  run ./scripts/check-rust-file-size.sh
  run_newapi_bridge_service_accounting_tests
  run_standalone_tool_lockfiles_checks
}

run_group() {
  case "$1" in
    baseline) run_required_gate_checks ;;
    rust_baseline) run_rust_baseline ;;
    oasis7_required) run_oasis7_required_tier_tests; run_oasis7_required_tier_clippy; run_cargo test -p oasis7 --lib snapshot_progress::; run_cargo test -p oasis7 --lib snapshot_player_gameplay_execution_state_backfills_from_legacy_fields ;;
    consensus) run_oasis7_consensus_tests; run_oasis7_consensus_clippy ;;
    distfs) run_oasis7_distfs_tests; run_oasis7_distfs_clippy ;;
    node) run_oasis7_node_tests; run_oasis7_node_clippy ;;
    net) run_oasis7_net_tests; run_oasis7_net_libp2p_tests; run_oasis7_net_clippy; run_oasis7_net_libp2p_clippy ;;
    viewer_js_required) run_oasis7_viewer_software_safe_feedback_contract_tests; run_oasis7_viewer_software_safe_build ;;
    viewer_performance_report) run bash ./scripts/viewer-performance-report-only-contract.test.sh; run_oasis7_viewer_software_safe_build; run_oasis7_viewer_performance_smoke_report_only ;;
    pixel_world_bridge) run_pixel_world_bridge_lib_tests; run_pixel_world_bridge_wasm_check ;;
    launcher_web) run_oasis7_client_launcher_web_build ;;
    workspace_support) run_oasis7_workspace_support_crate_tests ;;
    scenario_regression) run_scenario_regression_tests ;;
    operational_contracts) run_operational_contract_tests ;;
    packaging_contracts) run_packaging_contract_tests ;;
    workflow_governance) run_workflow_governance_contract_tests ;;
    codex_agent_config_validation) run_codex_agent_config_validation ;;
    compile_metrics) run_compile_metrics_contract_tests ;;
    site_quality) run_site_contract_tests ;;
    doc_checker_contracts) run_doc_checker_contract_tests ;;
    cargo_tooling_contracts) run_cargo_tooling_contract_tests ;;
    *) echo "Unknown CI group: $1" >&2; return 2 ;;
  esac
}

echo "+ ci test tier: $tier"
if [[ -n "$group" ]]; then
  [[ "$tier" == required ]] || { echo '--group requires required tier' >&2; exit 2; }
  run_group "$group"
else
  case "$tier" in
    commit) run_commit_gate_checks ;;
    required)
      for group in baseline rust_baseline oasis7_required consensus distfs node net viewer_js_required pixel_world_bridge launcher_web workspace_support scenario_regression operational_contracts packaging_contracts workflow_governance codex_agent_config_validation compile_metrics site_quality doc_checker_contracts cargo_tooling_contracts; do run_group "$group"; done
      ;;
    full) run_full_required_superset; run_rust_baseline; run_oasis7_full_tier_tests; run_oasis7_llm_baseline_fixture_smoke; run_cargo test -p oasis7 --features wasmtime --lib --bins ;;
    full-core) run_full_core_tier_tests ;;
    full-support) run_full_support_tier_tests ;;
  esac
fi

if [[ "${OASIS7_CI_RUN_PROVIDER_LIVE_GATE:-false}" == true ]]; then run_provider_bridge_live_gate; fi
if [[ "${OASIS7_CI_RUN_HOSTED_ACCOUNT_SMOKE:-false}" == true ]]; then run_hosted_account_local_smoke; fi
