#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

tmp_dir="${TMPDIR:-/tmp}/oasis7-letai-mapping-test-$$"
mkdir -p "$tmp_dir"
trap 'rm -rf "$tmp_dir"' EXIT

selected_config="$tmp_dir/selected.env"
cat >"$selected_config" <<'EOF'
base_url = https://api.example.test/v1
token_key = selected-project-token
model = test-model
EOF

mapping_config="$tmp_dir/authoritative.env"
cat >"$mapping_config" <<'EOF'
base_url = https://api.example.test/v1
token_key = authoritative-project-token
platform_user_id = user-123
platform_project_id = project-123
EOF

mismatch_stderr="$tmp_dir/mismatch.stderr"
set +e
./scripts/ensure-letai-local-token-config.sh \
  --config "$selected_config" \
  --authoritative-mapping "$mapping_config" \
  --out "$tmp_dir/mismatch.out" \
  --model test-model \
  --chat-base-url https://api.example.test/v1 \
  --platform-base-url https://api.example.test \
  >"$tmp_dir/mismatch.stdout" 2>"$mismatch_stderr"
mismatch_status=$?
set -e

if [[ "$mismatch_status" -eq 0 ]]; then
  echo "authoritative mapping mismatch must fail closed" >&2
  exit 1
fi
if ! grep -Fq "authoritative project token mapping mismatch" "$mismatch_stderr"; then
  echo "mismatch must report the bounded mapping error" >&2
  cat "$mismatch_stderr" >&2
  exit 1
fi
if grep -Fq "selected-project-token" "$mismatch_stderr" || grep -Fq "authoritative-project-token" "$mismatch_stderr"; then
  echo "mismatch error must not print token values" >&2
  exit 1
fi

matching_config="$tmp_dir/matching.env"
cat >"$matching_config" <<'EOF'
base_url = https://api.example.test/v1
token_key = authoritative-project-token
model = test-model
EOF

matching_out="$tmp_dir/matching.out"
matching_json="$tmp_dir/matching.json"
./scripts/ensure-letai-local-token-config.sh \
  --config "$matching_config" \
  --authoritative-mapping "$mapping_config" \
  --out "$matching_out" \
  --model test-model \
  --chat-base-url https://api.example.test/v1 \
  --platform-base-url https://api.example.test \
  >"$matching_json"

python3 - "$matching_json" "$matching_out" <<'PY'
import json
import re
import sys
from pathlib import Path

payload = json.loads(Path(sys.argv[1]).read_text())
output = Path(sys.argv[2]).read_text()
assert payload["ok"] is True
assert payload["generated_token"] is False
assert payload["authoritative_mapping_present"] is True
assert payload["authoritative_mapping_match"] is True
assert payload["authoritative_project_id_present"] is True
assert re.fullmatch(r"[0-9a-f]{16}", payload["mapping_fingerprint"])
assert "authoritative-project-token" not in Path(sys.argv[1]).read_text()
assert "authoritative-project-token" in output
assert "platform_project_id = project-123" in output
assert (Path(sys.argv[2]).stat().st_mode & 0o777) == 0o600
PY

echo "letai authoritative mapping guard smoke passed"
