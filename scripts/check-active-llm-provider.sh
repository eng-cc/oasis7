#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

resolve_target_dir() {
  local base_dir
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    if [[ "${CARGO_TARGET_DIR}" == /* ]]; then
      base_dir="${CARGO_TARGET_DIR}"
    else
      base_dir="$ROOT_DIR/${CARGO_TARGET_DIR}"
    fi
  else
    base_dir="$ROOT_DIR/target"
  fi
  printf '%s/debug\n' "$base_dir"
}

TARGET_DIR="$(resolve_target_dir)"
PROBE_BIN="$TARGET_DIR/oasis7_llm_provider_probe"

(
  cd "$ROOT_DIR"
  env -u RUSTC_WRAPPER cargo build -q -p oasis7 --bin oasis7_llm_provider_probe >&2
)
exec "$PROBE_BIN" "$@"
