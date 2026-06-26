#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

exec npm --prefix "$repo_root/crates/oasis7_viewer" run test:performance -- "$@"
