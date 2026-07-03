#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

valid_evidence="$TMPDIR/chain-proof-evidence-valid.json"
valid_external_verifier_evidence="$TMPDIR/external-verifier-light-client-lite-valid.json"
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
  "external_verifier": {
    "schema_version": "oasis7.world_head_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldHeadProofV1",
    "proof_ref": "proof-ref-42",
    "proof_hash": "proof-hash-42",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "verifier_command": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --proof proof.cbor --proof-ref proof-ref-42 --expect-hash proof-hash-42 --json",
    "verified_at_unix_ms": 1772467200000,
    "does_not_claim": [
      "mainnet-grade finality",
      "state proof",
      "receipt proof",
      "DA sampling",
      "full light client"
    ]
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
external_verifier_light_client_lite_ready	blockchain_ops_engineer	pass	$valid_external_verifier_evidence	optional external verifier lane must not promote public_testnet readiness by itself
EOF

cat >"$valid_external_verifier_evidence" <<'JSON'
{
  "evidence_schema": "oasis7.external_verifier_light_client_lite.v1",
  "status": "pass",
  "verifier_mode": "external_light_client_lite",
  "independent_process": true,
  "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
  "command_ref": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --proof proof.cbor --expect-hash proof-hash-42 --json",
  "network_tier": {
    "tier": "public_testnet",
    "status": "rehearsal",
    "chain_id": "oasis7-public-testnet-example",
    "network_id": "oasis7-public-testnet-example",
    "world_id": "world-a"
  },
  "manifest_ref": "doc/testing/templates/network-tier-public-testnet.example.json",
  "genesis_ref": "doc/testing/templates/public-testnet-genesis.example.json",
  "bootstrap_peer_ref": "doc/testing/templates/public-testnet-bootstrap.example.txt",
  "rpc_ref": "https://public-testnet.example.invalid/rpc",
  "sample_window": {
    "started_at": "2026-07-03T00:00:00Z",
    "ended_at": "2026-07-03T00:01:00Z"
  },
  "observed_head": {
    "height": 42,
    "hash": "proof-hash-42",
    "state_root": "state-root-42"
  },
  "verified_range": {
    "from_height": 40,
    "to_height": 42
  },
  "verification_result": "accepted",
  "proof_ref": "proof-ref-42",
  "proof_hash": "proof-hash-42",
  "external_verifier": {
    "schema_version": "oasis7.world_head_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldHeadProofV1",
    "hash_domain": "oasis7.world_head_proof.v1",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "proof_ref": "proof-ref-42",
    "proof_hash": "proof-hash-42",
    "world_id": "world-a",
    "height": 42,
    "head": {
      "block_hash": "proof-hash-42",
      "state_root": "state-root-42"
    },
    "checkpoint_bound": true,
    "does_not_claim": [
      "mainnet-grade finality",
      "state proof",
      "receipt proof",
      "DA sampling",
      "full light client"
    ]
  },
  "node_db_access_used": false,
  "manual_checkpoint_or_data_copy_used": false,
  "privileged_internal_api_used": false,
  "does_not_claim": [
    "mainnet-grade",
    "production OC settlement",
    "public validator onboarding open",
    "multi-client consensus equivalence",
    "full light client security",
    "ready_for_live_candidate"
  ],
  "residual_risk": [
    "external verifier is light-client-lite only and does not prove full consensus security"
  ]
}
JSON

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
for optional_lane in ("chain_proof_evidence_ready", "external_verifier_light_client_lite_ready"):
    if optional_lane in summary.get("required_lanes", []):
        raise SystemExit(f"{optional_lane} must not be required for public_testnet promotion")
matches = [lane for lane in ignored if lane.get("lane_id") == "chain_proof_evidence_ready"]
if len(matches) != 1:
    raise SystemExit(f"expected one ignored chain proof lane, got {ignored}")
if matches[0].get("resolved_evidence_path") != evidence:
    raise SystemExit(f"expected resolved evidence path {evidence}, got {matches[0]}")
external_matches = [
    lane for lane in ignored if lane.get("lane_id") == "external_verifier_light_client_lite_ready"
]
if len(external_matches) != 1:
    raise SystemExit(f"expected one ignored external verifier lane, got {ignored}")
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
elif case_name == "missing_external_verifier":
    data.pop("external_verifier", None)
elif case_name == "verifier_failed":
    data["external_verifier"]["status"] = "fail"
elif case_name == "verifier_hash_mismatch":
    data["external_verifier"]["proof_hash"] = "wrong-proof-hash"
elif case_name == "verifier_missing_denials":
    data["external_verifier"]["does_not_claim"] = ["full light client"]
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
run_invalid_case "missing_external_verifier" "chain_proof_evidence_ready external_verifier object missing"
run_invalid_case "verifier_failed" "chain_proof_evidence_ready external_verifier.status must be pass"
run_invalid_case "verifier_hash_mismatch" "chain_proof_evidence_ready external_verifier.proof_hash must match proof hash"
run_invalid_case "verifier_missing_denials" "chain_proof_evidence_ready external_verifier.does_not_claim missing"
run_invalid_case "missing_failed_gates" "chain_proof_evidence_ready readiness_linkage.failed_gates must be an array"
run_invalid_case "empty_residual_risk" "chain_proof_evidence_ready residual_risk must be a non-empty array"

generated_proof="$TMPDIR/generated-world-head-proof.json"
generated_proof_summary="$TMPDIR/generated-world-head-proof-summary.json"
generated_external_evidence="$TMPDIR/generated-external-verifier-evidence.json"

cargo run -p oasis7_proto --quiet --bin oasis7_world_head_proof_verify -- \
  --write-sample-json "$generated_proof" \
  --json >"$generated_proof_summary"

generated_head_hash=$(jq -r '.head.block_hash' "$generated_proof_summary")
generated_state_root=$(jq -r '.head.state_root' "$generated_proof_summary")

./scripts/network-tier-external-verifier-light-client-lite.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof "$generated_proof" \
  --proof-format json \
  --proof-ref proof-ref-42 \
  --world-id world-a \
  --expect-height 42 \
  --observed-head-hash "$generated_head_hash" \
  --observed-state-root "$generated_state_root" \
  --from-height 40 \
  --sample-started-at 2026-07-03T00:00:00Z \
  --sample-ended-at 2026-07-03T00:01:00Z \
  --out "$generated_external_evidence" >/dev/null

jq -e '.evidence_schema == "oasis7.external_verifier_light_client_lite.v1" and .verification_result == "accepted" and .observed_head.hash == "'"$generated_head_hash"'" and .observed_head.state_root == "'"$generated_state_root"'"' \
  "$generated_external_evidence" >/dev/null

cat >"$lanes" <<EOF
external_verifier_light_client_lite_ready	blockchain_ops_engineer	pass	$generated_external_evidence	generated external verifier lane must pass readiness validation
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$lanes" \
  --out-dir "$TMPDIR/readiness-generated-external" >"$TMPDIR/generated-external-summary.json"

jq -e '.live_candidate_allowed == false and (.ignored_lanes | any(.lane_id == "external_verifier_light_client_lite_ready"))' \
  "$TMPDIR/generated-external-summary.json" >/dev/null

if ./scripts/network-tier-external-verifier-light-client-lite.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof "$generated_proof" \
  --proof-format json \
  --proof-ref proof-ref-42 \
  --world-id world-a \
  --expect-height 42 \
  --observed-head-hash wrong-head-hash \
  --observed-state-root "$generated_state_root" \
  --from-height 40 \
  --sample-started-at 2026-07-03T00:00:00Z \
  --sample-ended-at 2026-07-03T00:01:00Z \
  --out "$TMPDIR/should-not-write-hash-mismatch.json" >"$TMPDIR/hash-mismatch.stdout" 2>"$stderr"; then
  echo "expected wrapper to reject observed head hash mismatch" >&2
  exit 1
fi
grep -q "observed head hash mismatch" "$stderr"

if ./scripts/network-tier-external-verifier-light-client-lite.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof "$generated_proof" \
  --proof-format json \
  --proof-ref proof-ref-42 \
  --world-id world-a \
  --expect-height 42 \
  --observed-head-hash "$generated_head_hash" \
  --observed-state-root wrong-state-root \
  --from-height 40 \
  --sample-started-at 2026-07-03T00:00:00Z \
  --sample-ended-at 2026-07-03T00:01:00Z \
  --out "$TMPDIR/should-not-write-state-mismatch.json" >"$TMPDIR/state-mismatch.stdout" 2>"$stderr"; then
  echo "expected wrapper to reject observed state root mismatch" >&2
  exit 1
fi
grep -q "observed state root mismatch" "$stderr"

run_invalid_external_case() {
  local case_name=$1
  local expected_error=$2
  local invalid_evidence="$TMPDIR/external-verifier-invalid-$case_name.json"
  python3 - "$valid_external_verifier_evidence" "$invalid_evidence" "$case_name" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
case_name = sys.argv[3]

if case_name == "network_mismatch":
    data["network_tier"]["network_id"] = "other-network"
elif case_name == "manifest_mismatch":
    data["manifest_ref"] = "doc/testing/templates/network-tier-mainnet.example.json"
elif case_name == "db_access":
    data["node_db_access_used"] = True
elif case_name == "range_too_short":
    data["verified_range"]["to_height"] = 41
elif case_name == "missing_verifier":
    data.pop("external_verifier", None)
elif case_name == "verifier_head_hash_mismatch":
    data["external_verifier"]["head"]["block_hash"] = "wrong-head-hash"
elif case_name == "verifier_state_root_mismatch":
    data["external_verifier"]["head"]["state_root"] = "wrong-state-root"
elif case_name == "verifier_proof_hash_mismatch":
    data["external_verifier"]["proof_hash"] = "wrong-proof-hash"
elif case_name == "missing_denials":
    data["does_not_claim"] = ["ready_for_live_candidate"]
else:
    raise SystemExit(f"unknown invalid external verifier case: {case_name}")

pathlib.Path(sys.argv[2]).write_text(
    json.dumps(data, indent=2) + "\n",
    encoding="utf-8",
)
PY

  cat >"$lanes" <<EOF
external_verifier_light_client_lite_ready	blockchain_ops_engineer	pass	$invalid_evidence	invalid external verifier lane must fail closed
EOF

  if ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json \
    --lanes-tsv "$lanes" \
    --out-dir "$TMPDIR/readiness-invalid-external-$case_name" \
    >"$TMPDIR/invalid-external-summary-$case_name.json" 2>"$stderr"; then
    echo "expected invalid external verifier lane to fail: $case_name" >&2
    exit 1
  fi

  grep -q "$expected_error" "$stderr"
}

run_invalid_external_case "network_mismatch" "external_verifier_light_client_lite_ready network_tier.network_id must match manifest"
run_invalid_external_case "manifest_mismatch" "external_verifier_light_client_lite_ready manifest_ref must match manifest"
run_invalid_external_case "db_access" "external_verifier_light_client_lite_ready node_db_access_used must be false"
run_invalid_external_case "range_too_short" "external_verifier_light_client_lite_ready verified_range.to_height must be >= observed_head.height"
run_invalid_external_case "missing_verifier" "external_verifier_light_client_lite_ready external_verifier object missing"
run_invalid_external_case "verifier_head_hash_mismatch" "external_verifier_light_client_lite_ready external_verifier.head.block_hash must match observed_head.hash"
run_invalid_external_case "verifier_state_root_mismatch" "external_verifier_light_client_lite_ready external_verifier.head.state_root must match observed_head.state_root"
run_invalid_external_case "verifier_proof_hash_mismatch" "external_verifier_light_client_lite_ready external_verifier.proof_hash must match evidence proof_hash"
run_invalid_external_case "missing_denials" "external_verifier_light_client_lite_ready does_not_claim missing"
