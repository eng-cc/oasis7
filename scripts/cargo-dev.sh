#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: ./scripts/cargo-dev.sh [--print-target-dir] <cargo-args...>

Run cargo with a repo-family shared development target dir so multiple git
worktrees can reuse build artifacts.

Default behavior:
- Computes a stable cache namespace from `git rev-parse --git-common-dir`
- Stores shared targets under `<repo-parent>/.oasis7-cache/cargo-target/`
- Unsets `RUSTC_WRAPPER` to match the repository's default cargo invocation rule

Options:
  --print-target-dir   Print the resolved shared target dir and exit
  -h, --help           Show this help

Environment:
  OASIS7_CARGO_SHARED_TARGET_DIR
      Override the resolved shared target dir.

Notes:
- This wrapper is for local development commands such as `check`, `test`, `run`,
  and `build`.
- Do not use it for deterministic wasm / release flows that require
  `CARGO_TARGET_DIR` to stay unset.

Examples:
  ./scripts/cargo-dev.sh check -p oasis7
  ./scripts/cargo-dev.sh test -p oasis7_viewer
  ./scripts/cargo-dev.sh run -p oasis7 --bin oasis7_game_launcher
  ./scripts/cargo-dev.sh --print-target-dir
USAGE
}

if [[ $# -eq 0 ]]; then
  usage >&2
  exit 2
fi

case "${1:-}" in
  -h|--help)
    usage
    exit 0
    ;;
esac

COMMON_GIT_DIR="$(cd "$(git rev-parse --git-common-dir)" && pwd -P)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
CANONICAL_REPO_ROOT="$(cd "$COMMON_GIT_DIR/.." && pwd -P)"
HOST_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
RUSTC_RELEASE="$(rustc -vV | sed -n 's/^release: //p')"

cache_namespace() {
  python3 - "$COMMON_GIT_DIR" <<'PY'
from __future__ import annotations

import hashlib
import pathlib
import sys

path = pathlib.Path(sys.argv[1]).resolve()
digest = hashlib.sha256(str(path).encode("utf-8")).hexdigest()[:12]
print(f"git-{digest}")
PY
}

NAMESPACE="$(cache_namespace)"
if [[ -n "${OASIS7_CARGO_SHARED_TARGET_DIR:-}" ]]; then
  TARGET_DIR="$OASIS7_CARGO_SHARED_TARGET_DIR"
else
  CACHE_BASE_DIR="$(dirname "$CANONICAL_REPO_ROOT")/.oasis7-cache"
  TARGET_DIR="$CACHE_BASE_DIR/cargo-target/$NAMESPACE/rustc-$RUSTC_RELEASE-$HOST_TRIPLE"
fi

case "${1:-}" in
  --print-target-dir)
    if [[ $# -ne 1 ]]; then
      echo "error: --print-target-dir does not accept cargo arguments" >&2
      exit 2
    fi
    printf '%s\n' "$TARGET_DIR"
    exit 0
    ;;
esac

mkdir -p "$TARGET_DIR"
exec env -u RUSTC_WRAPPER CARGO_TARGET_DIR="$TARGET_DIR" cargo "$@"
