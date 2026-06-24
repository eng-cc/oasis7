#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

plan_for_path() {
  "$ROOT_DIR/scripts/plan-wasm-determinism-scope.sh" \
    --event-name pull_request \
    --changed-path "$1"
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

wasm_build_output="$(plan_for_path crates/oasis7_wasm_build/src/lib.rs)"
assert_key_equals "$wasm_build_output" scope all
assert_key_equals "$wasm_build_output" run_all true
assert_key_equals "$wasm_build_output" run_m1 true
assert_key_equals "$wasm_build_output" run_m4 true
assert_key_equals "$wasm_build_output" run_m5 true
assert_key_equals "$wasm_build_output" selected_module_sets m1,m4,m5
assert_reason_contains "$wasm_build_output" "shared_wasm_pipeline:crates/oasis7_wasm_build/src/lib.rs"

sdk_output="$(plan_for_path crates/oasis7_wasm_sdk/src/lib.rs)"
assert_key_equals "$sdk_output" scope all
assert_key_equals "$sdk_output" run_all true
assert_reason_contains "$sdk_output" "shared_wasm_pipeline:crates/oasis7_wasm_sdk/src/lib.rs"

unrelated_output="$(plan_for_path doc/engineering/project.md)"
assert_key_equals "$unrelated_output" scope skip
assert_key_equals "$unrelated_output" run_all false
assert_reason_contains "$unrelated_output" "no_builtin_wasm_inputs_changed"

echo "plan-wasm-determinism-scope.test: OK"
