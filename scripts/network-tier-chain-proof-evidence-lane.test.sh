#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$ROOT_DIR"

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

valid_evidence="$TMPDIR/chain-proof-evidence-valid.json"
valid_external_verifier_evidence="$TMPDIR/external-verifier-light-client-lite-valid.json"
valid_state_receipt_evidence="$TMPDIR/state-resource-receipt-proof-valid.json"
valid_state_receipt_absence_evidence="$TMPDIR/state-resource-receipt-proof-absence-valid.json"
valid_finality_evidence="$TMPDIR/validator-finality-proof-valid.json"
valid_finality_transition_evidence="$TMPDIR/validator-finality-proof-transition-valid.json"
lanes="$TMPDIR/lanes.tsv"
absence_lanes="$TMPDIR/lanes-state-receipt-absence.tsv"
transition_lanes="$TMPDIR/lanes-validator-finality-transition.tsv"
out_dir="$TMPDIR/readiness"
absence_out_dir="$TMPDIR/readiness-state-receipt-absence"
transition_out_dir="$TMPDIR/readiness-validator-finality-transition"
summary="$TMPDIR/summary.json"
absence_summary="$TMPDIR/summary-state-receipt-absence.json"
transition_summary="$TMPDIR/summary-validator-finality-transition.json"
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
state_resource_receipt_proof_ready	blockchain_ops_engineer	pass	$valid_state_receipt_evidence	optional state/resource/receipt proof lane must not promote public_testnet readiness by itself
validator_finality_proof_ready	blockchain_ops_engineer	pass	$valid_finality_evidence	optional validator/finality proof lane must not promote public_testnet readiness by itself
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

cat >"$valid_state_receipt_evidence" <<'JSON'
{
  "evidence_schema": "oasis7.state_resource_receipt_proof_evidence.v1",
  "status": "pass",
  "proof_contract": "WorldStateReceiptProofV1",
  "claim_boundary": "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness",
  "observed_at_unix_ms": 1772467200000,
  "independent_process": true,
  "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
  "command_ref": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --state-receipt-proof proof.json --proof-ref state-receipt-proof-ref-42 --expect-hash state-receipt-proof-hash-42 --json",
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
  "observed_head": {
    "height": 42,
    "hash": "block-hash-42",
    "state_root": "state-root-42",
    "receipts_root": "receipts-root-42"
  },
  "state_receipt_proof_ref": "state-receipt-proof-ref-42",
  "state_receipt_proof_hash": "state-receipt-proof-hash-42",
  "proof_targets": {
    "state_or_query": {
      "proof_kind": "resource_state",
      "namespace": "inventory",
      "resource_id": "agent-1/bag",
      "query_id": "",
      "root_hash": "state-root-42",
      "leaf_hash": "leaf-hash-42",
      "proof_status": "included"
    },
    "resource": {
      "resource_manifest_ref": "resource-manifest-ref-42",
      "resource_delta_ref": "resource-delta-ref-42",
      "content_hash": "resource-content-hash-42",
      "commit_height": 42,
      "commit_hash": "block-hash-42"
    }
  },
  "external_verifier": {
    "schema_version": "oasis7.world_state_receipt_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldStateReceiptProofV1",
    "hash_domain": "oasis7.world_state_receipt_proof.v1",
    "claim_boundary": "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness",
    "proof_ref": "state-receipt-proof-ref-42",
    "proof_hash": "state-receipt-proof-hash-42",
    "head_proof_hash": "head-proof-hash-42",
    "world_id": "world-a",
    "height": 42,
    "proof_kind": "resource_state",
    "proof_status": "included",
    "subject": {
      "subject_kind": "resource_state",
      "namespace": "inventory",
      "resource_id": "agent-1/bag",
      "value_hash": "resource-value-hash-42",
      "value_codec": "cbor"
    },
    "root_hash": "state-root-42",
    "leaf_hash": "leaf-hash-42",
    "proof_path_nodes": 2,
    "head": {
      "block_hash": "block-hash-42",
      "state_root": "state-root-42",
      "receipts_root": "receipts-root-42"
    },
    "does_not_claim": [
      "mainnet-grade finality",
      "full light client",
      "validator-set finality",
      "DA sampling",
      "multi-client consensus equivalence",
      "live runtime arbitrary state proof availability"
    ]
  },
  "node_db_access_used": false,
  "manual_checkpoint_or_data_copy_used": false,
  "privileged_internal_api_used": false,
  "does_not_claim": [
    "ready_for_live_candidate",
    "mainnet-grade",
    "full light client security",
    "validator-set finality",
    "multi-client consensus equivalence",
    "production OC settlement",
    "live runtime arbitrary state proof availability"
  ],
  "residual_risk": [
    "sampled state/resource/receipt proof evidence does not prove full light-client security"
  ]
}
JSON

cat >"$valid_finality_evidence" <<'JSON'
{
  "evidence_schema": "oasis7.validator_finality_proof.v1",
  "status": "pass",
  "verifier_mode": "validator_set_finality",
  "independent_process": true,
  "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
  "command_ref": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --finality-proof proof.json --format json --expect-world-id world-a --expect-height 42 --json",
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
  "finality_proof_ref": "finality-proof-ref-42",
  "finality_proof_hash": "finality-proof-hash-42",
  "sample_window": {
    "started_at": "2026-07-03T00:00:00Z",
    "ended_at": "2026-07-03T00:01:00Z"
  },
  "trusted_anchor": {
    "height": 39,
    "block_hash": "anchor-block-39"
  },
  "observed_head": {
    "height": 42,
    "hash": "block-hash-42",
    "state_root": "state-root-42"
  },
  "verified_range": {
    "from_height": 40,
    "to_height": 42
  },
  "validator_set": {
    "validator_set_id": "sample-set-1",
    "activation_height": 1,
    "validator_set_hash": "validator-set-hash-42",
    "validator_count": 3,
    "quorum_threshold_bps": 6667
  },
  "finality_sample": {
    "commitment_count": 3,
    "vote_count": 6,
    "misbehavior_evidence_count": 1,
    "stake_threshold_checked": true,
    "validator_set_hash_checked": true,
    "consensus_approver_subset_checked": true,
    "ed25519_signature_verification_checked": true,
    "validator_set_transition_execution_checked": true,
    "validator_set_transition_count": 0
  },
  "transition_sample": {
    "transition_result": "no_transition_in_sample",
    "transition_count": 0,
    "reason": "bounded transition semantics are verified when present; not trust-minimized validator governance"
  },
  "fork_or_reorg_cases": [
    "conflicting_head_rejected_or_recorded",
    "unknown_signer_rejected",
    "insufficient_signed_stake_rejected"
  ],
  "misbehavior_result": "evidence_recorded",
  "external_verifier": {
    "schema_version": "oasis7.world_finality_proof_verifier.v1",
    "status": "pass",
    "verifier_mode": "validator_set_finality",
    "proof_contract": "WorldFinalityProofV1",
    "hash_domain": "oasis7.world_finality_proof.v1",
    "claim_boundary": "validator_set_finality_evidence_only_not_full_light_client_or_mainnet_readiness",
    "proof_ref": "finality-proof-ref-42",
    "proof_hash": "finality-proof-hash-42",
    "world_id": "world-a",
    "from_height": 40,
    "to_height": 42,
    "trusted_anchor": {
      "height": 39,
      "block_hash": "anchor-block-39"
    },
    "validator_set": {
      "validator_set_id": "sample-set-1",
      "activation_height": 1,
      "validator_set_hash": "validator-set-hash-42",
      "validator_count": 3,
      "quorum_threshold_bps": 6667
    },
    "finality": {
      "commitment_count": 3,
      "vote_count": 6,
      "misbehavior_evidence_count": 1,
      "stake_threshold_checked": true,
      "validator_set_hash_checked": true,
      "consensus_approver_subset_checked": true,
      "ed25519_signature_verification_checked": true,
      "validator_set_transition_execution_checked": true,
      "validator_set_transition_count": 0
    },
    "head": {
      "height": 42,
      "block_hash": "block-hash-42",
      "state_root": "state-root-42"
    },
    "does_not_claim": [
      "mainnet-grade finality",
      "full light client",
      "trust-minimized validator-set transition governance",
      "DA sampling",
      "multi-client consensus equivalence",
      "public validator onboarding open"
    ]
  },
  "node_db_access_used": false,
  "manual_checkpoint_or_data_copy_used": false,
  "privileged_internal_api_used": false,
  "does_not_claim": [
    "full light client security",
    "mainnet-grade finality",
    "trust-minimized validator transition",
    "public validator onboarding open",
    "permissionless validator onboarding",
    "DA sampling",
    "multi-client consensus equivalence",
    "ready_for_live_candidate",
    "live public launch"
  ],
  "residual_risk": [
    "same implementation family verifier",
    "signature evidence hash is not live signature verification"
  ]
}
JSON

python3 - "$valid_finality_evidence" "$valid_finality_transition_evidence" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
data["finality_sample"]["validator_set_transition_count"] = 1
data["transition_sample"]["transition_result"] = "bounded_transition_execution_checked"
data["transition_sample"]["transition_count"] = 1
data["external_verifier"]["finality"]["validator_set_transition_count"] = 1
pathlib.Path(sys.argv[2]).write_text(json.dumps(data, indent=2), encoding="utf-8")
PY

python3 - "$valid_state_receipt_evidence" "$valid_state_receipt_absence_evidence" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
data["proof_targets"]["state_or_query"]["proof_status"] = "absent"
data["proof_targets"]["state_or_query"]["resource_id"] = "agent-1/missing-slot"
data["proof_targets"].pop("resource", None)
data["external_verifier"]["proof_status"] = "absent"
data["external_verifier"]["subject"] = {
    "subject_kind": "resource_state",
    "namespace": "inventory",
    "resource_id": "agent-1/missing-slot",
    "value_hash": "",
    "value_codec": "",
    "absence_marker_hash": "resource-absence-marker-42"
}
pathlib.Path(sys.argv[2]).write_text(json.dumps(data, indent=2), encoding="utf-8")
PY

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$lanes" \
  --out-dir "$out_dir" >"$summary"

cat >"$absence_lanes" <<EOF
state_resource_receipt_proof_ready	blockchain_ops_engineer	pass	$valid_state_receipt_absence_evidence	optional absence proof lane must not require missing resource content
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$absence_lanes" \
  --out-dir "$absence_out_dir" >"$absence_summary"

cat >"$transition_lanes" <<EOF
validator_finality_proof_ready	blockchain_ops_engineer	pass	$valid_finality_transition_evidence	optional validator/finality transition proof lane must not promote public_testnet readiness by itself
EOF

./scripts/network-tier-public-testnet-readiness.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --lanes-tsv "$transition_lanes" \
  --out-dir "$transition_out_dir" >"$transition_summary"

python3 - "$summary" "$absence_summary" "$transition_summary" "$valid_evidence" "$valid_state_receipt_absence_evidence" "$valid_finality_transition_evidence" <<'PY'
import json
import pathlib
import sys

summary = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
absence_summary = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
transition_summary = json.loads(pathlib.Path(sys.argv[3]).read_text(encoding="utf-8"))
evidence = str(pathlib.Path(sys.argv[4]).resolve())
absence_evidence = str(pathlib.Path(sys.argv[5]).resolve())
transition_evidence = str(pathlib.Path(sys.argv[6]).resolve())
ignored = summary.get("ignored_lanes", [])
if summary.get("readiness_verdict") == "ready_for_live_candidate":
    raise SystemExit("optional chain proof lane must not promote readiness")
if summary.get("live_candidate_allowed") is not False:
    raise SystemExit("optional chain proof lane must keep live_candidate_allowed=false")
for optional_lane in (
    "chain_proof_evidence_ready",
    "external_verifier_light_client_lite_ready",
    "state_resource_receipt_proof_ready",
    "validator_finality_proof_ready",
):
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
state_receipt_matches = [
    lane for lane in ignored if lane.get("lane_id") == "state_resource_receipt_proof_ready"
]
if len(state_receipt_matches) != 1:
    raise SystemExit(f"expected one ignored state/resource/receipt proof lane, got {ignored}")
finality_matches = [
    lane for lane in ignored if lane.get("lane_id") == "validator_finality_proof_ready"
]
if len(finality_matches) != 1:
    raise SystemExit(f"expected one ignored validator/finality proof lane, got {ignored}")
absence_ignored = absence_summary.get("ignored_lanes", [])
absence_matches = [
    lane for lane in absence_ignored if lane.get("lane_id") == "state_resource_receipt_proof_ready"
]
if len(absence_matches) != 1:
    raise SystemExit(f"expected one ignored absence state/resource proof lane, got {absence_ignored}")
if absence_matches[0].get("resolved_evidence_path") != absence_evidence:
    raise SystemExit(
        f"expected resolved absence evidence path {absence_evidence}, got {absence_matches[0]}"
    )
if absence_summary.get("readiness_verdict") == "ready_for_live_candidate":
    raise SystemExit("optional absence proof lane must not promote readiness")
transition_ignored = transition_summary.get("ignored_lanes", [])
transition_matches = [
    lane for lane in transition_ignored if lane.get("lane_id") == "validator_finality_proof_ready"
]
if len(transition_matches) != 1:
    raise SystemExit(
        f"expected one ignored validator/finality transition lane, got {transition_ignored}"
    )
if transition_matches[0].get("resolved_evidence_path") != transition_evidence:
    raise SystemExit(
        f"expected resolved transition evidence path {transition_evidence}, got {transition_matches[0]}"
    )
if transition_summary.get("readiness_verdict") == "ready_for_live_candidate":
    raise SystemExit("optional validator/finality transition lane must not promote readiness")
PY

run_invalid_state_receipt_case() {
  local case_name=$1
  local expected_error=$2
  local invalid_evidence="$TMPDIR/state-resource-receipt-proof-invalid-$case_name.json"
  python3 - "$valid_state_receipt_evidence" "$invalid_evidence" "$case_name" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
case_name = sys.argv[3]

if case_name == "missing_proof_ref":
    data["state_receipt_proof_ref"] = ""
elif case_name == "verifier_hash_mismatch":
    data["external_verifier"]["proof_hash"] = "wrong-proof-hash"
elif case_name == "hash_domain_mismatch":
    data["external_verifier"]["hash_domain"] = "wrong-domain"
elif case_name == "state_root_mismatch":
    data["external_verifier"]["root_hash"] = "wrong-state-root"
elif case_name == "unsupported_proof_kind":
    data["external_verifier"]["proof_kind"] = "bogus"
elif case_name == "head_block_hash_mismatch":
    data["external_verifier"]["head"]["block_hash"] = "wrong-block-hash"
elif case_name == "empty_proof_targets":
    data["proof_targets"]["state_or_query"] = {}
    data["proof_targets"]["resource"] = {}
elif case_name == "resource_commit_mismatch":
    data["proof_targets"]["resource"]["commit_hash"] = "wrong-commit-hash"
elif case_name == "state_target_subject_mismatch":
    data["proof_targets"]["state_or_query"]["resource_id"] = "agent-2/bag"
elif case_name == "state_target_status_mismatch":
    data["external_verifier"]["proof_status"] = "absent"
elif case_name == "receipt_action_mismatch":
    data["external_verifier"]["proof_kind"] = "receipt"
    data["external_verifier"]["root_hash"] = "receipts-root-42"
    data["external_verifier"]["leaf_hash"] = "receipt-leaf-hash-42"
    data["external_verifier"]["subject"] = {
        "subject_kind": "receipt",
        "action_id": "wrong-action-42",
        "receipt_hash": "receipt-hash-42",
        "status": "committed",
        "result_hash": "receipt-result-hash-42"
    }
    data["proof_targets"] = {
        "receipt": {
            "action_id": "action-42",
            "receipt_hash": "receipt-hash-42",
            "execution_status": "committed",
            "result_hash": "receipt-result-hash-42",
            "root_hash": "receipts-root-42",
            "leaf_hash": "receipt-leaf-hash-42"
        }
    }
elif case_name == "db_access":
    data["node_db_access_used"] = True
elif case_name == "missing_denials":
    data["does_not_claim"] = ["ready_for_live_candidate"]
else:
    raise SystemExit(f"unknown invalid state receipt case: {case_name}")

pathlib.Path(sys.argv[2]).write_text(json.dumps(data, indent=2), encoding="utf-8")
PY
  local invalid_lanes="$TMPDIR/lanes-state-receipt-invalid-$case_name.tsv"
  cat >"$invalid_lanes" <<EOF
state_resource_receipt_proof_ready	blockchain_ops_engineer	pass	$invalid_evidence	invalid state/resource/receipt proof lane must fail closed
EOF
  if ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json \
    --lanes-tsv "$invalid_lanes" \
    --out-dir "$TMPDIR/readiness-invalid-state-receipt-$case_name" >"$TMPDIR/invalid-state-receipt-$case_name.json" 2>"$stderr"; then
    echo "expected invalid state/resource/receipt proof case to fail: $case_name" >&2
    exit 1
  fi
  if ! grep -Fq "$expected_error" "$stderr"; then
    echo "expected error '$expected_error' for state/resource/receipt case '$case_name'" >&2
    cat "$stderr" >&2
    exit 1
  fi
}

run_invalid_state_receipt_case "missing_proof_ref" "state_resource_receipt_proof_ready state_receipt_proof_ref missing"
run_invalid_state_receipt_case "verifier_hash_mismatch" "state_resource_receipt_proof_ready external_verifier.proof_hash must match evidence state_receipt_proof_hash"
run_invalid_state_receipt_case "hash_domain_mismatch" "state_resource_receipt_proof_ready external_verifier.hash_domain mismatch"
run_invalid_state_receipt_case "state_root_mismatch" "state_resource_receipt_proof_ready external_verifier.root_hash must match observed_head.state_root"
run_invalid_state_receipt_case "unsupported_proof_kind" "state_resource_receipt_proof_ready external_verifier.proof_kind unsupported"
run_invalid_state_receipt_case "head_block_hash_mismatch" "state_resource_receipt_proof_ready external_verifier.head.block_hash must match observed_head.hash"
run_invalid_state_receipt_case "empty_proof_targets" "state_resource_receipt_proof_ready proof_targets.state_or_query.proof_kind missing"
run_invalid_state_receipt_case "resource_commit_mismatch" "state_resource_receipt_proof_ready proof_targets.resource.commit_hash must match observed_head.hash"
run_invalid_state_receipt_case "state_target_subject_mismatch" "state_resource_receipt_proof_ready proof_targets.state_or_query.resource_id must match external_verifier.subject.resource_id"
run_invalid_state_receipt_case "state_target_status_mismatch" "state_resource_receipt_proof_ready proof_targets.state_or_query.proof_status must match external_verifier.proof_status"
run_invalid_state_receipt_case "receipt_action_mismatch" "state_resource_receipt_proof_ready proof_targets.receipt.action_id must match external_verifier.subject.action_id"
run_invalid_state_receipt_case "db_access" "state_resource_receipt_proof_ready node_db_access_used must be false"
run_invalid_state_receipt_case "missing_denials" "state_resource_receipt_proof_ready does_not_claim missing"

run_invalid_finality_case() {
  local case_name=$1
  local expected_error=$2
  local invalid_evidence="$TMPDIR/validator-finality-proof-invalid-$case_name.json"
  python3 - "$valid_finality_evidence" "$invalid_evidence" "$case_name" <<'PY'
import json
import pathlib
import sys

data = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
case_name = sys.argv[3]

if case_name == "network_mismatch":
    data["network_tier"]["network_id"] = "other-network"
elif case_name == "verifier_hash_mismatch":
    data["external_verifier"]["proof_hash"] = "wrong-finality-proof-hash"
elif case_name == "head_hash_mismatch":
    data["external_verifier"]["head"]["block_hash"] = "wrong-block-hash"
elif case_name == "validator_set_hash_mismatch":
    data["external_verifier"]["validator_set"]["validator_set_hash"] = "wrong-validator-set-hash"
elif case_name == "threshold_not_checked":
    data["finality_sample"]["stake_threshold_checked"] = False
elif case_name == "signature_not_checked":
    data["finality_sample"]["ed25519_signature_verification_checked"] = False
elif case_name == "transition_not_checked":
    data["finality_sample"]["validator_set_transition_execution_checked"] = False
elif case_name == "external_signature_mismatch":
    data["external_verifier"]["finality"]["ed25519_signature_verification_checked"] = False
elif case_name == "external_transition_count_mismatch":
    data["external_verifier"]["finality"]["validator_set_transition_count"] = 1
elif case_name == "external_missing_denials":
    data["external_verifier"]["does_not_claim"] = ["full light client"]
elif case_name == "db_access":
    data["node_db_access_used"] = True
elif case_name == "missing_denials":
    data["does_not_claim"] = ["ready_for_live_candidate"]
else:
    raise SystemExit(f"unknown invalid finality case: {case_name}")

pathlib.Path(sys.argv[2]).write_text(json.dumps(data, indent=2), encoding="utf-8")
PY
  local invalid_lanes="$TMPDIR/lanes-finality-invalid-$case_name.tsv"
  cat >"$invalid_lanes" <<EOF
validator_finality_proof_ready	blockchain_ops_engineer	pass	$invalid_evidence	invalid validator/finality proof lane must fail closed
EOF
  if ./scripts/network-tier-public-testnet-readiness.sh \
    --manifest doc/testing/templates/network-tier-public-testnet.example.json \
    --lanes-tsv "$invalid_lanes" \
    --out-dir "$TMPDIR/readiness-invalid-finality-$case_name" >"$TMPDIR/invalid-finality-$case_name.json" 2>"$stderr"; then
    echo "expected invalid validator/finality proof case to fail: $case_name" >&2
    exit 1
  fi
  if ! grep -Fq "$expected_error" "$stderr"; then
    echo "expected error '$expected_error' for validator/finality case '$case_name'" >&2
    cat "$stderr" >&2
    exit 1
  fi
}

run_invalid_finality_case "network_mismatch" "validator_finality_proof_ready network_tier.network_id must match manifest"
run_invalid_finality_case "verifier_hash_mismatch" "validator_finality_proof_ready external_verifier.proof_hash must match evidence finality_proof_hash"
run_invalid_finality_case "head_hash_mismatch" "validator_finality_proof_ready external_verifier.head.block_hash must match observed_head.hash"
run_invalid_finality_case "validator_set_hash_mismatch" "validator_finality_proof_ready external_verifier.validator_set_hash must match evidence"
run_invalid_finality_case "threshold_not_checked" "validator_finality_proof_ready finality_sample.stake_threshold_checked must be true"
run_invalid_finality_case "signature_not_checked" "validator_finality_proof_ready finality_sample.ed25519_signature_verification_checked must be true"
run_invalid_finality_case "transition_not_checked" "validator_finality_proof_ready finality_sample.validator_set_transition_execution_checked must be true"
run_invalid_finality_case "external_signature_mismatch" "validator_finality_proof_ready external_verifier.finality.ed25519_signature_verification_checked must match finality_sample"
run_invalid_finality_case "external_transition_count_mismatch" "validator_finality_proof_ready external_verifier.finality.validator_set_transition_count must match finality_sample"
run_invalid_finality_case "external_missing_denials" "validator_finality_proof_ready external_verifier.does_not_claim missing"
run_invalid_finality_case "db_access" "validator_finality_proof_ready node_db_access_used must be false"
run_invalid_finality_case "missing_denials" "validator_finality_proof_ready does_not_claim missing"

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

if ./scripts/network-tier-external-verifier-light-client-lite.sh \
  --manifest doc/testing/templates/network-tier-public-testnet.example.json \
  --proof "$generated_proof" \
  --proof-format json \
  --proof-ref proof-ref-42 \
  --world-id world-a \
  --expect-height 42 \
  --observed-head-hash "$generated_head_hash" \
  --observed-state-root "$generated_state_root" \
  --from-height 100 \
  --sample-started-at 2026-07-03T00:00:00Z \
  --sample-ended-at 2026-07-03T00:01:00Z \
  --out "$TMPDIR/should-not-write-inverted-range.json" >"$TMPDIR/inverted-range.stdout" 2>"$stderr"; then
  echo "expected wrapper to reject inverted verified range" >&2
  exit 1
fi
grep -q "verified range is inverted" "$stderr"

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
elif case_name == "missing_proof_ref":
    data["proof_ref"] = ""
    data["external_verifier"]["proof_ref"] = ""
elif case_name == "missing_proof_hash":
    data["proof_hash"] = ""
    data["external_verifier"]["proof_hash"] = ""
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
run_invalid_external_case "missing_proof_ref" "external_verifier_light_client_lite_ready proof_ref missing"
run_invalid_external_case "missing_proof_hash" "external_verifier_light_client_lite_ready proof_hash missing"
run_invalid_external_case "missing_verifier" "external_verifier_light_client_lite_ready external_verifier object missing"
run_invalid_external_case "verifier_head_hash_mismatch" "external_verifier_light_client_lite_ready external_verifier.head.block_hash must match observed_head.hash"
run_invalid_external_case "verifier_state_root_mismatch" "external_verifier_light_client_lite_ready external_verifier.head.state_root must match observed_head.state_root"
run_invalid_external_case "verifier_proof_hash_mismatch" "external_verifier_light_client_lite_ready external_verifier.proof_hash must match evidence proof_hash"
run_invalid_external_case "missing_denials" "external_verifier_light_client_lite_ready does_not_claim missing"
