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

echo "ci-tests full superset contract: passed"
