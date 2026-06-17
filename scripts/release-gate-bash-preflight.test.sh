#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

assert_guard_precedes_body() {
  local script="$1"
  local expected_reason="$2"
  local expected_hint="$3"
  python3 - "$script" "$expected_reason" "$expected_hint" <<'PY'
from __future__ import annotations

import pathlib
import sys

script = pathlib.Path(sys.argv[1])
expected_reason = sys.argv[2]
expected_hint = sys.argv[3]
lines = script.read_text(encoding="utf-8").splitlines()

def find(fragment: str) -> int:
    for index, line in enumerate(lines):
        if fragment in line:
            return index
    raise SystemExit(f"{script}: missing {fragment!r}")

guard_index = find("BASH_VERSINFO[0] < 4")
requirement_index = find("requires Bash 4+")
reason_index = find(expected_reason)
hint_index = find(expected_hint)
exit_index = find("exit 2")
body_candidates = [
    index
    for index, line in enumerate(lines)
    for stripped in [line.lstrip()]
    if (
        stripped.startswith("source ")
        or stripped.startswith("declare -A")
        or stripped.startswith("mapfile ")
        or stripped.startswith("usage()")
    )
]
if not body_candidates:
    raise SystemExit(f"{script}: missing body marker")
first_body_index = min(body_candidates)
for label, index in {
    "guard": guard_index,
    "requirement": requirement_index,
    "reason": reason_index,
    "hint": hint_index,
    "exit": exit_index,
}.items():
    if index >= first_body_index:
        raise SystemExit(
            f"{script}: {label} line must appear before first body marker "
            f"(line {index + 1} >= {first_body_index + 1})"
        )
PY
}

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

assert_guard_precedes_body \
  "scripts/p2p-longrun-soak.sh" \
  "uses mapfile and associative arrays" \
  "before release-gate longrun execution"
assert_preflight_for_bash3 \
  "scripts/p2p-longrun-soak.sh" \
  "uses mapfile and associative arrays" \
  "before release-gate longrun execution"

assert_guard_precedes_body \
  "scripts/s10-five-node-game-soak.sh" \
  "uses associative arrays" \
  "before release-gate longrun execution"
assert_preflight_for_bash3 \
  "scripts/s10-five-node-game-soak.sh" \
  "uses associative arrays" \
  "before release-gate longrun execution"

assert_guard_precedes_body \
  "scripts/module-release-node-acceptance.sh" \
  "uses associative arrays" \
  "before module release acceptance execution"
assert_preflight_for_bash3 \
  "scripts/module-release-node-acceptance.sh" \
  "uses associative arrays" \
  "before module release acceptance execution"

echo "release-gate-bash-preflight.test: OK"
