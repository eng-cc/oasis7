# Public Testnet Resource Delta Replay Evidence (2026-07-05)

## Meta
- Task: GitHub issue #1964 / `task_643302fe9bf7404fb9bd9dcf7f8985ed`
- Lane: `resource_delta_replay_ready`
- Owner role: `runtime_engineer`
- Integration role: `tpm`
- Verdict: `pass`

This packet verifies the current governed `public_testnet` committed resource
delta replay boundary by cross-checking each live node's
`/v1/chain/status.world_resource` fields against the same node's
execution-world `snapshot.json` resource manifest and latest resource delta.

It does not claim `ready_for_live_candidate`, mainnet-grade durability, or
production OC settlement.

## Verification Command

```bash
python3 scripts/public-testnet-resource-delta-replay-check.py \
  --expected-world-id oasis7-public-testnet-governed-20260606 \
  --expected-chain-id oasis7-public-testnet-governed-20260606 \
  --node sequencer \
    .tmp/runtime-bootstrap-1964/resource-delta-replay-20260705/sequencer-status-replay-ready.json \
    .tmp/runtime-bootstrap-1964/resource-delta-replay-20260705/sequencer-execution-world-resource-replay-ready.json \
  --node storage \
    .tmp/runtime-bootstrap-1964/resource-delta-replay-20260705/storage-status-replay-ready.json \
    .tmp/runtime-bootstrap-1964/resource-delta-replay-20260705/storage-execution-world-resource-replay-ready.json \
  --output .tmp/runtime-bootstrap-1964/resource-delta-replay-20260705/resource-delta-replay-report.json
```

Observed result:

```json
{
  "pass": true,
  "expected_world_id": "oasis7-public-testnet-governed-20260606",
  "expected_chain_id": "oasis7-public-testnet-governed-20260606"
}
```

## Live Node Samples

| Node | Height | Consensus hash | Manifest hash | Delta id | Delta replay status | Committed chunks | Provisional chunks | Pending delta | Failed gates |
| --- | ---: | --- | --- | --- | --- | ---: | ---: | ---: | --- |
| sequencer | `92` | `2a145662b0778ecfba6d540a33d5ee7dabe3eda4f198c1e66eac53d95e12ee87` | `697b8bce556db61c2f13bfa6c46854c84dda70834fe52a7a1d1f1d69e6eeb9a2` | `resource-delta-92-92` | `committed` | `2` | `0` | `0` | `[]` |
| storage | `92` | `2a145662b0778ecfba6d540a33d5ee7dabe3eda4f198c1e66eac53d95e12ee87` | `697b8bce556db61c2f13bfa6c46854c84dda70834fe52a7a1d1f1d69e6eeb9a2` | `resource-delta-92-92` | `committed` | `2` | `0` | `0` | `[]` |

Both nodes reported:

```json
{
  "world_id": "oasis7-public-testnet-governed-20260606",
  "chain_id": "oasis7-public-testnet-governed-20260606",
  "schema_version": "oasis7.world_resource_manifest.v1",
  "delta_schema_version": "oasis7.world_resource_delta.v1",
  "readiness_status": "ready",
  "failed_gates": []
}
```

## Replay Binding Checks

The generated report passed all checks for both sequencer and storage:

| Check family | Required result | Observed |
| --- | --- | --- |
| Live status readiness | `world_resource.readiness_status=ready`, `failed_gates=[]` | pass |
| Manifest identity | status, execution-world manifest, expected world/chain all match `oasis7-public-testnet-governed-20260606` | pass |
| Manifest hash replay target | `status.seed_manifest_hash == snapshot.chain_resource_manifest.manifest_hash` | pass |
| Delta replay target | `delta.resulting_manifest_hash == snapshot.chain_resource_manifest.manifest_hash` | pass |
| Delta schema/currentness | `delta.schema_version=oasis7.world_resource_delta.v1`, base/resulting hashes present | pass |
| Delta ordering | `delta.block_height == delta.ordering_key.height == consensus.committed_height == status.last_delta_commit_height` | pass |
| Delta status binding | `delta.delta_id == status.last_delta_id`, `delta.replay_status=committed`, `delta.commit_block_hash` present | pass |
| Provisional exclusion | `status.provisional_chunk_count=0`, no execution-world chunk has `provisional` or `chain_pending` status | pass |
| Pending exclusion | `status.pending_delta_count=0` | pass |
| Committed resource content | `committed_chunk_count > 0`; execution-world manifest has committed chunks | pass |

The latest delta has `entries=[]` at height `92`. This is acceptable for this
sample because the manifest already has committed chunks and the delta is a
committed no-op at the current height: `base_manifest_hash`,
`resulting_manifest_hash`, and the manifest hash all match. The runtime status
would fail `world_resource_delta_zero_effect` only when there is no committed
resource content.

## Code Contract

- `status_payload_world_resource.rs` loads the execution-world runtime snapshot
  first, falls back to simulator mirror only when needed, and reports
  `world_resource` readiness from the manifest and latest delta.
- It fails readiness when manifest identity/schema/hash, committed chunks,
  provisional chunks, pending delta, delta manifest hash, delta commit hash
  presence, or delta height are invalid.
- Runtime persistence writes `latest_chain_resource_delta` from the current
  resource manifest with `replay_status=committed`.
- The resource delta `commit_block_hash` is the execution resource commit
  reference generated by the execution bridge, while
  `world_resource.latest_resource_commit_hash` reports the node consensus block
  hash. The pass gate therefore requires the delta commit hash to be present,
  and binds replay through height, status delta id, and resulting manifest hash.

## Boundary

Allowed:
- `resource_delta_replay_ready=pass`
- current governed `public_testnet` nodes have committed resource delta replay
  evidence tied to live world/chain identity, height, status delta id, and
  manifest hash
- provisional and pending resource truth are excluded from the live resource
  status

Still denied:
- `ready_for_live_candidate`
- mainnet-grade settlement/durability
- production OC settlement
- hosted same-world entry readiness
- API/viewer same-window projection readiness
