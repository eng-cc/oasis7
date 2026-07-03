# Public Testnet Current Required Lanes (2026-07-03)

## Meta
- owner_role: `tpm` integration
- original_packet_task: GitHub issue #1846 / `task_c345e90f2bf24cfd9ae60b4d1f2756dd`
- current_refresh_task: GitHub issue #1855 / `task_bb618bcf2fbe4a889bfa18f9927af90b`
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
| `public_rpc_ready` | `pass` | Fresh governed public RPC samples returned `200` twice with `ok`, liveness/readiness/sync, and height `1082 -> 1083`. |
| `explorer_public_ready` | `pass` | Fresh governed explorer samples returned `200` twice with matching world id and height `1082 -> 1083`. |
| `faucet_guard_ready` | `block` | Fresh faucet samples failed to connect twice; no public-faucet-open claim is allowed. |
| `reset_policy_announced` | `pass` | Current governed 20260606 resettable/non-mainnet policy is explicitly recorded. |
| `runtime_bootstrap` | `block` | Local observer sync path exists, but remaining live-head catch-up and release/runtime drift prevent pass. |
| `world_resource_provenance_ready` | `block` | Only cold-start placeholder/rehearsal world evidence exists; no current manifest/world/chain-bound pass. |
| `provider_resource_provenance_ready` | `block` | No current provider resource manifest/delta compatibility evidence exists for governed `public_testnet`. |
| `resource_delta_replay_ready` | `block` | No current committed resource-delta replay pass evidence exists. |
| `api_viewer_projection_ready` | `block` | No same-window API/viewer projection JSON pass evidence exists. |
| `same_world_hosted_entry_ready` | `block` | No `oasis7.same_world_hosted_entry.v1` JSON pass evidence exists. |
| `claims_boundary_review` | `pass` | QA claim boundary remains valid and does not widen readiness. |

## Blocker Summary
1. The governed-bootstrap manifest is still `status=rehearsal`.
2. The fresh public surface sample shows RPC/explorer progress, but the faucet endpoint is unreachable.
3. The live node still exposes the older 7-lane `network_tier.required_gates`, while repo readiness uses the current 11-lane contract.
4. The runtime lane remains blocked by bootstrap/recovery and deployment-contract drift rather than public RPC reachability.
5. World/provider/resource-delta lanes do not have current pass evidence tied to the governed `public_testnet` manifest/world/chain identity.
6. API/viewer and same-world hosted entry lanes do not have the required same-window JSON evidence.

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

1. Restore and revalidate the guarded faucet endpoint for governed `public_testnet`.
2. Redeploy or realign the live network-tier manifest surface so live `required_gates` matches the repo current 11-lane contract.
3. Healthy governed runtime bootstrap from the current manifest/bundle/genesis/bootstrap peer set.
4. Manifest/world/chain-bound world resource provenance evidence.
5. Provider resource provenance and committed resource-delta replay evidence.
6. Same-window API/viewer projection JSON evidence.
7. `oasis7.same_world_hosted_entry.v1` JSON evidence for hosted-login / launcher / viewer / pure API against the same formal `public_testnet` world.
