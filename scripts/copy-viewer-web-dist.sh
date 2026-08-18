#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$ROOT_DIR/scripts/viewer-web-dist-contract.sh"
VIEWER_ROOT="$ROOT_DIR/crates/oasis7_viewer"
DIST_DIR=""
OPTIONAL_PAYLOAD_DIR=""
OPTIONAL_PAYLOAD_NAME="pixel_world_bridge_bindgen_bg.wasm"

usage() {
  cat <<'USAGE'
Usage: ./scripts/copy-viewer-web-dist.sh --dist-dir <path> [--viewer-root <path>] \
  [--optional-payload-dir <path>]

Copy the canonical viewer web dist into a prepared output directory.

When --optional-payload-dir is provided, the WebGL2 bridge WASM is staged in
that directory instead of the primary viewer dist. The primary dist receives
optional-payloads.json so consumers can distinguish an available payload from
a deterministic source_missing result.
USAGE
}

resolve_abs_path() {
  local raw="$1"
  if [[ "$raw" == /* ]]; then
    printf '%s\n' "$raw"
  else
    printf '%s\n' "$ROOT_DIR/$raw"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dist-dir)
      DIST_DIR="${2:-}"
      shift 2
      ;;
    --viewer-root)
      VIEWER_ROOT="${2:-}"
      shift 2
      ;;
    --optional-payload-dir)
      OPTIONAL_PAYLOAD_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$DIST_DIR" ]] || { echo "error: --dist-dir is required" >&2; exit 2; }
VIEWER_ROOT="$(resolve_abs_path "$VIEWER_ROOT")"
DIST_DIR="$(resolve_abs_path "$DIST_DIR")"
if [[ -n "$OPTIONAL_PAYLOAD_DIR" ]]; then
  OPTIONAL_PAYLOAD_DIR="$(resolve_abs_path "$OPTIONAL_PAYLOAD_DIR")"
fi

require_file() {
  local path="$1"
  [[ -f "$path" ]] || {
    echo "error: required viewer dist input missing: $path" >&2
    exit 1
  }
}

viewer_html="$VIEWER_ROOT/viewer.html"
software_safe_html="$VIEWER_ROOT/software_safe.html"
viewer_js="$VIEWER_ROOT/viewer.js"
compat_js="$VIEWER_ROOT/software_safe.js"
pixel_world_bridge_dir="$VIEWER_ROOT/dist/pixel-world-bridge"
pixel_world_bridge_js="$pixel_world_bridge_dir/pixel_world_bridge.js"
pixel_world_webgl2_bridge_js="$pixel_world_bridge_dir/webgl2/pixel_world_bridge.js"
pixel_world_webgl2_bindgen_js="$pixel_world_bridge_dir/webgl2/pixel_world_bridge_bindgen.js"
pixel_world_webgl2_wasm="$pixel_world_bridge_dir/webgl2/pixel_world_bridge_bindgen_bg.wasm"

require_file "$viewer_html"
require_file "$software_safe_html"
require_file "$viewer_js"
require_file "$compat_js"
require_file "$pixel_world_bridge_js"
require_file "$pixel_world_webgl2_bridge_js"
require_file "$pixel_world_webgl2_bindgen_js"
if [[ -z "$OPTIONAL_PAYLOAD_DIR" ]]; then
  require_file "$pixel_world_webgl2_wasm"
fi
while read -r source_rel _; do
  require_file "$VIEWER_ROOT/$source_rel"
done < <(viewer_web_dist_contract_pairs)

if ! grep -Fq 'src="./viewer.js"' "$viewer_html"; then
  echo "error: viewer.html no longer points at canonical viewer.js" >&2
  exit 1
fi
if ! cmp -s "$viewer_html" "$software_safe_html"; then
  echo "error: software_safe.html must remain a compatibility copy of canonical viewer.html" >&2
  exit 1
fi
if ! grep -Fq 'import "./viewer.js";' "$compat_js"; then
  echo "error: software_safe.js is no longer a compat alias to viewer.js" >&2
  exit 1
fi
if cmp -s "$viewer_js" "$compat_js"; then
  echo "error: viewer.js and software_safe.js unexpectedly contain identical payloads" >&2
  exit 1
fi

mkdir -p "$DIST_DIR"
while read -r source_rel dist_rel; do
  source_path="$VIEWER_ROOT/$source_rel"
  dist_path="$DIST_DIR/$dist_rel"
  if [[ "$source_path" != "$dist_path" ]]; then
    cp "$source_path" "$dist_path"
  fi
done < <(viewer_web_dist_contract_pairs)

if [[ -d "$pixel_world_bridge_dir" ]]; then
  if [[ "$pixel_world_bridge_dir" != "$DIST_DIR/pixel-world-bridge" ]]; then
    rm -rf "$DIST_DIR/pixel-world-bridge"
    cp -R "$pixel_world_bridge_dir" "$DIST_DIR/pixel-world-bridge"
  fi
else
  # A prior packaging-mode copy may have left the optional manifest in a
  # reused output directory. Legacy developer copies keep the adjacent WASM
  # and must not leave stale split-mode metadata behind.
  rm -f "$DIST_DIR/optional-payloads.json"
fi

if [[ -n "$OPTIONAL_PAYLOAD_DIR" ]]; then
  mkdir -p "$OPTIONAL_PAYLOAD_DIR"
  staged_optional_payload="$OPTIONAL_PAYLOAD_DIR/$OPTIONAL_PAYLOAD_NAME"

  if [[ -f "$pixel_world_webgl2_wasm" ]]; then
    cp "$pixel_world_webgl2_wasm" "$staged_optional_payload"
    rm -f "$DIST_DIR/pixel-world-bridge/webgl2/$OPTIONAL_PAYLOAD_NAME"
    python3 - "$DIST_DIR/optional-payloads.json" "$OPTIONAL_PAYLOAD_NAME" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
payload_name = sys.argv[2]
manifest_path.write_text(
    json.dumps(
        {payload_name: {"available": True, "path": payload_name}},
        ensure_ascii=False,
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
  elif [[ -f "$staged_optional_payload" ]]; then
    # An in-place split copy removes the generated WASM from the primary
    # dist. Reuse the already staged payload on a freshness-skipped rerun so
    # staging remains idempotent instead of reporting a false source_missing.
    rm -f "$DIST_DIR/pixel-world-bridge/webgl2/$OPTIONAL_PAYLOAD_NAME"
    python3 - "$DIST_DIR/optional-payloads.json" "$OPTIONAL_PAYLOAD_NAME" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
payload_name = sys.argv[2]
manifest_path.write_text(
    json.dumps(
        {payload_name: {"available": True, "path": payload_name}},
        ensure_ascii=False,
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
  else
    rm -f "$DIST_DIR/pixel-world-bridge/webgl2/$OPTIONAL_PAYLOAD_NAME"
    python3 - "$DIST_DIR/optional-payloads.json" "$OPTIONAL_PAYLOAD_NAME" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

manifest_path = Path(sys.argv[1])
payload_name = sys.argv[2]
manifest_path.write_text(
    json.dumps(
        {payload_name: {"available": False, "reason": "source_missing"}},
        ensure_ascii=False,
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY
  fi
fi

viewer_web_dist_write_manifest "$ROOT_DIR" "$DIST_DIR"
