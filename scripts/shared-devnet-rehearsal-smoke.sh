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

smoke_root=".tmp/shared_devnet_rehearsal_smoke"
rm -rf "$smoke_root"
mkdir -p "$smoke_root/runtime" "$smoke_root/world" "$smoke_root/evidence" "$smoke_root/fallback"

printf 'runtime-build-v1\n' >"$smoke_root/runtime/runtime.bin"
printf 'snapshot\n' >"$smoke_root/world/state.txt"
printf '{"signers":["signer01"]}\n' >"$smoke_root/world/public_manifest.json"
printf '# shared endpoint\n' >"$smoke_root/evidence/shared-endpoint.md"
printf '# shared operator\n' >"$smoke_root/evidence/shared-operator.md"
printf '# web evidence\n' >"$smoke_root/evidence/web.md"
printf '# headless evidence\n' >"$smoke_root/evidence/headless.md"
printf '# pure api evidence\n' >"$smoke_root/evidence/pure-api.md"
printf '# governance evidence\n' >"$smoke_root/evidence/governance.md"
printf '# longrun evidence\n' >"$smoke_root/evidence/longrun.md"
printf '# mixed topology evidence\n' >"$smoke_root/evidence/mixed-topology.md"
printf '# mixed topology pass decision\n' >"$smoke_root/evidence/mixed-topology-pass-decision.md"
printf '# shared access evidence\n' >"$smoke_root/evidence/shared-access.md"
printf '# fallback gate\n' >"$smoke_root/evidence/fallback-gate.md"
printf '# fallback owner\n' >"$smoke_root/evidence/fallback-owner.md"
printf '# rollback restore step\n' >"$smoke_root/evidence/rollback-restore.md"
printf '{"candidate":"fallback"}\n' >"$smoke_root/fallback/pass-bundle.json"

partial_out="$smoke_root/output-partial"
run ./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-orch-smoke-partial \
  --candidate-id shared-devnet-orch-smoke-partial \
  --candidate-bundle-out "$smoke_root/shared-devnet-orch-smoke-partial.json" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree \
  --out-dir "$partial_out" \
  --release-gate-mode skip \
  --web-mode skip \
  --headless-mode skip \
  --pure-api-mode skip \
  --governance-mode skip \
  --longrun-mode skip

partial_gate=$(find "$partial_out/shared-devnet-orch-smoke-partial/gate" -mindepth 2 -maxdepth 2 -type f -name summary.json | sort | tail -n 1)
partial_lanes="$partial_out/shared-devnet-orch-smoke-partial/lanes.shared_devnet.tsv"
ensure_file_contains "$partial_gate" '"gate_result": "partial"'
ensure_file_contains "$partial_gate" '"promotion_recommendation": "hold_promotion"'
ensure_file_contains "$partial_lanes" $'multi_entry_closure\tqa_engineer\tpartial'
ensure_file_contains "$partial_lanes" $'mixed_topology_baseline\tqa_engineer\tpartial'
ensure_file_contains "$partial_lanes" $'short_window_longrun\truntime_engineer\tpartial'

if ./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-orch-smoke-missing-decision \
  --candidate-id shared-devnet-orch-smoke-missing-decision \
  --candidate-bundle-out "$smoke_root/shared-devnet-orch-smoke-missing-decision.json" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree \
  --out-dir "$smoke_root/output-missing-decision" \
  --release-gate-mode skip \
  --web-mode skip \
  --headless-mode skip \
  --pure-api-mode skip \
  --governance-mode skip \
  --longrun-mode skip \
  --mixed-topology-pass \
  --mixed-topology-shared-evidence-ref "$smoke_root/evidence/mixed-topology.md" \
  >/dev/null 2>"$smoke_root/missing-decision.stderr"; then
  echo "error: mixed-topology pass should require a pass-decision ref" >&2
  exit 1
fi
ensure_file_contains "$smoke_root/missing-decision.stderr" '--mixed-topology-pass requires --mixed-topology-pass-decision-ref'

if ./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-orch-smoke-missing-access-evidence \
  --candidate-id shared-devnet-orch-smoke-missing-access-evidence \
  --candidate-bundle-out "$smoke_root/shared-devnet-orch-smoke-missing-access-evidence.json" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree \
  --out-dir "$smoke_root/output-missing-access-evidence" \
  --release-gate-mode skip \
  --web-mode skip \
  --headless-mode skip \
  --pure-api-mode skip \
  --governance-mode skip \
  --longrun-mode skip \
  --shared-access-pass \
  --shared-endpoint-ref "$smoke_root/evidence/shared-endpoint.md" \
  --shared-operator-ref "$smoke_root/evidence/shared-operator.md" \
  >/dev/null 2>"$smoke_root/missing-access-evidence.stderr"; then
  echo "error: shared-access pass should require an access-evidence ref" >&2
  exit 1
fi
ensure_file_contains "$smoke_root/missing-access-evidence.stderr" '--shared-access-pass requires at least one --shared-endpoint-ref, one --shared-operator-ref, and one --shared-access-evidence-ref'

run ./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-orch-smoke-rollback-partial \
  --candidate-id shared-devnet-orch-smoke-rollback-partial \
  --candidate-bundle-out "$smoke_root/shared-devnet-orch-smoke-rollback-partial.json" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree \
  --out-dir "$smoke_root/output-rollback-partial" \
  --release-gate-mode skip \
  --web-mode skip \
  --headless-mode skip \
  --pure-api-mode skip \
  --governance-mode skip \
  --longrun-mode skip \
  --fallback-candidate-bundle "$smoke_root/fallback/pass-bundle.json"
ensure_file_contains "$smoke_root/output-rollback-partial/shared-devnet-orch-smoke-rollback-partial/rollback-target.md" 'fallback bundle is pinned, but audited fallback gate/owner/restore scope contract is still incomplete'

pass_out="$smoke_root/output-pass"
run ./scripts/shared-devnet-rehearsal.sh \
  --window-id shared-devnet-orch-smoke-pass \
  --candidate-id shared-devnet-orch-smoke-pass \
  --candidate-bundle-out "$smoke_root/shared-devnet-orch-smoke-pass.json" \
  --runtime-build-ref "$smoke_root/runtime/runtime.bin" \
  --world-snapshot-ref "$smoke_root/world" \
  --governance-manifest-ref "$smoke_root/world/public_manifest.json" \
  --allow-dirty-worktree \
  --out-dir "$pass_out" \
  --release-gate-mode skip \
  --web-mode evidence \
  --web-evidence-ref "$smoke_root/evidence/web.md" \
  --headless-mode evidence \
  --headless-evidence-ref "$smoke_root/evidence/headless.md" \
  --pure-api-mode evidence \
  --pure-api-evidence-ref "$smoke_root/evidence/pure-api.md" \
  --governance-mode evidence \
  --governance-window-evidence-ref "$smoke_root/evidence/governance.md" \
  --mixed-topology-pass \
  --mixed-topology-shared-evidence-ref "$smoke_root/evidence/mixed-topology.md" \
  --mixed-topology-pass-decision-ref "$smoke_root/evidence/mixed-topology-pass-decision.md" \
  --longrun-mode evidence \
  --longrun-window-evidence-ref "$smoke_root/evidence/longrun.md" \
  --shared-access-pass \
  --shared-endpoint-ref "$smoke_root/evidence/shared-endpoint.md" \
  --shared-operator-ref "$smoke_root/evidence/shared-operator.md" \
  --shared-access-evidence-ref "$smoke_root/evidence/shared-access.md" \
  --fallback-candidate-bundle "$smoke_root/fallback/pass-bundle.json" \
  --fallback-gate-ref "$smoke_root/evidence/fallback-gate.md" \
  --fallback-owner-ref "$smoke_root/evidence/fallback-owner.md" \
  --fallback-class bootstrap_restore_ready \
  --rollback-restore-step-ref "$smoke_root/evidence/rollback-restore.md" \
  --rollback-restoration-scope "runtime build | world snapshot | governance manifest"

pass_gate=$(find "$pass_out/shared-devnet-orch-smoke-pass/gate" -mindepth 2 -maxdepth 2 -type f -name summary.json | sort | tail -n 1)
pass_lanes="$pass_out/shared-devnet-orch-smoke-pass/lanes.shared_devnet.tsv"
ensure_file_contains "$pass_gate" '"gate_result": "pass"'
ensure_file_contains "$pass_gate" '"promotion_recommendation": "eligible_for_promotion"'
ensure_file_contains "$pass_lanes" $'shared_access\tqa_engineer\tpass'
ensure_file_contains "$pass_lanes" $'mixed_topology_baseline\tqa_engineer\tpass'
ensure_file_contains "$pass_lanes" $'governance_live_drill\truntime_engineer\tpass'
ensure_file_contains "$pass_lanes" $'rollback_target_ready\tliveops_community\tpass'
ensure_file_contains "$pass_out/shared-devnet-orch-smoke-pass/mixed-topology-gate.md" 'pass-uplift decision ref'
ensure_file_contains "$pass_out/shared-devnet-orch-smoke-pass/access-check.md" 'shared access evidence refs'
ensure_file_contains "$pass_out/shared-devnet-orch-smoke-pass/rollback-target.md" 'fallback gate ref'
ensure_file_contains "$pass_out/shared-devnet-orch-smoke-pass/rollback-target.md" 'restore step refs'

echo "shared-devnet rehearsal smoke checks passed"
