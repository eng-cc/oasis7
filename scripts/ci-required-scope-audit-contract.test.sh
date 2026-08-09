#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
planner="$repo_root/scripts/plan-rust-required-scope.sh"

value_for_key() {
  local output="$1"
  local key="$2"
  printf '%s\n' "$output" | awk -F= -v key="$key" '$1 == key {print substr($0, length(key) + 2)}'
}

require_key() {
  local output="$1"
  local key="$2"
  local expected="$3"
  local actual
  actual="$(value_for_key "$output" "$key")"
  if [[ "$actual" != "$expected" ]]; then
    echo "required-gate always-on check is not auditable in planner: expected $key=$expected, got $actual" >&2
    exit 1
  fi
}

require_reason_contains() {
  local output="$1"
  local expected="$2"
  local actual
  actual="$(value_for_key "$output" reason_summary)"
  if [[ "$actual" != *"$expected"* ]]; then
    echo "required-gate always-on check has no auditable planner reason: expected reason_summary to contain $expected, got $actual" >&2
    exit 1
  fi
}

minimal_plan="$("$planner" --event-name pull_request --changed-path doc/testing/prd.md)"
require_key "$minimal_plan" run_required_gate_baseline true
require_key "$minimal_plan" run_operational_contracts false
require_key "$minimal_plan" selected_capabilities required_gate_baseline
require_reason_contains "$minimal_plan" required_gate_baseline:always_on

operational_plan="$("$planner" --event-name pull_request --changed-path scripts/p2p-public-testnet-package-rollout.test.sh)"
require_key "$operational_plan" run_required_gate_baseline true
require_key "$operational_plan" run_operational_contracts true
require_key "$operational_plan" selected_capabilities operational_contracts
require_reason_contains "$operational_plan" operational_contracts:scripts/p2p-public-testnet-package-rollout.test.sh

workflow="$repo_root/.github/workflows/rust.yml"
for job in windows-package-rollout-behavior testnet-packages-macos-arm64-contract public-testnet-fleet-health-contract; do
  if ! awk -v job="$job" '
    $0 ~ "^  " job ":" { active=1; next }
    active && /^  [A-Za-z0-9_-]+:/ { exit }
    active && /if: github.event_name == .pull_request. && needs.required-gate.outputs.run_operational_contracts == .true./ { found=1 }
    END { exit(found ? 0 : 1) }
  ' "$workflow"; then
    echo "operational PR job is not planner-scoped: $job" >&2
    exit 1
  fi
done

echo "ci required scope audit contract: passed"
