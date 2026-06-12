#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
viewer_wasm_bindgen_bin="$("$ROOT_DIR/scripts/ensure-wasm-bindgen-cli.sh" --print-bin)"

if [[ ! -x "$ROOT_DIR/crates/oasis7_viewer/node_modules/.bin/vite" ]]; then
  echo "+ npm --prefix crates/oasis7_viewer ci"
  (
    cd "$ROOT_DIR"
    npm --prefix crates/oasis7_viewer ci
  )
fi

echo "+ WASM_BINDGEN_BIN=$viewer_wasm_bindgen_bin npm --prefix crates/oasis7_viewer run build:software-safe"
(
  cd "$ROOT_DIR"
  WASM_BINDGEN_BIN="$viewer_wasm_bindgen_bin" npm --prefix crates/oasis7_viewer run build:software-safe
)

"$ROOT_DIR/scripts/copy-viewer-web-dist.sh" --dist-dir "$ROOT_DIR/crates/oasis7_viewer/dist" >/dev/null
