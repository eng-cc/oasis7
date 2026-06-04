#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."
python3 scripts/provider-remote-https/provider_bridge_contract_smoke.py "$@"
