#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

authority=doc/testing/evidence/legacy-shared-devnet-provenance-2026-07-26.md
candidate=doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json
lanes=doc/testing/evidence/shared-network-shared-devnet-live-reset-lanes-2026-05-23.tsv

[[ -f "$authority" ]]
jq -e '
  .schema_version == "oasis7.release_candidate_bundle.v1" and
  .candidate_id == "shared-devnet-live-reset-20260523-01" and
  .track == "shared_devnet" and
  (.git_commit | type == "string" and length > 0) and
  (.runtime_build.sha256 | type == "string" and length == 64) and
  (.world_snapshot.sha256_tree | type == "string" and length == 64) and
  (.governance_manifest.sha256 | type == "string" and length == 64) and
  (.evidence_refs | type == "array" and length == 3) and
  all(.evidence_refs[]; (.sha256 | type == "string" and length == 64))
' "$candidate" >/dev/null

required_text=(
  'shared_devnet-20260523-191122'
  'shared_devnet-20260523-191232'
  'shared_devnet-20260523-194826'
  'shared_devnet-20260523-214249'
  'shared_devnet-20260524-101652'
  'pass` / `eligible_for_promotion'
  'candidate_bundle_integrity'
  'shared_access'
  'multi_entry_closure'
  'mixed_topology_baseline'
  'governance_live_drill'
  'short_window_longrun'
  'rollback_target_ready'
  'shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md'
  'shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md'
  'shared-network-shared-devnet-rollback-contract-2026-05-23.md'
  'historical/rehearsal record only'
)

for needle in "${required_text[@]}"; do
  rg -F --quiet "$needle" "$authority"
done

expected_lanes=(
  candidate_bundle_integrity
  shared_access
  multi_entry_closure
  mixed_topology_baseline
  governance_live_drill
  short_window_longrun
  rollback_target_ready
)
lane_count=$(awk -F $'\t' '$1 !~ /^#/ && NF { count += 1 } END { print count + 0 }' "$lanes")
[[ "$lane_count" -eq 7 ]]
for lane in "${expected_lanes[@]}"; do
  lane_row=$(awk -F $'\t' -v lane="$lane" '$1 == lane { print $3 "\t" $4 }' "$lanes")
  [[ $(printf '%s\n' "$lane_row" | wc -l | tr -d ' ') -eq 1 ]]
  IFS=$'\t' read -r status evidence_path <<< "$lane_row"
  [[ "$status" == "pass" ]]
  [[ -f "$evidence_path" ]]
done

capture_trees=(
  5ec4b20bd7a6a28836f980ac87ed38cf2fddf9fe
  9b97eb05d6aaf94b18562811172a798ef7689f53
  d60af4f4a0a44822fbb5dd9892e491bf32e44288
  6c4de3a840875f931e21c9398e3454e2c18e4641
  9c89918b1d4542ff7f38adbfd8f030f0cee07858
)
for tree in "${capture_trees[@]}"; do
  git cat-file -e "${tree}^{tree}"
done

if rg -n 'generated-shared-network-gates/shared_devnet-' doc scripts --glob '!legacy-shared-devnet-provenance-smoke.sh'; then
  echo "error: active generated shared_devnet capture reference remains" >&2
  exit 1
fi

echo "legacy-shared-devnet-provenance-smoke: OK"
