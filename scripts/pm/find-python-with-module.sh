#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]] || [[ ! "$1" =~ ^[A-Za-z_][A-Za-z0-9_.]*$ ]]; then
  echo "Usage: find-python-with-module.sh <python-module>" >&2
  exit 2
fi

MODULE="$1"

supports_module() {
  local candidate="$1"
  [[ -x "$candidate" ]] || return 1
  "$candidate" - "$MODULE" >/dev/null 2>&1 <<'PY'
import importlib
import sys

importlib.import_module(sys.argv[1])
PY
}

emit_if_supported() {
  local candidate="$1"
  if supports_module "$candidate"; then
    (cd "$(dirname "$candidate")" && printf '%s/%s\n' "$PWD" "$(basename "$candidate")")
    exit 0
  fi
}

for generic_name in python python3; do
  if candidate="$(command -v "$generic_name" 2>/dev/null)"; then
    emit_if_supported "$candidate"
  fi
done

while IFS= read -r path_dir; do
  [[ -n "$path_dir" ]] || path_dir="."
  for candidate in "$path_dir"/python*; do
    [[ -e "$candidate" ]] || continue
    emit_if_supported "$candidate"
  done
done < <(printf '%s' "$PATH" | tr ':' '\n')

echo "find-python-with-module: no Python interpreter on PATH can import ${MODULE}" >&2
exit 1
