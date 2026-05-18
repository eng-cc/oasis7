#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

manifest_path="$tmpdir/public-testnet-smoke.json"
out_dir="$tmpdir/readiness"
bundle_path="$tmpdir/public-testnet-smoke-bundle.json"
skeleton_lanes_tsv="$tmpdir/public-testnet-skeleton-lanes.tsv"
ready_lanes_tsv="$tmpdir/public-testnet-ready-lanes.tsv"

latest_summary() {
  local scenario_dir=$1
  ls -1dt "$scenario_dir"/public-testnet-* | head -n 1
}

cat >"$bundle_path" <<'EOF'
{"bundle":"public-testnet-smoke"}
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
  --promote-from shared_devnet \
  --require-gate shared_devnet_pass \
  --require-gate public_rpc_ready \
  --require-gate faucet_guard_ready \
  --require-gate reset_policy_announced \
  --allowed-claim public_testnet \
  --denied-claim mainnet_live \
  --denied-claim production_oc_settlement \
  --evidence-ref doc/testing/evidence/public-testnet-skeleton-example.md >/dev/null

cat >"$skeleton_lanes_tsv" <<'EOF'
shared_devnet_pass	qa_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	shared devnet source
public_rpc_ready	runtime_engineer	partial	doc/testing/evidence/public-testnet-skeleton-example.md	placeholder rpc evidence
explorer_public_ready	runtime_engineer	partial	doc/testing/evidence/public-testnet-skeleton-example.md	placeholder explorer evidence
faucet_guard_ready	liveops_community	partial	doc/testing/evidence/public-testnet-skeleton-example.md	placeholder faucet evidence
reset_policy_announced	liveops_community	partial	doc/testing/evidence/public-testnet-skeleton-example.md	placeholder reset evidence
runtime_bootstrap	runtime_engineer	partial	doc/testing/templates/public-testnet-rehearsal-template.md	template bootstrap evidence
claims_boundary_review	qa_engineer	partial	doc/testing/templates/public-testnet-exit-review-template.md	template claims evidence
EOF

cat >"$ready_lanes_tsv" <<'EOF'
shared_devnet_pass	qa_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	shared devnet source
public_rpc_ready	runtime_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	public rpc ready
explorer_public_ready	runtime_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	explorer ready
faucet_guard_ready	liveops_community	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	faucet guard ready
reset_policy_announced	liveops_community	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	reset policy announced
runtime_bootstrap	runtime_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	runtime bootstrap ready
claims_boundary_review	qa_engineer	pass	doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-03-24.md	claims boundary reviewed
EOF

./scripts/network-tier-manifest.sh validate --manifest "$manifest_path" >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-shared-devnet.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-manifest.sh validate --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-public-testnet.example.json >/dev/null
./scripts/network-tier-exit-review.sh --manifest doc/testing/templates/network-tier-mainnet.example.json >/dev/null
./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --out-dir "$out_dir/example-skeleton" >/dev/null
jq -e '.readiness_verdict == "specified_skeleton_only" and (.missing_required_lanes | length) == 7' \
  "$(latest_summary "$out_dir/example-skeleton")/summary.json" >/dev/null

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --out-dir "$out_dir/smoke-skeleton" >/dev/null
jq -e '.readiness_verdict == "specified_skeleton_only" and (.missing_required_lanes | length) == 7' \
  "$(latest_summary "$out_dir/smoke-skeleton")/summary.json" >/dev/null

python3 - <<'PY' "$manifest_path"
import json
import pathlib
import sys
path = pathlib.Path(sys.argv[1])
data = json.loads(path.read_text(encoding="utf-8"))
data["status"] = "rehearsal"
data["endpoint_policy"]["rpc_ref"] = "https://live-candidate.oasis7.example/rpc"
data["endpoint_policy"]["explorer_ref"] = "https://live-candidate.oasis7.example/explorer"
data["endpoint_policy"]["faucet_ref"] = "https://live-candidate.oasis7.example/faucet"
path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --out-dir "$out_dir/no-lanes-block" >/dev/null
jq -e '.readiness_verdict == "block" and (.missing_required_lanes | length) == 7' \
  "$(latest_summary "$out_dir/no-lanes-block")/summary.json" >/dev/null

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$skeleton_lanes_tsv" \
  --out-dir "$out_dir/partial-lanes" >/dev/null
jq -e '.readiness_verdict == "partial" and (.missing_required_lanes | length) == 0 and (.manifest_blockers | length) == 0' \
  "$(latest_summary "$out_dir/partial-lanes")/summary.json" >/dev/null

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest "$manifest_path" \
  --lanes-tsv "$ready_lanes_tsv" \
  --out-dir "$out_dir/ready-lanes" >/dev/null
jq -e '.readiness_verdict == "ready_for_live_candidate" and .live_candidate_allowed == true' \
  "$(latest_summary "$out_dir/ready-lanes")/summary.json" >/dev/null

echo "network-tier-manifest smoke passed"
