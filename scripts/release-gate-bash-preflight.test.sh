#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

assert_preflight_for_bash3() {
  local script="$1"
  local expected_reason="$2"
  local expected_hint="$3"
  local status=0
  local output

  output="$(bash "$script" --help 2>&1)" || status=$?
  if (( BASH_VERSINFO[0] < 4 )); then
    if [[ "$status" -ne 2 ]]; then
      echo "expected $script to exit 2 under Bash ${BASH_VERSION}, got $status" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
    if [[ "$output" != *"$script requires Bash 4+"* ]]; then
      echo "expected $script to report Bash 4+ requirement" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
    if [[ "$output" != *"$expected_reason"* ]]; then
      echo "expected $script to report reason: $expected_reason" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
    if [[ "$output" != *"$expected_hint"* ]]; then
      echo "expected $script to report hint: $expected_hint" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
    if [[ "$output" == *"mapfile: command not found"* || "$output" == *"declare: -A"* ]]; then
      echo "expected $script to fail before raw Bash 4-only syntax errors" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
  else
    if [[ "$status" -ne 0 ]]; then
      echo "expected $script --help to pass under Bash ${BASH_VERSION}, got $status" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
    if [[ "$output" != *"Usage:"* ]]; then
      echo "expected $script --help to print usage under Bash ${BASH_VERSION}" >&2
      printf '%s\n' "$output" >&2
      exit 1
    fi
  fi
}

assert_preflight_for_bash3 \
  "scripts/p2p-longrun-soak.sh" \
  "uses mapfile and associative arrays" \
  "before release-gate longrun execution"

assert_preflight_for_bash3 \
  "scripts/s10-five-node-game-soak.sh" \
  "uses associative arrays" \
  "before release-gate longrun execution"

assert_preflight_for_bash3 \
  "scripts/module-release-node-acceptance.sh" \
  "uses associative arrays" \
  "before module release acceptance execution"

echo "release-gate-bash-preflight.test: OK"
