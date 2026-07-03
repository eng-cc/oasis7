# chain proof evidence v1 template

Purpose: record one sampled proof closure that binds a committed world head to
execution output and checkpoint/blob closure evidence. This packet is an
evidence contract only; it is not a light-client proof, state proof, public
testnet readiness verdict, or mainnet-grade finality claim.

## Claim Boundary

| Field | Value |
| --- | --- |
| evidence_schema | `oasis7.chain_proof_evidence.v1` |
| proof_contract | `WorldHeadProofV1` |
| claim_boundary | `head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness` |
| allowed_verdicts | `proof_complete`, `proof_partial`, `proof_blocked` |
| does_not_claim | `module_full`, `integration_required`, `release_full`, `public_testnet ready`, `ready_for_live_candidate`, `mainnet-grade` |

## Required Fields

```json
{
  "evidence_schema": "oasis7.chain_proof_evidence.v1",
  "proof_contract": "WorldHeadProofV1",
  "observed_at_unix_ms": 0,
  "node_id": "",
  "network_tier": {
    "tier": "",
    "status": "",
    "chain_id": "",
    "network_id": ""
  },
  "proof_closure_status": "proof_blocked",
  "world_head_proof_v1": {
    "schema_version": 1,
    "world_id": "",
    "height": 0,
    "timestamp_ms": 0,
    "proof_hash": "",
    "world_head_proof_ref": "",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness"
  },
  "external_verifier": {
    "schema_version": "oasis7.world_head_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldHeadProofV1",
    "proof_ref": "",
    "proof_hash": "",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "verifier_command": "cargo run -p oasis7_proto --bin oasis7_world_head_proof_verify -- --proof <proof> --proof-ref <ref> --expect-hash <hash> --json",
    "verified_at_unix_ms": 0,
    "does_not_claim": [
      "mainnet-grade finality",
      "state proof",
      "receipt proof",
      "DA sampling",
      "full light client"
    ]
  },
  "readiness_linkage": {
    "readiness_status": "",
    "failed_gates": [],
    "network_height_lag": null,
    "state_sync_fallback_required": null
  },
  "does_not_claim": [
    "module_full",
    "integration_required",
    "release_full",
    "public_testnet ready",
    "ready_for_live_candidate",
    "mainnet-grade"
  ],
  "residual_risk": []
}
```

## Validation Notes

- `world_head_proof_v1.proof_hash` must be produced from canonical CBOR with
  the `oasis7.world_head_proof.v1` hash domain.
- `WorldHeadProofV1::validate_contract()` must pass before this packet may use
  `proof_complete`; attach the `oasis7_world_head_proof_verify --json` summary
  under `external_verifier`.
- `external_verifier.proof_hash` and `external_verifier.proof_ref` must match
  `world_head_proof_v1.proof_hash` and
  `world_head_proof_v1.world_head_proof_ref`.
- If checkpoint evidence is present, it must pin both the snapshot manifest ref
  and journal segments ref.
- A passing packet only proves sampled evidence consistency. Public network
  promotion remains controlled by `network-tier-public-testnet-readiness.sh`
  and same-window live evidence.
