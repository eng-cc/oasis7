#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CONTRACT="$ROOT_DIR/scripts/launcher-help-contract.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

if [[ ! -f "$CONTRACT" ]]; then
  echo "launcher-help-contract: missing canonical shared-help producer: $CONTRACT" >&2
  exit 1
fi

shared_help="$TMP_DIR/shared-help.txt"
bash "$CONTRACT" shared >"$shared_help"
if [[ ! -s "$shared_help" ]]; then
  echo "launcher-help-contract: canonical shared help is empty" >&2
  exit 1
fi

assert_shared_help() {
  local name=$1
  local script=$2
  local actual="$TMP_DIR/$name.actual"

  "$ROOT_DIR/$script" --help >"$actual" 2>&1
  python3 - "$shared_help" "$actual" "$name" <<'PY'
from pathlib import Path
import sys

shared = Path(sys.argv[1]).read_text(encoding="utf-8").strip()
actual = Path(sys.argv[2]).read_text(encoding="utf-8")
name = sys.argv[3]
if shared not in actual:
    raise SystemExit(f"{name} help does not contain canonical shared guidance")
PY
}

assert_shared_help worktree-harness scripts/worktree-harness.sh
assert_shared_help run-producer-playtest scripts/run-producer-playtest.sh
assert_shared_help run-launcher-stack scripts/run-launcher-stack.sh

echo "launcher help contract: PASS"
