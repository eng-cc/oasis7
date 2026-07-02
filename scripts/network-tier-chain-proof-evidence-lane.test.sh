#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

valid_evidence="$TMPDIR/chain-proof-evidence-valid.json"
lanes="$TMPDIR/lanes.tsv"
out_dir="$TMPDIR/readiness"
summary="$TMPDIR/summary.json"
stderr="$TMPDIR/stderr.txt"

cat >"$valid_evidence" <<'JSON'
{
  "evidence_schema": "oasis7.chain_proof_evidence.v1",
  "proof_contract": "WorldHeadProofV1",
  "observed_at_unix_ms": 1772467200000,
  "node_id": "validator-a",
  "network_tier": {
    "tier": "public_testnet",
    "status": "rehearsal",
    "chain_id": "oasis7-public-testnet-example",
    "network_id": "oasis7-public-testnet-example"
  },
  "proof_closure_status": "proof_complete",
  "world_head_proof_v1": {
    "schema_version": 1,
    "world_id": "world-a",
    "height": 42,
    "timestamp_ms": 1772467200000,
    "proof_hash": "proof-hash-42",
    "world_head_proof_ref": "proof-ref-42",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness"
  },
  "readiness_linkage": {
    "readiness_status": "partial",
    "failed_gates": ["runtime_bootstrap"],
    "network_height_lag": 0,
    "state_sync_fallback_required": false
  },
  "does_not_claim": [
    "module_full",
    "integration_required",
    "release_full",
    "public_testnet ready",
    "ready_for_live_candidate",
    "mainnet-grade"
  ],
  "residual_risk": [
    "sampled proof evidence is not a light-client or mainnet finality proof"
  ]
}
JSON

cat >"$lanes" <<EOF
chain_proof_evidence_ready	blockchain_ops_engineer	pass	$valid_evidence	optional proof lane must not promote public_testnet readiness by itself
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$lanes" \
  --out-dir "$out_dir" >"$summary"

python3 - "$summary" "$valid_evidence" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
evidence = str(pathlib.Path(sys.argv[2]).resolve())
ignored = summary.get("ignored_lanes", [])
if summary.get("readiness_verdict") == "ready_for_live_candidate":
    raise SystemExit("optional chain proof lane must not promote readiness")
if summary.get("live_candidate_allowed") is not False:
    raise SystemExit("optional chain proof lane must keep live_candidate_allowed=false")
if "chain_proof_evidence_ready" in summary.get("required_lanes", []):
    raise SystemExit("chain proof lane must not be required for public_testnet promotion")
matches = [lane for lane in ignored if lane.get("lane_id") == "chain_proof_evidence_ready"]
if len(matches) != 1:
    raise SystemExit(f"expected one ignored chain proof lane, got {ignored}")
if matches[0].get("resolved_evidence_path") != evidence:
    raise SystemExit(f"expected resolved evidence path {evidence}, got {matches[0]}")
PY

run_invalid_case() {
  local case_name=$1
  local expected_error=$2
  local invalid_evidence="$TMPDIR/chain-proof-evidence-invalid-$case_name.json"
  python3 - "$valid_evidence" "$invalid_evidence" "$case_name" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
case_name = sys.argv[3]

if case_name == "missing_denials":
    data["does_not_claim"] = ["module_full"]
elif case_name == "missing_scope_denials":
    data["does_not_claim"] = [
        "public_testnet ready",
        "ready_for_live_candidate",
        "mainnet-grade",
    ]
elif case_name == "wrong_boundary":
    data["world_head_proof_v1"]["claim_boundary"] = "mainnet_ready"
elif case_name == "missing_proof_ref":
    data["world_head_proof_v1"]["world_head_proof_ref"] = ""
elif case_name == "missing_proof_hash":
    data["world_head_proof_v1"]["proof_hash"] = ""
elif case_name == "zero_height":
    data["world_head_proof_v1"]["height"] = 0
elif case_name == "partial_closure":
    data["proof_closure_status"] = "proof_partial"
elif case_name == "missing_failed_gates":
    data["readiness_linkage"]["failed_gates"] = "runtime_bootstrap"
elif case_name == "empty_residual_risk":
    data["residual_risk"] = []
else:
    raise SystemExit(f"unknown invalid case: {case_name}")

pathlib.Path(sys.argv[2]).write_text(
    json.dumps(data, indent=2) + "\n",
    encoding="utf-8",
)
PY

  cat >"$lanes" <<EOF
chain_proof_evidence_ready	blockchain_ops_engineer	pass	$invalid_evidence	invalid proof lane must fail closed
EOF

  if ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json \
    --lanes-tsv "$lanes" \
    --out-dir "$TMPDIR/readiness-invalid-$case_name" \
    >"$TMPDIR/invalid-summary-$case_name.json" 2>"$stderr"; then
    echo "expected invalid chain proof evidence lane to fail: $case_name" >&2
    exit 1
  fi

  grep -q "$expected_error" "$stderr"
}

run_invalid_case "missing_denials" "chain_proof_evidence_ready does_not_claim missing"
run_invalid_case "missing_scope_denials" "chain_proof_evidence_ready does_not_claim missing"
run_invalid_case "wrong_boundary" "chain_proof_evidence_ready claim_boundary mismatch"
run_invalid_case "missing_proof_ref" "chain_proof_evidence_ready world_head_proof_v1.world_head_proof_ref missing"
run_invalid_case "missing_proof_hash" "chain_proof_evidence_ready world_head_proof_v1.proof_hash missing"
run_invalid_case "zero_height" "chain_proof_evidence_ready proof height must be positive"
run_invalid_case "partial_closure" "chain_proof_evidence_ready proof_closure_status must be proof_complete"
run_invalid_case "missing_failed_gates" "chain_proof_evidence_ready readiness_linkage.failed_gates must be an array"
run_invalid_case "empty_residual_risk" "chain_proof_evidence_ready residual_risk must be a non-empty array"
