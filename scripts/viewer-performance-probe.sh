#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root/crates/oasis7_viewer"

exec node ./scripts/viewer-performance-probe.mjs "$@"
