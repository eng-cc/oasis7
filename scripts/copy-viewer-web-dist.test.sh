#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$ROOT_DIR/scripts/viewer-web-dist-contract.sh"
TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT

viewer_root="$TMPDIR/viewer"
dist_dir="$TMPDIR/dist"
optional_payload_dir="$TMPDIR/optional-payload"
mkdir -p "$viewer_root/dist/pixel-world-bridge/webgl2"

printf '<!doctype html><link rel="stylesheet" href="./viewer_terminal_shell.css"><script type="module" src="./viewer.js"></script>\n' > "$viewer_root/viewer.html"
cp "$viewer_root/viewer.html" "$viewer_root/software_safe.html"
printf '.shell { grid-template-columns: minmax(0, 1fr); }\n' > "$viewer_root/viewer_terminal_shell.css"
printf 'console.log("canonical bundle");\n' > "$viewer_root/viewer.js"
printf '// Generated compat alias; canonical bundle truth lives in ./viewer.js.\nimport "./viewer.js";\n' > "$viewer_root/software_safe.js"
printf '<!doctype html>canonical claim evidence\n' > "$viewer_root/viewer_first_agent_claim_evidence.html"
printf '<!doctype html>claim evidence\n' > "$viewer_root/software_safe_first_agent_claim_evidence.html"
printf 'icon\n' > "$viewer_root/favicon.ico"
printf 'export const bridge = true;\n' > "$viewer_root/dist/pixel-world-bridge/pixel_world_bridge.js"
printf 'export const webgl2Bridge = true;\n' > "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge.js"
printf 'export function init() {}\n' > "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen.js"
printf '\0asm\n' > "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm"

"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" \
  --viewer-root "$viewer_root" \
  --dist-dir "$dist_dir" \
  --optional-payload-dir "$optional_payload_dir"

failures=0
expect_cmp() {
  local source=$1
  local copied=$2
  if ! cmp "$source" "$copied"; then
    echo "expected copied asset to match: $copied" >&2
    failures=1
  fi
}

expect_cmp "$viewer_root/viewer.js" "$dist_dir/viewer.js"
expect_cmp "$viewer_root/software_safe.js" "$dist_dir/software_safe.js"
expect_cmp "$viewer_root/viewer.html" "$dist_dir/index.html"
expect_cmp "$viewer_root/viewer.html" "$dist_dir/viewer.html"
expect_cmp "$viewer_root/software_safe.html" "$dist_dir/software_safe.html"
expect_cmp "$viewer_root/viewer_terminal_shell.css" "$dist_dir/viewer_terminal_shell.css"
expect_cmp "$viewer_root/viewer_first_agent_claim_evidence.html" "$dist_dir/viewer_first_agent_claim_evidence.html"
expect_cmp "$viewer_root/software_safe_first_agent_claim_evidence.html" "$dist_dir/software_safe_first_agent_claim_evidence.html"
expect_cmp "$viewer_root/favicon.ico" "$dist_dir/favicon.ico"
expect_cmp "$viewer_root/dist/pixel-world-bridge/pixel_world_bridge.js" "$dist_dir/pixel-world-bridge/pixel_world_bridge.js"
expect_cmp "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge.js" "$dist_dir/pixel-world-bridge/webgl2/pixel_world_bridge.js"
expect_cmp "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen.js" "$dist_dir/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen.js"
if [[ -e "$dist_dir/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm" ]]; then
  echo "optional pixel-world WASM must not be copied into the primary viewer dist" >&2
  exit 1
fi
cmp "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm" \
  "$optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm"
python3 - "$dist_dir/optional-payloads.json" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = manifest["pixel_world_bridge_bindgen_bg.wasm"]
assert payload == {
    "available": False,
    "reason": "separate_artifact",
}, payload
PY

missing_viewer_root="$TMPDIR/missing-viewer"
missing_dist_dir="$TMPDIR/missing-dist"
missing_optional_payload_dir="$TMPDIR/missing-optional-payload"
cp -R "$viewer_root" "$missing_viewer_root"
rm "$missing_viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm"
mkdir -p "$missing_optional_payload_dir"
printf 'stale bytes from another build\n' \
  > "$missing_optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm"

"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" \
  --viewer-root "$missing_viewer_root" \
  --dist-dir "$missing_dist_dir" \
  --optional-payload-dir "$missing_optional_payload_dir"

if [[ -e "$missing_optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm" ]]; then
  echo "missing optional pixel-world WASM must not create a staged payload" >&2
  exit 1
fi
python3 - "$missing_dist_dir/optional-payloads.json" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = manifest["pixel_world_bridge_bindgen_bg.wasm"]
assert payload["available"] is False, payload
assert payload["reason"] == "source_missing", payload
assert "path" not in payload, payload
PY

in_place_dir="$TMPDIR/in-place-viewer"
in_place_optional_payload_dir="$TMPDIR/in-place-optional-payload"
cp -R "$viewer_root" "$in_place_dir"

for pass in first second; do
  "$ROOT_DIR/scripts/copy-viewer-web-dist.sh" \
    --viewer-root "$in_place_dir" \
    --dist-dir "$in_place_dir" \
    --optional-payload-dir "$in_place_optional_payload_dir"

  if [[ ! -e "$in_place_dir/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm" ]]; then
    echo "in-place split pass '$pass' removed canonical source WASM" >&2
    exit 1
  fi
  cmp "$viewer_root/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm" \
    "$in_place_optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm"
  python3 - "$in_place_dir/optional-payloads.json" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
assert manifest == {
    "pixel_world_bridge_bindgen_bg.wasm": {
        "available": False,
        "reason": "separate_artifact",
    }
}, manifest
PY
done

rm "$in_place_dir/dist/pixel-world-bridge/webgl2/pixel_world_bridge_bindgen_bg.wasm"
printf 'stale bytes from another build\n' \
  > "$in_place_optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm"
"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" \
  --viewer-root "$in_place_dir" \
  --dist-dir "$in_place_dir" \
  --optional-payload-dir "$in_place_optional_payload_dir"
if [[ -e "$in_place_optional_payload_dir/pixel_world_bridge_bindgen_bg.wasm" ]]; then
  echo "stale staged optional WASM must be removed when source is missing" >&2
  exit 1
fi
python3 - "$in_place_dir/optional-payloads.json" <<'PY'
import json
import sys
from pathlib import Path

manifest = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
payload = manifest["pixel_world_bridge_bindgen_bg.wasm"]
assert payload == {"available": False, "reason": "source_missing"}, payload
PY

if ! viewer_web_dist_contract_pairs | grep -Fxq "viewer_terminal_shell.css viewer_terminal_shell.css"; then
  echo "viewer dist contract must include viewer_terminal_shell.css" >&2
  failures=1
fi
if ! viewer_web_dist_required_files | grep -Fxq "viewer_terminal_shell.css"; then
  echo "viewer dist required files must include viewer_terminal_shell.css" >&2
  failures=1
fi

fingerprint_repo="$TMPDIR/fingerprint-repo"
mkdir -p "$fingerprint_repo/crates/oasis7_viewer"
printf '.shell { grid-template-columns: minmax(0, 1fr); }\n' > "$fingerprint_repo/crates/oasis7_viewer/viewer_terminal_shell.css"
before_fingerprint="$(viewer_web_dist_source_metadata_json "$fingerprint_repo" | python3 -c 'import json,sys; print(json.load(sys.stdin)["sourceFingerprint"])')"
printf '\n.shell { overflow-wrap: anywhere; }\n' >> "$fingerprint_repo/crates/oasis7_viewer/viewer_terminal_shell.css"
after_fingerprint="$(viewer_web_dist_source_metadata_json "$fingerprint_repo" | python3 -c 'import json,sys; print(json.load(sys.stdin)["sourceFingerprint"])')"
if [[ "$before_fingerprint" == "$after_fingerprint" ]]; then
  echo "viewer dist freshness fingerprint must include viewer_terminal_shell.css" >&2
  failures=1
fi

if ((failures)); then
  exit 1
fi

echo "copy-viewer-web-dist.test: OK"
