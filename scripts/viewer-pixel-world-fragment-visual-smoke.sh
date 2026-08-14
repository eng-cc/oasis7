#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"
source "$repo_root/scripts/viewer-dependency-preflight.sh"
viewer_dependency_preflight "$repo_root" test

npm --prefix crates/oasis7_viewer run test:pixel-world:visual -- "$@"
