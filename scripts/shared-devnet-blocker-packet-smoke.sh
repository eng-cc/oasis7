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

smoke_root=".tmp/shared_devnet_blocker_packet_smoke"
rm -rf "$smoke_root"
mkdir -p "$smoke_root/runtime" "$smoke_root/world" "$smoke_root/evidence"

printf 'runtime-build-v1\n' >"$smoke_root/runtime/runtime.bin"
printf 'snapshot\n' >"$smoke_root/world/state.txt"
printf '{"signers":["signer01"]}\n' >"$smoke_root/world/public_manifest.json"
printf '# current gate\n' >"$smoke_root/evidence/current-gate.md"
printf '# fallback gate\n' >"$smoke_root/evidence/fallback-gate.md"
printf '# oncall\n' >"$smoke_root/evidence/oncall.md"
printf '# operator handoff\n' >"$smoke_root/evidence/operator.md"
printf '# restore\n' >"$smoke_root/evidence/restore.md"
printf '# screenshot\n' >"$smoke_root/evidence/screenshot.md"
printf '# mixed topology baseline\n' >"$smoke_root/evidence/mixed-topology-baseline.md"
printf '# mixed topology shared window\n' >"$smoke_root/evidence/mixed-topology-shared.md"
printf '# proxy drill\n' >"$smoke_root/evidence/mixed-topology-proxy.md"
printf '# pass uplift review\n' >"$smoke_root/evidence/mixed-topology-pass-decision.md"

current_bundle="$smoke_root/current.json"
fallback_bundle="$smoke_root/fallback.json"

run ./scripts/release-candidate-bundle.sh create \
  --bundle "$current_bundle" \
  --candidate-id "shared-devnet-current-01" \
  --track "shared_devnet" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree

run ./scripts/release-candidate-bundle.sh create \
  --bundle "$fallback_bundle" \
  --candidate-id "shared-devnet-fallback-01" \
  --track "shared_devnet" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree

access_out="$smoke_root/shared-access.md"
mixed_topology_out="$smoke_root/mixed-topology.md"
rollback_out="$smoke_root/rollback-target.md"
run ./scripts/shared-devnet-blocker-packet.sh \
  --window-id shared-devnet-20260324-06 \
  --candidate-bundle "$current_bundle" \
  --candidate-gate-summary "$smoke_root/evidence/current-gate.md" \
  --access-out "$access_out" \
  --mixed-topology-out "$mixed_topology_out" \
  --rollback-out "$rollback_out" \
  --viewer-url "https://shared.example.invalid/viewer" \
  --live-addr "shared.example.invalid:443" \
  --operator-contact-ref "$smoke_root/evidence/operator.md" \
  --independent-operator-ref "$smoke_root/evidence/oncall.md" \
  --access-evidence-ref "$smoke_root/evidence/screenshot.md" \
  --mixed-topology-baseline-ref "$smoke_root/evidence/mixed-topology-baseline.md" \
  --mixed-topology-shared-evidence-ref "$smoke_root/evidence/mixed-topology-shared.md" \
  --mixed-topology-proxy-ref "$smoke_root/evidence/mixed-topology-proxy.md" \
  --fallback-candidate-bundle "$fallback_bundle" \
  --fallback-class bootstrap_restore_ready \
  --fallback-gate-summary "$smoke_root/evidence/fallback-gate.md" \
  --fallback-owner-ref "$smoke_root/evidence/oncall.md" \
  --restore-steps-ref "$smoke_root/evidence/restore.md"

ensure_file_contains "$access_out" 'shared-devnet-current-01'
ensure_file_contains "$access_out" 'https://shared.example.invalid/viewer'
ensure_file_contains "$mixed_topology_out" 'mixed-topology-baseline.md'
ensure_file_contains "$mixed_topology_out" 'mixed-topology-shared.md'
ensure_file_contains "$mixed_topology_out" 'required when lane_result=pass'
ensure_file_contains "$rollback_out" 'shared-devnet-fallback-01'
ensure_file_contains "$rollback_out" 'bootstrap_restore_ready'
ensure_file_contains "$rollback_out" 'fallback-gate.md'

pass_mixed_topology_out="$smoke_root/mixed-topology-pass.md"
run ./scripts/shared-devnet-blocker-packet.sh \
  --window-id shared-devnet-20260324-07 \
  --candidate-bundle "$current_bundle" \
  --candidate-gate-summary "$smoke_root/evidence/current-gate.md" \
  --access-out "$smoke_root/shared-access-pass.md" \
  --mixed-topology-out "$pass_mixed_topology_out" \
  --rollback-out "$smoke_root/rollback-pass.md" \
  --mixed-topology-baseline-ref "$smoke_root/evidence/mixed-topology-baseline.md" \
  --mixed-topology-shared-evidence-ref "$smoke_root/evidence/mixed-topology-shared.md" \
  --mixed-topology-pass-decision-ref "$smoke_root/evidence/mixed-topology-pass-decision.md" \
  --mixed-topology-lane-result pass

ensure_file_contains "$pass_mixed_topology_out" 'mixed-topology-pass-decision.md'
ensure_file_contains "$pass_mixed_topology_out" '`pass_uplift_decision_ref`:'

echo "shared-devnet blocker packet smoke checks passed"
