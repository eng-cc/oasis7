#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

plan_for_path() {
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
    --event-name pull_request \
    --changed-path "$1"
}

plan_for_paths() {
  local args=(--event-name pull_request)
  local path
  for path in "$@"; do
    args+=(--changed-path "$path")
  done
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" "${args[@]}"
}

value_for_key() {
  local output="$1"
  local key="$2"
  printf '%s\n' "$output" | awk -F= -v key="$key" '$1 == key {print substr($0, length(key) + 2)}'
}

assert_key_equals() {
  local output="$1"
  local key="$2"
  local expected="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ "$actual" != "$expected" ]]; then
    echo "expected $key=$expected, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

assert_reason_contains() {
  local output="$1"
  local expected="$2"
  local actual
  actual="$(value_for_key "$output" reason_summary)"
  if [[ "$actual" != *"$expected"* ]]; then
    echo "expected reason_summary to contain $expected, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

assert_key_matches() {
  local output="$1"
  local key="$2"
  local pattern="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ ! "$actual" =~ $pattern ]]; then
    echo "expected $key to match $pattern, got $actual" >&2
    printf '%s\n' "$output" >&2
    exit 1
  fi
}

product_doc_output="$(plan_for_path doc/product/world-rules-core-gameplay.prd.md)"
assert_key_equals "$product_doc_output" scope minimal
assert_key_equals "$product_doc_output" run_rust_baseline false
assert_key_equals "$product_doc_output" needs_rust_toolchain false
assert_key_equals "$product_doc_output" needs_node false
assert_key_matches "$product_doc_output" planner_config_sha256 '^sha256:[0-9a-f]{64}$'
assert_reason_contains "$product_doc_output" "governance_doc:doc/product/world-rules-core-gameplay.prd.md"

launcher_output="$(plan_for_path crates/oasis7_client_launcher/src/lib.rs)"
assert_key_equals "$launcher_output" needs_node true
assert_key_equals "$launcher_output" needs_trunk true
assert_key_equals "$launcher_output" needs_rust_toolchain true

overlap_output="$(plan_for_paths crates/oasis7_node/src/network_bridge.rs crates/oasis7_net/src/lib.rs)"
assert_key_equals "$overlap_output" scope targeted
assert_key_equals "$overlap_output" run_oasis7_node_tests true
assert_key_equals "$overlap_output" run_oasis7_net_tests true
assert_key_equals "$overlap_output" run_oasis7_net_libp2p_tests true

rename_delete_output="$(plan_for_paths crates/oasis7_node/src/old_bridge.rs crates/oasis7_net/src/new_bridge.rs)"
assert_key_equals "$rename_delete_output" scope targeted
assert_key_equals "$rename_delete_output" run_oasis7_node_tests true
assert_key_equals "$rename_delete_output" run_oasis7_net_tests true

config_output="$(plan_for_path scripts/ci-required-scope.v1.json)"
assert_key_equals "$config_output" scope full
assert_key_equals "$config_output" run_rust_baseline true
assert_key_equals "$config_output" needs_rust_toolchain true
assert_key_equals "$config_output" needs_node true

unknown_output="$(plan_for_path unknown-unclassified-input.txt)"
assert_key_equals "$unknown_output" scope full
assert_reason_contains "$unknown_output" "unclassified_or_unresolvable:unknown-unclassified-input.txt"

invalid_config="$(mktemp)"
trap 'rm -f "$invalid_config"' EXIT
printf '{not json}\n' >"$invalid_config"
if "$ROOT_DIR/scripts/plan-rust-required-scope.sh" --event-name pull_request --config "$invalid_config" --changed-path README.md >"$invalid_config.out" 2>"$invalid_config.err"; then
  echo "expected invalid planner configuration to fail closed" >&2
  exit 1
fi
if ! grep -qi "config" "$invalid_config.err"; then
  echo "expected config validation failure, got:" >&2
  cat "$invalid_config.err" >&2
  exit 1
fi

wasm_build_output="$(plan_for_path crates/oasis7_wasm_build/src/lib.rs)"
assert_key_equals "$wasm_build_output" scope targeted
assert_key_equals "$wasm_build_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_build_output" run_launcher_web_build false
assert_reason_contains "$wasm_build_output" "wasm_support:crates/oasis7_wasm_build/src/lib.rs"

wasm_store_output="$(plan_for_path crates/oasis7_wasm_store/src/lib.rs)"
assert_key_equals "$wasm_store_output" scope targeted
assert_key_equals "$wasm_store_output" run_oasis7_required_tests true
assert_key_equals "$wasm_store_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_store_output" run_launcher_web_build false
assert_key_equals "$wasm_store_output" needs_system_deps true
assert_reason_contains "$wasm_store_output" "runtime_wasm_support:crates/oasis7_wasm_store/src/lib.rs"
assert_reason_contains "$wasm_store_output" "wasm_support:crates/oasis7_wasm_store/src/lib.rs"

wasm_router_output="$(plan_for_path crates/oasis7_wasm_router/src/lib.rs)"
assert_key_equals "$wasm_router_output" scope targeted
assert_key_equals "$wasm_router_output" run_oasis7_required_tests true
assert_key_equals "$wasm_router_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$wasm_router_output" "runtime_wasm_support:crates/oasis7_wasm_router/src/lib.rs"
assert_reason_contains "$wasm_router_output" "wasm_support:crates/oasis7_wasm_router/src/lib.rs"

wasm_executor_output="$(plan_for_path crates/oasis7_wasm_executor/src/lib.rs)"
assert_key_equals "$wasm_executor_output" scope targeted
assert_key_equals "$wasm_executor_output" run_oasis7_required_tests true
assert_key_equals "$wasm_executor_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$wasm_executor_output" "runtime_wasm_support:crates/oasis7_wasm_executor/src/lib.rs"
assert_reason_contains "$wasm_executor_output" "wasm_support:crates/oasis7_wasm_executor/src/lib.rs"

builtin_wasm_output="$(plan_for_path crates/oasis7_builtin_wasm_modules/src/lib.rs)"
assert_key_equals "$builtin_wasm_output" scope targeted
assert_key_equals "$builtin_wasm_output" run_oasis7_workspace_support_crate_tests true
assert_reason_contains "$builtin_wasm_output" "wasm_support:crates/oasis7_builtin_wasm_modules/src/lib.rs"

wasm_abi_output="$(plan_for_path crates/oasis7_wasm_abi/src/lib.rs)"
assert_key_equals "$wasm_abi_output" scope targeted
assert_key_equals "$wasm_abi_output" run_oasis7_workspace_support_crate_tests true
assert_key_equals "$wasm_abi_output" run_launcher_web_build true
assert_key_equals "$wasm_abi_output" needs_system_deps true
assert_key_equals "$wasm_abi_output" needs_wasm_target true
assert_reason_contains "$wasm_abi_output" "wasm_abi_support:crates/oasis7_wasm_abi/src/lib.rs"
assert_reason_contains "$wasm_abi_output" "launcher_wasm_abi:crates/oasis7_wasm_abi/src/lib.rs"

proto_output="$(plan_for_path crates/oasis7_proto/src/lib.rs)"
assert_key_equals "$proto_output" scope targeted
assert_key_equals "$proto_output" run_launcher_web_build true
assert_key_equals "$proto_output" needs_trunk true
assert_reason_contains "$proto_output" "launcher_proto:crates/oasis7_proto/src/lib.rs"

viewer_output="$(plan_for_path crates/oasis7_viewer/src/lib.rs)"
assert_key_equals "$viewer_output" scope targeted
assert_key_equals "$viewer_output" run_viewer_contract_tests true
assert_key_equals "$viewer_output" run_viewer_wasm_check true
assert_key_equals "$viewer_output" run_viewer_perf_smoke true
assert_key_equals "$viewer_output" needs_system_deps true
assert_reason_contains "$viewer_output" "viewer:crates/oasis7_viewer/src/lib.rs"

shared_required_output="$(plan_for_path .github/workflows/rust.yml)"
assert_key_equals "$shared_required_output" scope full
assert_key_equals "$shared_required_output" run_oasis7_required_tests true
assert_key_equals "$shared_required_output" run_launcher_web_build true
assert_key_equals "$shared_required_output" needs_system_deps true
assert_reason_contains "$shared_required_output" "shared_required_gate:.github/workflows/rust.yml"

node_output="$(plan_for_path crates/oasis7_node/src/network_bridge.rs)"
assert_key_equals "$node_output" scope targeted
assert_key_equals "$node_output" run_oasis7_node_tests true
assert_key_equals "$node_output" run_oasis7_net_tests false
assert_key_equals "$node_output" run_oasis7_net_libp2p_tests false
assert_reason_contains "$node_output" "node:crates/oasis7_node/src/network_bridge.rs"

net_output="$(plan_for_path crates/oasis7_net/src/lib.rs)"
assert_key_equals "$net_output" scope targeted
assert_key_equals "$net_output" run_oasis7_node_tests false
assert_key_equals "$net_output" run_oasis7_net_tests true
assert_key_equals "$net_output" run_oasis7_net_libp2p_tests true
assert_reason_contains "$net_output" "net:crates/oasis7_net/src/lib.rs"

echo "plan-rust-required-scope.test: OK"
