#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run() {
  echo "+ $*"
  "$@"
}

run env -u RUSTC_WRAPPER cargo fmt --all
run git add -u
run ./scripts/ci-tests.sh commit
