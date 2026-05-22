#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
LOCKFILE="$REPO_ROOT/Cargo.lock"

if [[ ! -f "$LOCKFILE" ]]; then
  echo "error: missing Cargo.lock at $LOCKFILE" >&2
  exit 1
fi

WASM_BINDGEN_LOCK_VERSION="$(
  awk '
    $0 == "[[package]]" { in_pkg = 1; pkg_name = ""; next }
    in_pkg && $0 ~ /^name = "wasm-bindgen"$/ { pkg_name = "wasm-bindgen"; next }
    in_pkg && pkg_name == "wasm-bindgen" && $0 ~ /^version = "/ {
      gsub(/^version = "/, "", $0)
      gsub(/"$/, "", $0)
      print $0
      exit
    }
  ' "$LOCKFILE"
)"

if [[ -z "$WASM_BINDGEN_LOCK_VERSION" ]]; then
  echo "error: failed to resolve wasm-bindgen version from $LOCKFILE" >&2
  exit 1
fi

WASM_BINDGEN_CACHE_ROOT="${XDG_CACHE_HOME:-$HOME/.cache}/oasis7/wasm-bindgen-cli/$WASM_BINDGEN_LOCK_VERSION"
