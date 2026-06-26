#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

manifests=(
  "tools/wasm_build_suite/Cargo.toml"
  "tools/scenario_test_runner/Cargo.toml"
  "tools/wasm_module_observe/Cargo.toml"
)

for manifest in "${manifests[@]}"; do
  lockfile="$(dirname "$manifest")/Cargo.lock"
  if [[ ! -f "$lockfile" ]]; then
    echo "error: standalone tool lockfile missing: $lockfile" >&2
    exit 1
  fi

  echo "checking standalone tool lockfile: $manifest"
  env -u RUSTC_WRAPPER cargo metadata \
    --manifest-path "$manifest" \
    --locked \
    --format-version 1 >/dev/null
done

echo "ok: standalone tool lockfiles are locked and manifest-consistent (${#manifests[@]} manifests)"
