#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
LOCKFILE="$REPO_ROOT/Cargo.lock"

usage() {
  cat <<'USAGE'
Usage: ./scripts/ensure-wasm-bindgen-cli.sh [--print-bin]

Provision the pinned `wasm-bindgen` CLI version required by the viewer build.

Options:
  --print-bin   Print the resolved executable path.
  -h, --help    Show this help.
USAGE
}

PRINT_BIN=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --print-bin)
      PRINT_BIN=1
      shift
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
cached_cli="$cache_root/bin/wasm-bindgen"

cli_version_matches() {
  local candidate="${1:-}"
  [[ -x "$candidate" ]] || return 1
  local version_output
  version_output="$("$candidate" --version 2>/dev/null || true)"
  [[ "$version_output" == "wasm-bindgen $lock_version" ]]
}

install_cli() {
  mkdir -p "$cache_root"
  env -u RUSTC_WRAPPER cargo install \
    --locked \
    --root "$cache_root" \
    --version "$lock_version" \
    wasm-bindgen-cli >&2
}

resolved_cli=""
if cli_version_matches "$cached_cli"; then
  resolved_cli="$cached_cli"
elif cli_version_matches "${WASM_BINDGEN_BIN:-}"; then
  resolved_cli="${WASM_BINDGEN_BIN}"
else
  system_cli="$(command -v wasm-bindgen 2>/dev/null || true)"
  if cli_version_matches "$system_cli"; then
    resolved_cli="$system_cli"
  else
    install_cli
    if ! cli_version_matches "$cached_cli"; then
      echo "failed to provision wasm-bindgen $lock_version under $cache_root" >&2
      exit 1
    fi
    resolved_cli="$cached_cli"
  fi
fi

if [[ "$PRINT_BIN" == "1" ]]; then
  printf '%s\n' "$resolved_cli"
fi
