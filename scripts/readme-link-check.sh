#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

fail_count=0

extract_refs() {
  local md_file="$1"
  grep -Eo '\[[^]]+\]\(([^)]+)\)' "$md_file" | sed -E 's/^.*\(([^)]+)\)$/\1/' || true
}

check_file() {
  local md_file="$1"
  while IFS= read -r ref; do
    [[ -z "$ref" ]] && continue
    local clean="${ref%%#*}"
    clean="${clean%%\?*}"
    [[ -z "$clean" ]] && continue
    case "$clean" in
      http:*|https:*|mailto:*|tel:*)
        continue
        ;;
      esac
    local target
    target="$(python3 - "$(dirname "$md_file")/$clean" <<'PY'
from __future__ import annotations

from pathlib import Path
import sys

print(Path(sys.argv[1]).resolve(strict=False))
PY
)"
    if [[ ! -e "$target" ]]; then
      echo "error: broken README reference in $md_file: $ref -> $target" >&2
      fail_count=$((fail_count + 1))
    fi
  done < <(extract_refs "$md_file")
}

check_file "$REPO_ROOT/README.md"
check_file "$REPO_ROOT/doc/README.md"

if [[ "$fail_count" -gt 0 ]]; then
  echo "error: readme link check failed with ${fail_count} broken reference(s)" >&2
  exit 1
fi

echo "ok: README.md and doc/README.md local markdown references are valid"
