# Public Testnet Runtime And World Resource Closure Evidence (2026-07-05)

证据窗口边界：本文的 `current`、`live`、`fresh`、lane verdict 和后续建议仅指文中记录的采集窗口，不证明当前端点可用、节点健康、发布 readiness 或持续任务义务。当前部署/恢复以[governed bootstrap runbook](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md)为准，当前 readiness 以[formal network runbook](../../p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md)及新采集的同窗证据裁决；保留本次失败、通过与未验证范围，不以较晚结果覆盖。

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
