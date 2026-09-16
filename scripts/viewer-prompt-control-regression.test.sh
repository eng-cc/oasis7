#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
runner="$repo_root/scripts/viewer-prompt-control-regression.sh"
tmp_root=$(mktemp -d)
trap 'rm -rf "$tmp_root"' EXIT

test -x "$runner"

# Contract-only verification must never touch a browser or launcher/provider.
fake_bin="$tmp_root/bin"
mkdir -p "$fake_bin"
cat >"$fake_bin/agent-browser" <<'EOF'
#!/usr/bin/env bash
printf 'agent-browser must not be invoked during contract-only verification\n' >&2
exit 91
EOF
chmod +x "$fake_bin/agent-browser"

run_contract() {
  local out_dir="$1"
  PATH="$fake_bin:$PATH" "$runner" \
    --contract-only \
    --headed \
    --case-id PWT-004 \
    --out-dir "$out_dir"
}

first_out="$tmp_root/first"
second_out="$tmp_root/second"
run_contract "$first_out"
run_contract "$second_out"

first_manifest=$(find "$first_out" -name artifact-manifest.json -type f -print -quit)
second_manifest=$(find "$second_out" -name artifact-manifest.json -type f -print -quit)
test -n "$first_manifest"
test -n "$second_manifest"
cmp -s "$first_manifest" "$second_manifest"

python3 - "$first_manifest" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest["schema"] == "oasis7.viewer.prompt-control-artifact-manifest/v1"
assert manifest["caseId"] == "PWT-004"
assert manifest["evidenceTier"] == "contract_only"
assert manifest["acceptanceEligible"] is False
assert manifest["providerCallsDuringVerification"] is False
assert manifest["browserMode"] == "headed"
assert manifest["visibleActionContract"] == {
    "selection": "[data-pixel-world-agent-marker=\\\"true\\\"][data-agent-id]",
    "promptDisclosure": "Advanced Prompt Settings",
    "shortTermGoal": "#prompt-short",
    "preview": "button[data-prompt-action=\\\"preview\\\"]",
    "apply": "button[data-prompt-action=\\\"apply\\\"]",
    "rollback": "button[data-prompt-action=\\\"rollback\\\"]",
}
paths = [entry["path"] for entry in manifest["artifacts"]]
assert paths == sorted(paths)
assert paths == ["contract.json", "manifest-input.json"]
for entry in manifest["artifacts"]:
    assert set(entry) == {"bytes", "path", "sha256"}
    assert len(entry["sha256"]) == 64
PY

if "$runner" --headless --contract-only --out-dir "$tmp_root/headless" >"$tmp_root/headless.log" 2>&1; then
  echo "headless contract unexpectedly passed" >&2
  exit 1
fi
grep -Fq -- "--headed is required" "$tmp_root/headless.log"

if rg -n 'sendPromptControl|__AW_TEST__\.sendPromptControl' "$runner"; then
  echo "runner must not replace visible prompt actions with sendPromptControl" >&2
  exit 1
fi
rg -Fq 'data-pixel-world-agent-marker' "$runner"
rg -Fq 'fill "#prompt-short"' "$runner"
rg -Fq 'button[data-prompt-action="preview"]' "$runner"
rg -Fq 'button[data-prompt-action="apply"]' "$runner"
rg -Fq 'button[data-prompt-action="rollback"]' "$runner"

echo "viewer-prompt-control runner contract: passed"
