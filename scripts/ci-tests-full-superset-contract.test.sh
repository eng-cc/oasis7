#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
ci_tests="$repo_root/scripts/ci-tests.sh"

full_tier_source="$(sed -n '/^run_full_required_superset() {/,/^}/p' "$ci_tests")"

require_full_check() {
  local expected="$1"
  if ! grep -Fqx "  $expected" <<<"$full_tier_source"; then
    echo "full tier must mechanically include required scoped check: $expected" >&2
    exit 1
  fi
}

require_full_check run_oasis7_required_tier_tests
require_full_check run_oasis7_consensus_tests
require_full_check run_oasis7_distfs_tests
require_full_check run_oasis7_node_tests
require_full_check run_oasis7_net_tests
require_full_check run_oasis7_net_libp2p_tests
require_full_check run_oasis7_viewer_software_safe_feedback_contract_tests
require_full_check run_oasis7_viewer_software_safe_build
require_full_check run_pixel_world_bridge_lib_tests
require_full_check run_pixel_world_bridge_wasm_check
require_full_check run_oasis7_client_launcher_web_build
require_full_check run_oasis7_workspace_support_crate_tests
require_full_check run_scenario_regression_tests
require_full_check run_site_contract_tests

full_capability_source="$(sed -n '/^run_all_required_gate_capability_contract_tests() {/,/^}/p' "$ci_tests")"
for capability_runner in \
  run_doc_checker_contract_tests \
  run_cargo_tooling_contract_tests \
  run_workflow_governance_contract_tests \
  run_packaging_contract_tests \
  run_operational_contract_tests; do
  if ! grep -Fq "$capability_runner" <<<"$full_capability_source"; then
    echo "full suite must ignore planner toggles and include capability contracts: $capability_runner" >&2
    exit 1
  fi
done

for full_tier_runner in run_full_required_superset run_full_core_tier_tests run_full_support_tier_tests; do
  tier_source="$(sed -n "/^${full_tier_runner}() {/,/^}/p" "$ci_tests")"
  if ! grep -Fq 'run_all_required_gate_capability_contract_tests' <<<"$tier_source"; then
    echo "${full_tier_runner} must retain all moved capability tests regardless of stale selectors" >&2
    exit 1
  fi
done

if grep -Fq 'run_provider_bridge_live_gate' <<<"$full_capability_source" || grep -Fq 'run_hosted_account_local_smoke' <<<"$full_capability_source"; then
  echo "full capability coverage must not implicitly enable manual-only provider/account actions" >&2
  exit 1
fi

echo "ci-tests full superset contract: passed"
