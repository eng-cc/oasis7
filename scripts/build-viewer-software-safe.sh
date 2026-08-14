#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
source "$ROOT_DIR/scripts/viewer-web-dist-contract.sh"
source "$ROOT_DIR/scripts/viewer-dependency-preflight.sh"
viewer_dist_dir="$ROOT_DIR/crates/oasis7_viewer/dist"

if [[ "${OASIS7_FORCE_VIEWER_SOFTWARE_SAFE_BUILD:-0}" != "1" ]] \
  && viewer_web_dist_check_freshness "$ROOT_DIR" "$viewer_dist_dir" >/dev/null 2>&1; then
  echo "+ viewer software-safe dist is fresh; skipping rebuild"
  "$ROOT_DIR/scripts/copy-viewer-web-dist.sh" --dist-dir "$viewer_dist_dir" >/dev/null
  exit 0
fi

viewer_dependency_preflight "$ROOT_DIR" build

viewer_wasm_bindgen_bin="$("$ROOT_DIR/scripts/ensure-wasm-bindgen-cli.sh" --print-bin)"
echo "+ WASM_BINDGEN_BIN=$viewer_wasm_bindgen_bin npm --prefix crates/oasis7_viewer run build:software-safe"
(
  cd "$ROOT_DIR"
  WASM_BINDGEN_BIN="$viewer_wasm_bindgen_bin" npm --prefix crates/oasis7_viewer run build:software-safe
)

"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" --dist-dir "$viewer_dist_dir" >/dev/null
