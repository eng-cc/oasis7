#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
viewer_wasm_bindgen_bin="$("$ROOT_DIR/scripts/ensure-wasm-bindgen-cli.sh" --print-bin)"

echo "+ WASM_BINDGEN_BIN=$viewer_wasm_bindgen_bin npm --prefix crates/oasis7_viewer run build:software-safe"
(
  cd "$ROOT_DIR"
  WASM_BINDGEN_BIN="$viewer_wasm_bindgen_bin" npm --prefix crates/oasis7_viewer run build:software-safe
)
