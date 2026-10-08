#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PYTHON_BIN="$("$ROOT_DIR/scripts/find-python-with-module.sh" json)"
exec "$PYTHON_BIN" "$ROOT_DIR/scripts/pr-review-threads.py" "$@"
