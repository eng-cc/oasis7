#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

echo "warning: scripts/shared-devnet-blocker-packet.sh is a legacy wrapper; use scripts/public-testnet-rehearsal-blocker-packet.sh" >&2
exec ./scripts/public-testnet-rehearsal-blocker-packet.sh "$@"
