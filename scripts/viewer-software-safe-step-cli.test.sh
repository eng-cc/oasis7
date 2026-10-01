#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
runner="$repo_root/scripts/viewer-software-safe-step-regression.sh"
tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

help=$(bash "$runner" --progress-timeout-ms 1234 --help)
[[ "$help" == *"--progress-timeout-ms <ms>"* ]] || { echo "error: canonical timeout missing from help" >&2; exit 1; }

expect_rejection() {
  local expected=$1
  shift
  local output status=0
  output=$(bash "$runner" "$@" 2>&1) || status=$?
  [[ "$status" == 2 && "$output" == *"$expected"* ]] || {
    echo "error: expected exit 2 with '$expected', got exit $status: $output" >&2
    exit 1
  }
}

expect_rejection "error: --progress-timeout-ms must be positive" --progress-timeout-ms 0
expect_rejection "error: unsupported option --step-timeout-ms; use --progress-timeout-ms" --step-timeout-ms 1234 --help
expect_rejection "error: unsupported option --step-timeout-ms; use --progress-timeout-ms" --url http://127.0.0.1:1 --out-dir "$tmpdir/output" --step-timeout-ms 1234
[[ ! -e "$tmpdir/output" ]] || { echo "error: retired option created runtime artifacts" >&2; exit 1; }
[[ "$help" != *"--step-timeout-ms"* ]] || { echo "error: retired timeout remains in help" >&2; exit 1; }
echo "viewer software-safe timeout CLI contract passed"
