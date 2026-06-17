#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

# Retirement strategy: keep this compatibility entry only for historical
# automation discovery; it must not emit legacy manifest tiers or schemas.
# Remove after downstream automation/runbooks call public-testnet-rehearsal.
echo "warning: scripts/shared-devnet-rehearsal.sh is a legacy compatibility wrapper; use scripts/public-testnet-rehearsal.sh. It forwards without emitting legacy manifest tiers or output schemas." >&2
exec ./scripts/public-testnet-rehearsal.sh "$@"
