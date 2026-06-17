#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

run() {
  echo "+ $*"
  "$@"
}

ensure_file_contains() {
  local file=$1
  local pattern=$2
  if ! rg -q -- "$pattern" "$file"; then
    echo "error: pattern not found: $pattern" >&2
    echo "  file=$file" >&2
    exit 1
  fi
}

smoke_root=".tmp/release_candidate_bundle_smoke"
rm -rf "$smoke_root"
mkdir -p "$smoke_root/runtime" "$smoke_root/world" "$smoke_root/evidence"

printf 'runtime-build-v1\n' >"$smoke_root/runtime/runtime.bin"
printf 'snapshot\n' >"$smoke_root/world/state.txt"
printf '{"signers":["signer01"]}\n' >"$smoke_root/world/public_manifest.json"
printf '# smoke evidence\n' >"$smoke_root/evidence/evidence.md"

bundle_path="$smoke_root/candidate.json"
validation_ok="$smoke_root/validation-ok.json"
validation_fail="$smoke_root/validation-fail.log"

run ./scripts/release-candidate-bundle.sh create \
  --bundle "$bundle_path" \
  --candidate-id "public-testnet-rehearsal-smoke-01" \
  --track "public_testnet_rehearsal" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --evidence-ref "$smoke_root/evidence/evidence.md" \
  --note "smoke test bundle" \
  --allow-dirty-worktree

run ./scripts/release-candidate-bundle.sh validate \
  --bundle "$bundle_path" \
  >"$validation_ok"
ensure_file_contains "$validation_ok" '"validation": "ok"'
ensure_file_contains "$bundle_path" '"candidate_id": "public-testnet-rehearsal-smoke-01"'

printf 'mutated\n' >>"$smoke_root/world/state.txt"
set +e
./scripts/release-candidate-bundle.sh validate \
  --bundle "$bundle_path" \
  >"$validation_fail" 2>&1
fail_code=$?
set -e

if [[ "$fail_code" -eq 0 ]]; then
  echo "error: expected validation drift failure after mutating snapshot" >&2
  exit 1
fi
ensure_file_contains "$validation_fail" 'world_snapshot drift detected'

echo "release candidate bundle smoke checks passed"
