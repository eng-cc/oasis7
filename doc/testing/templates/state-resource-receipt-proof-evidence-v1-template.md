# state/resource/receipt proof evidence v1 template

Purpose: record one sampled operator-verifiable state/resource/receipt proof
closure anchored to a verified `WorldHeadProofV1`. This packet proves bounded
inclusion or absence evidence against a committed state or receipt root. It is
not a public-testnet promotion gate, full light client, validator-set finality
proof, production settlement proof, or live arbitrary runtime proof availability
claim.

## Claim Boundary

| Field | Value |
| --- | --- |
| evidence_schema | `oasis7.state_resource_receipt_proof_evidence.v1` |
| proof_contract | `WorldStateReceiptProofV1` |
| claim_boundary | `state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness` |
| readiness_lane | `state_resource_receipt_proof_ready` |
| readiness_lane_scope | optional / non-promotional / ignored by formal public_testnet required lanes |

## Required Fields

```json
{
  "evidence_schema": "oasis7.state_resource_receipt_proof_evidence.v1",
  "status": "pass",
  "proof_contract": "WorldStateReceiptProofV1",
  "claim_boundary": "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness",
  "observed_at_unix_ms": 0,
  "independent_process": true,
  "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
  "command_ref": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --state-receipt-proof <proof> --proof-ref <ref> --expect-hash <hash> --json",
  "network_tier": {
    "tier": "public_testnet",
    "status": "",
    "chain_id": "",
    "network_id": "",
    "world_id": ""
  },
  "manifest_ref": "",
  "genesis_ref": "",
  "bootstrap_peer_ref": "",
  "rpc_ref": "",
  "status_endpoint_ref": "",
  "observed_head": {
    "height": 0,
    "hash": "",
    "state_root": "",
    "receipts_root": ""
  },
  "state_receipt_proof_ref": "",
  "state_receipt_proof_hash": "",
  "proof_targets": {
    "state_or_query": {
      "proof_kind": "resource_state|query_result",
      "namespace": "",
      "resource_id": "",
      "query_id": "",
      "root_hash": "",
      "leaf_hash": "",
      "proof_status": "included|absent"
    },
    "resource": {
      "resource_manifest_ref": "",
      "resource_delta_ref": "",
      "content_hash": "",
      "commit_height": 0,
      "commit_hash": ""
    }
  },
  "external_verifier": {
    "schema_version": "oasis7.world_state_receipt_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldStateReceiptProofV1",
    "hash_domain": "oasis7.world_state_receipt_proof.v1",
    "claim_boundary": "state_resource_receipt_inclusion_evidence_only_not_full_light_client_or_mainnet_readiness",
    "proof_ref": "",
    "proof_hash": "",
    "head_proof_hash": "",
    "world_id": "",
    "height": 0,
    "proof_kind": "resource_state|query_result|receipt",
    "proof_status": "included|absent",
    "subject": {
      "subject_kind": "resource_state|query_result|receipt",
      "namespace": "",
      "resource_id": "",
      "query_id": "",
      "query_hash": "",
      "action_id": "",
      "receipt_hash": "",
      "status": "",
      "result_hash": ""
    },
    "root_hash": "",
    "leaf_hash": "",
    "proof_path_nodes": 0,
    "head": {
      "block_hash": "",
      "state_root": "",
      "receipts_root": ""
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
    "sampled inclusion evidence is not full state coverage or live arbitrary proof availability"
  ]
}
```

For a receipt proof, replace `proof_targets` with:

```json
{
  "proof_targets": {
    "receipt": {
      "action_id": "",
      "receipt_hash": "",
      "execution_status": "",
      "result_hash": "",
      "root_hash": "",
      "leaf_hash": ""
    }
  }
}
```

## Validation Notes

- `WorldStateReceiptProofV1::validate_contract()` must pass before this packet
  may use `status=pass`.
- `state_receipt_proof_hash` must be produced from canonical CBOR with the
  `oasis7.world_state_receipt_proof.v1` hash domain.
- `external_verifier.proof_hash` and `external_verifier.proof_ref` must match
  `state_receipt_proof_hash` and `state_receipt_proof_ref`.
- For `resource_state` and `query_result`, `external_verifier.root_hash` must
  match `observed_head.state_root`.
- For `receipt`, `external_verifier.root_hash` must match
  `observed_head.receipts_root`.
- Evidence packets are single-subject: use `proof_targets.state_or_query`
  for `resource_state` / `query_result` proofs, and use
  `receipt_proof_targets.receipt` for receipt examples. Do not imply one
  verifier summary proves unrelated state/query and receipt subjects at once.
- `node_db_access_used`, `manual_checkpoint_or_data_copy_used`, and
  `privileged_internal_api_used` must be `false` for external/operator
  evidence.
- A passing packet is optional evidence only; it must remain outside the 11
  formal `public_testnet` required lanes.
