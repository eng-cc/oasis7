# external verifier light-client-lite v1 template

Purpose: record one operator-run external verifier sample for `WorldHeadProofV1`.
This packet proves that a separate process loaded a proof artifact, ran the
repo-owned verifier, and checked a bounded height window. It is an audit lane
only; it is not full light-client security, multi-client consensus equivalence,
public-testnet live-candidate readiness, or mainnet-grade finality.

## Claim Boundary

| Field | Value |
| --- | --- |
| evidence_schema | `oasis7.external_verifier_light_client_lite.v1` |
| verifier_mode | `external_light_client_lite` |
| verification_result | `accepted` |
| readiness behavior | optional / non-promotional / reported in `ignored_lanes` |
| does_not_claim | `mainnet-grade`, `production OC settlement`, `public validator onboarding open`, `multi-client consensus equivalence`, `full light client security`, `ready_for_live_candidate` |

## Required Fields

```json
{
  "evidence_schema": "oasis7.external_verifier_light_client_lite.v1",
  "status": "pass",
  "verifier_mode": "external_light_client_lite",
  "independent_process": true,
  "implementation_ref": "crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs",
  "command_ref": "./scripts/network-tier-external-verifier-light-client-lite.sh --manifest <manifest> --proof <proof> --proof-ref <ref> --world-id <world> --expect-height <height> --observed-head-hash <hash> --observed-state-root <root> --from-height <height> --sample-started-at <time> --sample-ended-at <time> --out <evidence>",
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
  "sample_window": {
    "started_at": "",
    "ended_at": ""
  },
  "observed_head": {
    "height": 0,
    "hash": "",
    "state_root": ""
  },
  "verified_range": {
    "from_height": 0,
    "to_height": 0
  },
  "verification_result": "accepted",
  "proof_ref": "",
  "proof_hash": "",
  "external_verifier": {
    "schema_version": "oasis7.world_head_proof_verifier.v1",
    "status": "pass",
    "proof_contract": "WorldHeadProofV1",
    "hash_domain": "oasis7.world_head_proof.v1",
    "claim_boundary": "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness",
    "proof_hash": "",
    "world_id": "",
    "height": 0,
    "checkpoint_bound": true,
    "does_not_claim": [
      "mainnet-grade finality",
      "state proof",
      "receipt proof",
      "DA sampling",
      "full light client"
    ]
  },
  "verified_at_unix_ms": 0,
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
  "residual_risk": []
}
```

## Validation Notes

- Run the helper from outside the node process and preserve the command output
  as the `external_verifier` object.
- `manifest_ref`, `genesis_ref`, and `bootstrap_peer_ref` must match the active
  network-tier manifest.
- `rpc_ref` should match `endpoint_policy.rpc_ref`; use `status_endpoint_ref`
  when the exact status URL is a sub-path of the same public RPC service.
- `node_db_access_used`, `manual_checkpoint_or_data_copy_used`, and
  `privileged_internal_api_used` must remain `false`.
- This lane is intentionally optional until runtime and QA promote stronger
  verifier semantics into a formal required release gate.
