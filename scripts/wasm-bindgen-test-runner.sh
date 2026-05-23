#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
source "$SCRIPT_DIR/wasm-bindgen-cli-common.sh"

cache_root="$WASM_BINDGEN_CACHE_ROOT"
cached_runner="$cache_root/bin/wasm-bindgen-test-runner"

runner_version_matches() {
  local candidate="$1"
  [[ -x "$candidate" ]] || return 1
  local version_output
  version_output="$("$candidate" --version 2>/dev/null || true)"
  [[ "$version_output" == "wasm-bindgen-test-runner $WASM_BINDGEN_LOCK_VERSION" ]]
}

install_runner() {
  mkdir -p "$cache_root"
  env -u RUSTC_WRAPPER cargo install \
    --locked \
    --root "$cache_root" \
    --version "$WASM_BINDGEN_LOCK_VERSION" \
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
  echo "error: failed to provision wasm-bindgen-test-runner $WASM_BINDGEN_LOCK_VERSION under $cache_root" >&2
  exit 1
fi

exec "$cached_runner" "$@"
