#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

manifest_path="$tmpdir/public-testnet-smoke.json"
legacy_gate_manifest_path="$tmpdir/public-testnet-legacy-gate-smoke.json"
out_dir="$tmpdir/readiness"
bundle_path="$tmpdir/public-testnet-smoke-bundle.json"
skeleton_lanes_tsv="$tmpdir/public-testnet-skeleton-lanes.tsv"
ready_lanes_tsv="$tmpdir/public-testnet-ready-lanes.tsv"
runtime_block_lanes_tsv="$tmpdir/public-testnet-runtime-block-lanes.tsv"
template_pass_lanes_tsv="$tmpdir/public-testnet-template-pass-lanes.tsv"
old_skeleton_pass_lanes_tsv="$tmpdir/public-testnet-old-skeleton-pass-lanes.tsv"
legacy_extra_lane_tsv="$tmpdir/public-testnet-legacy-extra-lane.tsv"
public_rpc_evidence="$tmpdir/public-rpc-ready.md"
explorer_evidence="$tmpdir/explorer-public-ready.md"
faucet_evidence="$tmpdir/faucet-guard-ready.md"
reset_policy_evidence="$tmpdir/reset-policy-announced.md"
runtime_bootstrap_evidence="$tmpdir/runtime-bootstrap-ready.md"
claims_boundary_evidence="$tmpdir/claims-boundary-review.md"
world_resource_provenance_evidence="$tmpdir/world-resource-provenance-ready.md"
provider_resource_provenance_evidence="$tmpdir/provider-resource-provenance-ready.md"
resource_delta_replay_evidence="$tmpdir/resource-delta-replay-ready.md"
api_viewer_projection_evidence="$tmpdir/api-viewer-projection-ready.md"
api_viewer_projection_vacuous_evidence="$tmpdir/api-viewer-projection-vacuous.json"
old_skeleton_evidence="$tmpdir/public-testnet-skeleton-example.md"
legacy_coarse_gate_evidence="$tmpdir/public-testnet-rehearsal-coarse-gate.md"

latest_summary() {
  local scenario_dir=$1
  ls -1dt "$scenario_dir"/public-testnet-* | head -n 1
}

replace_literal() {
  local path=$1
  local needle=$2
  local replacement=$3
  python3 - "$path" "$needle" "$replacement" <<'PY'
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
needle = sys.argv[2]
replacement = sys.argv[3]
text = path.read_text(encoding="utf-8")
if needle not in text:
    raise SystemExit(f"needle not found in {path}: {needle!r}")
path.write_text(text.replace(needle, replacement), encoding="utf-8")
PY
}

cat >"$bundle_path" <<'EOF'
{"bundle":"public-testnet-smoke"}
EOF

cat >"$public_rpc_evidence" <<'EOF'
# public RPC ready evidence

- endpoint: `https://public-testnet-live-candidate.oasis7.network/rpc`
- note: smoke-only public rpc lane evidence
EOF

cat >"$explorer_evidence" <<'EOF'
# explorer public ready evidence

- endpoint: `https://public-testnet-live-candidate.oasis7.network/explorer`
- note: smoke-only explorer lane evidence
EOF

cat >"$faucet_evidence" <<'EOF'
# faucet guard ready evidence

- endpoint: `https://public-testnet-live-candidate.oasis7.network/faucet`
- note: smoke-only faucet lane evidence
EOF

cat >"$reset_policy_evidence" <<'EOF'
# reset policy announced evidence

- policy: `resettable`
- note: smoke-only reset policy lane evidence
EOF

cat >"$runtime_bootstrap_evidence" <<'EOF'
# runtime bootstrap ready evidence

- status: `boot_ok`
- note: smoke-only runtime bootstrap lane evidence
EOF

cat >"$claims_boundary_evidence" <<'EOF'
# claims boundary review evidence

- allowed: `public_testnet`
- denied: `mainnet_live`
- note: smoke-only claims boundary lane evidence
EOF

cat >"$world_resource_provenance_evidence" <<'EOF'
# world resource provenance ready evidence

- world_id: `public-testnet-smoke`
- chain_id: `oasis7-public-testnet-smoke`
- note: smoke-only world resource provenance lane evidence
EOF

cat >"$provider_resource_provenance_evidence" <<'EOF'
# provider resource provenance ready evidence

- provider_manifest: `smoke-provider-manifest`
- note: smoke-only provider resource provenance lane evidence
EOF

cat >"$resource_delta_replay_evidence" <<'EOF'
# resource delta replay ready evidence

- replay_status: `pass`
- note: smoke-only resource delta replay lane evidence
EOF

cat >"$api_viewer_projection_evidence" <<'EOF'
{
  "api_viewer_projection": {
    "status": "pass",
    "same_window_required": true,
    "chain_status_samples_ref": "output/s10/timeline.csv",
    "api_projection_ref": "output/api/projection.json",
    "viewer_projection_ref": "output/viewer/projection.json",
    "world_state_projection_match": true
  }
}
EOF

cat >"$api_viewer_projection_vacuous_evidence" <<'EOF'
{
  "api_viewer_projection": {
    "status": "pass",
    "same_window_required": true,
    "world_state_projection_match": true
  }
}
EOF

cat >"$legacy_coarse_gate_evidence" <<'EOF'
# public testnet rehearsal coarse gate evidence

- note: smoke-only compatibility lane evidence
EOF

cat >"$old_skeleton_evidence" <<'EOF'
# old public testnet skeleton placeholder

- verdict: specified_skeleton_only
EOF

./scripts/network-tier-manifest.sh create \
  --manifest "$manifest_path" \
  --tier public_testnet \
  --status specified_skeleton_only \
  --network-id oasis7-public-testnet-smoke \
  --chain-id oasis7-public-testnet-smoke \
  --release-candidate-bundle-ref "$bundle_path" \
  --genesis-ref doc/testing/templates/public-testnet-genesis.example.json \
  --bootstrap-peer-ref doc/testing/templates/public-testnet-bootstrap.example.txt \
  --rpc-ref https://public-testnet.example.invalid/rpc \
  --explorer-ref https://public-testnet.example.invalid/explorer \
  --faucet-ref https://public-testnet.example.invalid/faucet \
  --governance-mode shared_ops \
  --validator-admission allowlist_or_governed_candidate \
  --target-validator-count 4 \
  --allow-observer-nodes true \
  --token-symbol OC \
  --faucet-mode guarded_testnet_faucet \
  --reset-policy resettable \
  --value-semantics testnet \
  --promote-from governed_bootstrap_rehearsal \
  --require-gate public_rpc_ready \
  --require-gate explorer_public_ready \
  --require-gate faucet_guard_ready \
  --require-gate reset_policy_announced \
  --require-gate runtime_bootstrap \
  --require-gate world_resource_provenance_ready \
  --require-gate provider_resource_provenance_ready \
  --require-gate resource_delta_replay_ready \
  --require-gate api_viewer_projection_ready \
  --require-gate claims_boundary_review \
  --allowed-claim public_testnet \
  --denied-claim mainnet_live \
  --denied-claim production_oc_settlement \
  --evidence-ref doc/testing/templates/public-testnet-skeleton-evidence.example.md >/dev/null

cat >"$skeleton_lanes_tsv" <<'EOF'
public_rpc_ready	runtime_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder rpc evidence
explorer_public_ready	runtime_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder explorer evidence
faucet_guard_ready	liveops_community	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder faucet evidence
reset_policy_announced	liveops_community	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder reset evidence
runtime_bootstrap	runtime_engineer	partial	doc/testing/templates/public-testnet-rehearsal-template.md	template bootstrap evidence
world_resource_provenance_ready	blockchain_ops_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder world resource provenance evidence
provider_resource_provenance_ready	agent_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder provider resource provenance evidence
resource_delta_replay_ready	runtime_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder resource delta replay evidence
api_viewer_projection_ready	viewer_engineer	partial	doc/testing/templates/public-testnet-skeleton-evidence.example.md	placeholder api/viewer projection evidence
claims_boundary_review	qa_engineer	partial	doc/testing/templates/public-testnet-exit-review-template.md	template claims evidence
EOF

cat >"$ready_lanes_tsv" <<'EOF'
public_rpc_ready	runtime_engineer	pass	PUBLIC_RPC_EVIDENCE	public rpc ready
explorer_public_ready	runtime_engineer	pass	EXPLORER_EVIDENCE	explorer ready
faucet_guard_ready	liveops_community	pass	FAUCET_EVIDENCE	faucet guard ready
reset_policy_announced	liveops_community	pass	RESET_POLICY_EVIDENCE	reset policy announced
runtime_bootstrap	runtime_engineer	pass	RUNTIME_BOOTSTRAP_EVIDENCE	runtime bootstrap ready
world_resource_provenance_ready	blockchain_ops_engineer	pass	WORLD_RESOURCE_PROVENANCE_EVIDENCE	world resource provenance ready
provider_resource_provenance_ready	agent_engineer	pass	PROVIDER_RESOURCE_PROVENANCE_EVIDENCE	provider resource provenance ready
resource_delta_replay_ready	runtime_engineer	pass	RESOURCE_DELTA_REPLAY_EVIDENCE	resource delta replay ready
api_viewer_projection_ready	viewer_engineer	pass	API_VIEWER_PROJECTION_EVIDENCE	api/viewer projection ready
claims_boundary_review	qa_engineer	pass	CLAIMS_BOUNDARY_EVIDENCE	claims boundary reviewed
EOF

replace_literal "$ready_lanes_tsv" "PUBLIC_RPC_EVIDENCE" "$public_rpc_evidence"
replace_literal "$ready_lanes_tsv" "EXPLORER_EVIDENCE" "$explorer_evidence"
replace_literal "$ready_lanes_tsv" "FAUCET_EVIDENCE" "$faucet_evidence"
replace_literal "$ready_lanes_tsv" "RESET_POLICY_EVIDENCE" "$reset_policy_evidence"
replace_literal "$ready_lanes_tsv" "RUNTIME_BOOTSTRAP_EVIDENCE" "$runtime_bootstrap_evidence"
replace_literal "$ready_lanes_tsv" "WORLD_RESOURCE_PROVENANCE_EVIDENCE" "$world_resource_provenance_evidence"
replace_literal "$ready_lanes_tsv" "PROVIDER_RESOURCE_PROVENANCE_EVIDENCE" "$provider_resource_provenance_evidence"
replace_literal "$ready_lanes_tsv" "RESOURCE_DELTA_REPLAY_EVIDENCE" "$resource_delta_replay_evidence"
replace_literal "$ready_lanes_tsv" "API_VIEWER_PROJECTION_EVIDENCE" "$api_viewer_projection_evidence"
replace_literal "$ready_lanes_tsv" "CLAIMS_BOUNDARY_EVIDENCE" "$claims_boundary_evidence"
cp "$ready_lanes_tsv" "$runtime_block_lanes_tsv"
replace_literal "$runtime_block_lanes_tsv" $'runtime_bootstrap\truntime_engineer\tpass\t' $'runtime_bootstrap\truntime_engineer\tblock\t'

./scripts/network-tier-manifest.sh validate --manifest "$manifest_path" >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet-rehearsal.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null
./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --out-dir "$out_dir/example-skeleton" >/dev/null
jq -e '.readiness_verdict == "specified_skeleton_only" and (.missing_required_lanes | length) == 10' \
  "$(latest_summary "$out_dir/example-skeleton")/summary.json" >/dev/null

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --out-dir "$out_dir/smoke-skeleton" >/dev/null
jq -e '.readiness_verdict == "specified_skeleton_only" and (.missing_required_lanes | length) == 10' \
  "$(latest_summary "$out_dir/smoke-skeleton")/summary.json" >/dev/null

python3 - <<'PY' "$manifest_path"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["status"] = "rehearsal"
data["endpoint_policy"]["rpc_ref"] = "https://public-testnet-live-candidate.oasis7.network/rpc"
data["endpoint_policy"]["explorer_ref"] = "https://public-testnet-live-candidate.oasis7.network/explorer"
data["endpoint_policy"]["faucet_ref"] = "https://public-testnet-live-candidate.oasis7.network/faucet"
path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --out-dir "$out_dir/no-lanes-block" >/dev/null
jq -e '.readiness_verdict == "block" and (.missing_required_lanes | length) == 10' \
  "$(latest_summary "$out_dir/no-lanes-block")/summary.json" >/dev/null

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$skeleton_lanes_tsv" \
  --out-dir "$out_dir/partial-lanes" >/dev/null
jq -e '.readiness_verdict == "partial" and (.missing_required_lanes | length) == 0 and (.manifest_blockers | length) == 0' \
  "$(latest_summary "$out_dir/partial-lanes")/summary.json" >/dev/null

python3 - <<'PY' "$manifest_path"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["endpoint_policy"]["rpc_ref"] = "http://127.0.0.1:8545/rpc"
path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$ready_lanes_tsv" \
  --out-dir "$out_dir/non-public-endpoint-block" >/dev/null
jq -e '.readiness_verdict == "block" and (.manifest_blockers | any(. == "rpc_ref_non_public:http://127.0.0.1:8545/rpc"))' \
  "$(latest_summary "$out_dir/non-public-endpoint-block")/summary.json" >/dev/null

python3 - <<'PY' "$manifest_path"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["endpoint_policy"]["rpc_ref"] = "https://public-testnet-live-candidate.oasis7.network/rpc"
path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$runtime_block_lanes_tsv" \
  --out-dir "$out_dir/runtime-bootstrap-block" >/dev/null
jq -e '.readiness_verdict == "block" and .live_candidate_allowed == false and (.blocking_lanes | any(.lane_id == "runtime_bootstrap"))' \
  "$(latest_summary "$out_dir/runtime-bootstrap-block")/summary.json" >/dev/null

cp "$ready_lanes_tsv" "$template_pass_lanes_tsv"
replace_literal "$template_pass_lanes_tsv" "$runtime_bootstrap_evidence" "doc/testing/templates/public-testnet-rehearsal-template.md"
if ./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$template_pass_lanes_tsv" \
  --out-dir "$out_dir/template-pass-rejected" >"$tmpdir/template-pass.stdout" 2>"$tmpdir/template-pass.stderr"; then
  echo "expected template pass evidence to be rejected" >&2
  exit 1
fi
grep -q "pass evidence cannot use placeholder/template ref" "$tmpdir/template-pass.stderr"

cp "$ready_lanes_tsv" "$old_skeleton_pass_lanes_tsv"
replace_literal "$old_skeleton_pass_lanes_tsv" "$public_rpc_evidence" "$old_skeleton_evidence"
if ./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$old_skeleton_pass_lanes_tsv" \
  --out-dir "$out_dir/old-skeleton-pass-rejected" >"$tmpdir/old-skeleton-pass.stdout" 2>"$tmpdir/old-skeleton-pass.stderr"; then
  echo "expected old skeleton pass evidence to be rejected" >&2
  exit 1
fi
grep -q "pass evidence cannot use placeholder/template ref" "$tmpdir/old-skeleton-pass.stderr"

cp "$ready_lanes_tsv" "$old_skeleton_pass_lanes_tsv"
replace_literal "$old_skeleton_pass_lanes_tsv" "$api_viewer_projection_evidence" "$api_viewer_projection_vacuous_evidence"
if ./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$old_skeleton_pass_lanes_tsv" \
  --out-dir "$out_dir/api-viewer-vacuous-pass-rejected" >"$tmpdir/api-viewer-vacuous-pass.stdout" 2>"$tmpdir/api-viewer-vacuous-pass.stderr"; then
  echo "expected vacuous API/viewer projection pass evidence to be rejected" >&2
  exit 1
fi
grep -q "api_viewer_projection_ready" "$tmpdir/api-viewer-vacuous-pass.stderr"

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$ready_lanes_tsv" \
  --out-dir "$out_dir/ready-lanes" >/dev/null
jq -e '.readiness_verdict == "ready_for_live_candidate" and .live_candidate_allowed == true' \
  "$(latest_summary "$out_dir/ready-lanes")/summary.json" >/dev/null

cp "$ready_lanes_tsv" "$legacy_extra_lane_tsv"
cat >>"$legacy_extra_lane_tsv" <<EOF
public_testnet_rehearsal_pass	qa_engineer	partial	$legacy_coarse_gate_evidence	legacy lane retained for historical trace only
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$legacy_extra_lane_tsv" \
  --out-dir "$out_dir/legacy-extra-lane-ignored" >/dev/null
jq -e '.readiness_verdict == "ready_for_live_candidate" and .live_candidate_allowed == true and (.ignored_lanes | any(.lane_id == "public_testnet_rehearsal_pass")) and (.partial_lanes | length) == 0' \
  "$(latest_summary "$out_dir/legacy-extra-lane-ignored")/summary.json" >/dev/null

cp "$manifest_path" "$legacy_gate_manifest_path"
python3 - <<'PY' "$legacy_gate_manifest_path"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["promotion_policy"]["required_gates"].insert(0, "public_testnet_rehearsal_pass")
path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$legacy_gate_manifest_path" \
  --lanes-tsv "$ready_lanes_tsv" \
  --out-dir "$out_dir/legacy-manifest-gate-block" >/dev/null
jq -e '.readiness_verdict == "block" and .live_candidate_allowed == false and (.manifest_blockers | any(. == "manifest_declares_unsupported_required_gates:public_testnet_rehearsal_pass"))' \
  "$(latest_summary "$out_dir/legacy-manifest-gate-block")/summary.json" >/dev/null

echo "network-tier-manifest smoke passed"
