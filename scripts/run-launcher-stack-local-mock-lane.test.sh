#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-run-launcher-stack-local-mock-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

config_json="$tmp_dir/provider-config.json"

./scripts/run-launcher-stack.sh \
  --deployment-mode trusted_local_only \
  --allow-trusted-local-playtest \
  --agent-provider-lane local-mock \
  --print-agent-provider-config \
  >"$config_json"

python3 - "$config_json" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
assert payload["deployment_mode"] == "trusted_local_only"
assert payload["allow_trusted_local_playtest"] == "1"
assert payload["agent_decision_source"] == "provider_backed"
assert payload["agent_provider_lane"] == "local-mock"
assert payload["agent_provider_backend"] == "provider_local_mock"
assert payload["agent_provider_contract"] == "worldsim_provider_v1"
assert payload["agent_provider_transport"] == "loopback_http"
assert payload["agent_provider_url"] == "http://127.0.0.1:5841"
assert payload["agent_provider_profile"] == "oasis7_p0_low_freq_npc"
PY
