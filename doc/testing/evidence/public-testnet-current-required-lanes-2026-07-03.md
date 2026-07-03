# Public Testnet Current Required Lanes (2026-07-03)

## Meta
- owner_role: `tpm` integration
- task: GitHub issue #1846 / `task_c345e90f2bf24cfd9ae60b4d1f2756dd`
- scope: current formal `public_testnet` 11 required-lane packet
- manifest: `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- lanes_tsv: `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- verdict: `block`
- live_candidate_allowed: `false`

## Purpose
This packet completes the current formal `public_testnet` required-lane table with all 11 active lanes used by `scripts/network-tier-public-testnet-readiness.sh`.

It does not promote `public_testnet` to `ready_for_live_candidate`. The packet is intentionally fail-closed because current evidence is still rehearsal/blocker evidence rather than fresh all-pass live evidence.

## Current Lane Matrix
| Lane | Status | Boundary |
| --- | --- | --- |
| `public_rpc_ready` | `partial` | Historical public RPC reachability exists, but no fresh July 2026 same-window pass after runtime drift. |
| `explorer_public_ready` | `partial` | Historical explorer reachability exists, but freshness/world-state alignment is not current pass evidence. |
| `faucet_guard_ready` | `partial` | Guard contract exists, but no fresh external claim after runtime drift/governed-bootstrap rehearsal status. |
| `reset_policy_announced` | `partial` | Resettable/non-mainnet testnet boundary exists at tier/older-network level, but current governed 20260606 network needs a fresh announcement ref. |
| `runtime_bootstrap` | `block` | Local observer sync path exists, but remaining live-head catch-up and release/runtime drift prevent pass. |
| `world_resource_provenance_ready` | `block` | Only cold-start placeholder/rehearsal world evidence exists; no current manifest/world/chain-bound pass. |
| `provider_resource_provenance_ready` | `block` | No current provider resource manifest/delta compatibility evidence exists for governed `public_testnet`. |
| `resource_delta_replay_ready` | `block` | No current committed resource-delta replay pass evidence exists. |
| `api_viewer_projection_ready` | `block` | No same-window API/viewer projection JSON pass evidence exists. |
| `same_world_hosted_entry_ready` | `block` | No `oasis7.same_world_hosted_entry.v1` JSON pass evidence exists. |
| `claims_boundary_review` | `pass` | QA claim boundary remains valid and does not widen readiness. |

## Blocker Summary
1. The governed-bootstrap manifest is still `status=rehearsal`.
2. Current public endpoint/faucet evidence is historical and constrained by the 2026-05-22 freshness/runtime drift audit.
3. The runtime lane remains blocked by live bootstrap/recovery drift rather than documentation absence.
4. World/provider/resource-delta lanes do not have current pass evidence tied to the governed `public_testnet` manifest/world/chain identity.
5. API/viewer and same-world hosted entry lanes do not have the required same-window JSON evidence.

## Claim Boundary
Allowed:
- `formal public_testnet mechanism is documented`
- `current governed bootstrap evidence is rehearsal / not ready_for_live_candidate`
- `current required-lane packet is complete but blocked`

Denied:
- `live public testnet is already online`
- `public faucet is open`
- `public validator admission is open`
- `ready_for_live_candidate`
- `mainnet-grade`
- `production OC settlement`

## Next Pass Requirements
To move this packet out of `block`, the next execution round must produce fresh evidence rather than reinterpret older artifacts:

1. Fresh same-window public RPC / explorer / guarded faucet samples.
2. Fresh governed-network reset-policy announcement ref for `oasis7-public-testnet-governed-20260606`.
3. Healthy governed runtime bootstrap from the current manifest/bundle/genesis/bootstrap peer set.
4. Manifest/world/chain-bound world resource provenance evidence.
5. Provider resource provenance and committed resource-delta replay evidence.
6. Same-window API/viewer projection JSON evidence.
7. `oasis7.same_world_hosted_entry.v1` JSON evidence for hosted-login / launcher / viewer / pure API against the same formal `public_testnet` world.
