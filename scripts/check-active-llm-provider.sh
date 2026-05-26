#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/cargo-dev-lib.sh"

TARGET_DIR="$(oasis7_cargo_dev_debug_bin_dir "$ROOT_DIR")"
PROBE_BIN="$TARGET_DIR/oasis7_llm_provider_probe"

(
  cd "$ROOT_DIR"
  oasis7_cargo_dev build -q -p oasis7 --bin oasis7_llm_provider_probe >&2
)
exec "$PROBE_BIN" "$@"
