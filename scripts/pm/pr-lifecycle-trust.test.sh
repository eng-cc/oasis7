#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Production CLI tests inject the shared HTTP transport in process. A fake `gh`
# executable no longer intercepts the lifecycle gate's direct API client.
python3 "$ROOT_DIR/scripts/pm/pr-lifecycle-trust-live.test.py"
echo "pr-lifecycle-trust.test: OK"
