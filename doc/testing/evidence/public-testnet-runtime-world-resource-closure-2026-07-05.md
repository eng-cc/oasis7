# Public Testnet Runtime And World Resource Closure Evidence (2026-07-05)

## Meta
- Task: GitHub issue #1964 / `task_643302fe9bf7404fb9bd9dcf7f8985ed`
- Lanes: `runtime_bootstrap`, `world_resource_provenance_ready`
- Owner roles: `runtime_engineer`, `blockchain_ops_engineer`
- Integration role: `tpm`
- Verdict: `pass`

This packet gives the current local evidence path for runtime bootstrap and
world-resource provenance closure. The detailed deployment and clean validator
rebuild evidence was recorded in GitHub issue #1964 comment
`4886618726`; this file keeps the lane TSV compatible with
`scripts/network-tier-public-testnet-readiness.sh`, which requires local
evidence paths.

## Current Live Samples

Same-window evidence from the current run is also captured in:

- `doc/testing/evidence/public-testnet-resource-delta-replay-2026-07-05.md`
- `doc/testing/evidence/public-testnet-api-viewer-projection-2026-07-05.json`

Observed current public-testnet state:

| Node | Role | Height | World id | Chain id | Runtime last error | Resource readiness | Resource failed gates |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | sequencer | `111` | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `null` | `ready` | `[]` |
| `triad-testnet-storage` | storage | `111` | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `null` | `ready` | `[]` |

The status samples show:

- both validators are running against the governed `public_testnet` world and
  chain identity
- committed height and world-resource commit height are advancing beyond
  genesis
- `world_resource.schema_version=oasis7.world_resource_manifest.v1`
- `world_resource.delta_schema_version=oasis7.world_resource_delta.v1`
- `world_resource.committed_chunk_count=2`
- `world_resource.provisional_chunk_count=0`
- `world_resource.pending_delta_count=0`

## Boundary

Allowed:
- `runtime_bootstrap=pass`
- `world_resource_provenance_ready=pass`
- current governed `public_testnet` runtime and world-resource status are
  healthy enough for the formal rehearsal lane closure

Still denied:
- `ready_for_live_candidate`
- mainnet-grade durability
- production OC settlement
- hosted same-world entry readiness
