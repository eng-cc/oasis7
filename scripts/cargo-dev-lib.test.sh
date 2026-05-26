#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

source "$ROOT_DIR/scripts/cargo-dev-lib.sh"

shared_dir="$(CI= oasis7_cargo_dev_target_dir "$ROOT_DIR")"
expected_shared="$("$ROOT_DIR/scripts/cargo-dev.sh" --print-target-dir)"
if [[ "$shared_dir" != "$expected_shared" ]]; then
  echo "unexpected shared target dir: $shared_dir != $expected_shared" >&2
  exit 1
fi

debug_dir="$(CI= oasis7_cargo_dev_debug_bin_dir "$ROOT_DIR")"
if [[ "$debug_dir" != "$expected_shared/debug" ]]; then
  echo "unexpected shared debug bin dir: $debug_dir" >&2
  exit 1
fi

raw_dir="$(CI=1 CARGO_TARGET_DIR=relative-target oasis7_cargo_dev_target_dir "$ROOT_DIR")"
if [[ "$raw_dir" != "$ROOT_DIR/relative-target" ]]; then
  echo "unexpected CI relative target dir: $raw_dir" >&2
  exit 1
fi

absolute_target="$ROOT_DIR/.tmp/absolute-target-fixture"
raw_absolute="$(OASIS7_CARGO_DEV_SHARED=0 CARGO_TARGET_DIR="$absolute_target" oasis7_cargo_dev_target_dir "$ROOT_DIR")"
if [[ "$raw_absolute" != "$absolute_target" ]]; then
  echo "unexpected raw absolute target dir: $raw_absolute" >&2
  exit 1
fi

default_raw="$(OASIS7_FORCE_RAW_CARGO=1 env -u CARGO_TARGET_DIR bash -c 'source "$1"; oasis7_cargo_dev_target_dir "$2"' _ "$ROOT_DIR/scripts/cargo-dev-lib.sh" "$ROOT_DIR")"
if [[ "$default_raw" != "$ROOT_DIR/target" ]]; then
  echo "unexpected default raw target dir: $default_raw" >&2
  exit 1
fi

echo "cargo-dev-lib.test: OK"
