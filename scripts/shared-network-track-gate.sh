#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

echo "warning: scripts/shared-network-track-gate.sh is a legacy wrapper; use scripts/network-rehearsal-track-gate.sh" >&2
exec ./scripts/network-rehearsal-track-gate.sh "$@"
