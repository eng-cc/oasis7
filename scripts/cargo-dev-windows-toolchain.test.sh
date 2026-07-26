#!/usr/bin/env bash
# Cross-platform test contract: Windows Git Bash fallback must not alter Linux/macOS PATH behavior.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

case "${OSTYPE:-}" in
  msys*|cygwin*) ;;
  *)
    echo "cargo-dev-windows-toolchain.test: SKIP (Windows-only fallback contract)"
    exit 0
    ;;
esac

GIT_DIR="$(dirname "$(command -v git)")"
clean_path="$GIT_DIR:/usr/bin:/bin"
if ! target_dir="$(PATH="$clean_path" "$ROOT_DIR/scripts/cargo-dev.sh" --print-target-dir)"; then
  echo "cargo-dev-windows-toolchain.test: cargo target discovery failed without login-profile Rust PATH" >&2
  exit 1
fi
[[ -n "$target_dir" ]] || {
  echo "cargo-dev-windows-toolchain.test: empty cargo target directory" >&2
  exit 1
}

echo "cargo-dev-windows-toolchain.test: OK"
