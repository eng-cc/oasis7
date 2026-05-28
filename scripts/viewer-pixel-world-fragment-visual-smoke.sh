#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

npm --prefix crates/oasis7_viewer run test:pixel-world:visual -- "$@"
