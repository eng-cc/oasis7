#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "$repo_root/scripts/viewer-dependency-preflight.sh"
viewer_dependency_preflight "$repo_root" test

exec npm --prefix "$repo_root/crates/oasis7_viewer" run test:performance -- "$@"
