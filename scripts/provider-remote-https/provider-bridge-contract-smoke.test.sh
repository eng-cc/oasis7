#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

python_bin="${PYTHON_BIN:-python3}"
"$python_bin" scripts/provider-remote-https/provider_bridge_contract_smoke.test.py
