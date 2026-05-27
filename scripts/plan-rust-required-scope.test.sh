#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

plan_for_path() {
  "$ROOT_DIR/scripts/plan-rust-required-scope.sh" \
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
assert_key_equals "$wasm_build_output" scope full
assert_reason_contains "$wasm_build_output" "unclassified_code_or_ci:crates/oasis7_wasm_build/src/lib.rs"

wasm_store_output="$(plan_for_path crates/oasis7_wasm_store/src/lib.rs)"
assert_key_equals "$wasm_store_output" scope full
assert_reason_contains "$wasm_store_output" "unclassified_code_or_ci:crates/oasis7_wasm_store/src/lib.rs"

proto_output="$(plan_for_path crates/oasis7_proto/src/lib.rs)"
assert_key_equals "$proto_output" scope targeted
assert_key_equals "$proto_output" run_launcher_web_build true
assert_key_equals "$proto_output" needs_trunk true
assert_reason_contains "$proto_output" "launcher_proto:crates/oasis7_proto/src/lib.rs"

echo "plan-rust-required-scope.test: OK"
