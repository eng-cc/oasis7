#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-letai-config-test-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

config_path="$tmp_dir/letai.txt"
cat >"$config_path" <<'EOF'
Doc: https://api.example.test/v1/chat/completions
Key: test-secret-key
EOF

sanitized_json="$tmp_dir/sanitized.json"
./scripts/with-letai-llm-config.sh \
  --config "$config_path" \
  --model test-model \
  --print-config \
  >"$sanitized_json"

python3 - "$sanitized_json" <<'PY'
import json
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
assert payload["base_url"] == "https://api.letai.run/v1"
assert payload["model"] == "test-model"
assert payload["doc_url_present"] is True
assert payload["api_key_present"] is True
assert payload["api_key_len"] == len("test-secret-key")
assert "test-secret-key" not in Path(sys.argv[1]).read_text()
PY

./scripts/with-letai-llm-config.sh \
  --config "$config_path" \
  --model test-model \
  -- \
  python3 - <<'PY'
import os

assert os.environ["OASIS7_LLM_BASE_URL"] == "https://api.letai.run/v1"
assert os.environ["OASIS7_LLM_API_KEY"] == "test-secret-key"
assert os.environ["OASIS7_LLM_MODEL"] == "test-model"
print("letai env loaded")
PY

config_with_base_url="$tmp_dir/letai-with-base.txt"
cat >"$config_with_base_url" <<'EOF'
base_url = https://api.override.test/v1
Key = test-secret-key
EOF

./scripts/with-letai-llm-config.sh \
  --config "$config_with_base_url" \
  --model test-model \
  -- \
  python3 - <<'PY'
import os

assert os.environ["OASIS7_LLM_BASE_URL"] == "https://api.override.test/v1"
assert os.environ["OASIS7_LLM_API_KEY"] == "test-secret-key"
assert os.environ["OASIS7_LLM_MODEL"] == "test-model"
print("letai explicit base loaded")
PY
