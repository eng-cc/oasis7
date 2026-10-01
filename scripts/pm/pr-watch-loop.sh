#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PR="${1:?usage: pr-watch-loop.sh <pr> --task-uid <task_uid>}"; shift
exec python3 "$SCRIPT_DIR/pr-lifecycle-gate.py" "$PR" "$@" --observe --watch --json
