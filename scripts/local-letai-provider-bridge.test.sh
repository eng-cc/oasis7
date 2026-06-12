#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-local-letai-provider-test-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

config_path="$tmp_dir/letai.txt"
cat >"$config_path" <<'EOF'
Doc: https://docs.example.test/not-an-api-endpoint
Key: test-secret-key
EOF

check_config="$tmp_dir/check-config.txt"
./scripts/check-letai-chat-completions.sh \
  --config "$config_path" \
  --model test-model \
  --base-url https://api.example.test/v1 \
  --print-config \
  >"$check_config"

bridge_config="$tmp_dir/bridge-config.txt"
./scripts/run-local-letai-provider-bridge.sh \
  --config "$config_path" \
  --model test-model \
  --base-url https://api.example.test/v1 \
  --bind 127.0.0.1:5999 \
  --print-config \
  >"$bridge_config"

python3 - "$check_config" "$bridge_config" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

check_text = Path(sys.argv[1]).read_text()
bridge_text = Path(sys.argv[2]).read_text()

payload = json.loads(check_text)
assert payload["base_url"] == "https://api.example.test/v1"
assert payload["model"] == "test-model"
assert payload["api_key_present"] is True
assert payload["api_key_len"] == len("test-secret-key")
assert "test-secret-key" not in check_text
assert "test-secret-key" not in bridge_text
assert "bridge_bind=127.0.0.1:5999" in bridge_text
assert "auth_token_present=false" in bridge_text
PY

echo "local letai provider scripts config smoke passed"
