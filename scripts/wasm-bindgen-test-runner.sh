#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
LOCKFILE="$REPO_ROOT/Cargo.lock"

if [[ ! -f "$LOCKFILE" ]]; then
  echo "missing Cargo.lock at $LOCKFILE" >&2
  exit 1
fi

lock_version="$(
  awk '
    $0 == "[[package]]" { in_pkg = 1; pkg_name = ""; pkg_version = ""; next }
    in_pkg && $0 ~ /^name = "wasm-bindgen"$/ { pkg_name = "wasm-bindgen"; next }
    in_pkg && pkg_name == "wasm-bindgen" && $0 ~ /^version = "/ {
      gsub(/^version = "/, "", $0)
      gsub(/"$/, "", $0)
      print $0
      exit
    }
  ' "$LOCKFILE"
)"

if [[ -z "$lock_version" ]]; then
  echo "failed to resolve wasm-bindgen version from $LOCKFILE" >&2
  exit 1
fi

cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/oasis7/wasm-bindgen-cli/$lock_version"
cached_runner="$cache_root/bin/wasm-bindgen-test-runner"

runner_version_matches() {
  local candidate="$1"
  [[ -x "$candidate" ]] || return 1
  local version_output
  version_output="$("$candidate" --version 2>/dev/null || true)"
  [[ "$version_output" == "wasm-bindgen-test-runner $lock_version" ]]
}

install_runner() {
  mkdir -p "$cache_root"
  env -u RUSTC_WRAPPER cargo install \
    --locked \
    --root "$cache_root" \
    --version "$lock_version" \
    wasm-bindgen-cli >&2
}

if runner_version_matches "$cached_runner"; then
  exec "$cached_runner" "$@"
fi

if runner_version_matches "${WASM_BINDGEN_TEST_RUNNER_BIN:-}"; then
  exec "${WASM_BINDGEN_TEST_RUNNER_BIN}" "$@"
fi

if runner_version_matches "$(command -v wasm-bindgen-test-runner 2>/dev/null || true)"; then
  exec "$(command -v wasm-bindgen-test-runner)" "$@"
fi

install_runner

if ! runner_version_matches "$cached_runner"; then
  echo "failed to provision wasm-bindgen-test-runner $lock_version under $cache_root" >&2
  exit 1
fi

exec "$cached_runner" "$@"
