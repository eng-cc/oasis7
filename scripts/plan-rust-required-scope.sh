#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
selector_python="$(bash "$SCRIPT_DIR/find-python-with-module.sh" tomllib)"
exec "$selector_python" "$SCRIPT_DIR/plan-rust-required-scope.py" "$@"
