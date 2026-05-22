#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$ROOT_DIR/scripts/viewer-web-dist-contract.sh"
VIEWER_ROOT="$ROOT_DIR/crates/oasis7_viewer"
DIST_DIR=""

usage() {
  cat <<'USAGE'
Usage: ./scripts/copy-viewer-web-dist.sh --dist-dir <path> [--viewer-root <path>]

Copy the canonical viewer web dist into a prepared output directory.
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

require_file() {
  local path="$1"
  [[ -f "$path" ]] || {
    echo "error: required viewer dist input missing: $path" >&2
    exit 1
  }
}

software_safe_html="$VIEWER_ROOT/software_safe.html"
viewer_js="$VIEWER_ROOT/viewer.js"
compat_js="$VIEWER_ROOT/software_safe.js"
pixel_world_bridge_dir="$VIEWER_ROOT/pixel-world-bridge"

require_file "$software_safe_html"
require_file "$viewer_js"
require_file "$compat_js"
while read -r source_rel _; do
  require_file "$VIEWER_ROOT/$source_rel"
done < <(viewer_web_dist_contract_pairs)

if ! grep -Fq 'src="./viewer.js"' "$software_safe_html"; then
  echo "error: software_safe.html no longer points at canonical viewer.js" >&2
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
  cp "$VIEWER_ROOT/$source_rel" "$DIST_DIR/$dist_rel"
done < <(viewer_web_dist_contract_pairs)

if [[ -d "$pixel_world_bridge_dir" ]]; then
  rm -rf "$DIST_DIR/pixel-world-bridge"
  cp -R "$pixel_world_bridge_dir" "$DIST_DIR/pixel-world-bridge"
fi

viewer_web_dist_write_manifest "$ROOT_DIR" "$DIST_DIR"
