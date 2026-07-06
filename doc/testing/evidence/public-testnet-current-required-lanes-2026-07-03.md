# Public Testnet Current Required Lanes (2026-07-03)

## Meta
- owner_role: `tpm` integration
- original_packet_task: GitHub issue #1846 / `task_c345e90f2bf24cfd9ae60b4d1f2756dd`
- current_refresh_task: GitHub issue #1855 / `task_bb618bcf2fbe4a889bfa18f9927af90b`
- faucet_recovery_task: GitHub issue #1868 / `task_17e9c8cd02014f938b29689a44cbfb89`
- scope: current formal `public_testnet` 11 required-lane packet
- manifest: `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- lanes_tsv: `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- verdict: `ready_for_live_candidate`
- live_candidate_allowed: `true`
- readiness_gate_result: `pass`
- readiness_review_run: `.tmp/runtime-bootstrap-1964/pr-final-readiness-after-review-fixes/public-testnet-20260706-101613`

## Purpose
This packet completes the current formal `public_testnet` required-lane table with all 11 active lanes used by `scripts/network-tier-public-testnet-readiness.sh`.

It proves the required-lane evidence packet is now all-pass. `scripts/network-tier-public-testnet-readiness.sh` returns `gate_result=pass`, `readiness_verdict=ready_for_live_candidate`, and `live_candidate_allowed=true` for controlled public testnet claims. It does not widen the claim boundary to mainnet-grade, production OC settlement, permissionless validator onboarding, or public validator admission.

## Current Lane Matrix
| Lane | Status | Boundary |
| --- | --- | --- |
| `public_rpc_ready` | `pass` | Fresh governed public RPC samples returned `200` twice with `ok`, liveness/readiness/sync, and height `1082 -> 1083`. |
| `explorer_public_ready` | `pass` | Fresh governed explorer samples returned `200` twice with matching world id and height `1082 -> 1083`. |
| `faucet_guard_ready` | `pass` | Fresh run142 faucet service evidence proves public root/health, guarded claim, cooldown, and confirmed transfer/account-state change. |
| `reset_policy_announced` | `pass` | Current governed 20260606 resettable/non-mainnet policy is explicitly recorded. |
| `runtime_bootstrap` | `pass` | Run142 clean validator rebuild advanced sequencer/storage committed and latest heights with `last_error=null`. |
| `world_resource_provenance_ready` | `pass` | Run142 live nodes report `world_resource.readiness_status=ready`, `failed_gates=[]`, and resource commits advancing with chain height. |
| `provider_resource_provenance_ready` | `pass` | Provider local mock bridge reports compatible world-resource manifest/delta schemas, required capabilities/action set, and health ready against the current governed `public_testnet` world-resource status. |
| `resource_delta_replay_ready` | `pass` | Fresh sequencer/storage samples bind live `world_resource` status to same-node execution-world manifest and committed latest resource delta at height `92`, with no failed gates, no provisional chunks, and no pending delta. |
| `api_viewer_projection_ready` | `pass` | Fresh same-window JSON binds public RPC status, storage status, and viewer-facing explorer overview to world `oasis7-public-testnet-governed-20260606` at height `111` with matching `last_block_hash` and ready `world_resource` status. |
| `same_world_hosted_entry_ready` | `pass` | Fresh hosted_public_join viewer_live v4 probe exposes pure API top-level and embedded runtime snapshot chain-resource context for world `oasis7-public-testnet-governed-20260606` at manifest height `1620`; node readiness and `world_resource` readiness were ready at height `1620`; no manual checkpoint or data copy was used. |
| `claims_boundary_review` | `pass` | Current QA/product/liveops boundary allows only controlled public_testnet live-candidate claims after the 11-lane readiness script passes, while continuing to deny mainnet, production settlement, and validator admission/onboarding claims. |

## Closure Summary
1. The fresh public surface sample includes a restored guarded faucet endpoint and claim path; the faucet lane is pass for the guarded/resettable testnet boundary.
2. The runtime lane and world-resource lane are closed by run142 live-node rebuild/deploy evidence.
3. Provider resource provenance is closed for schema/provenance compatibility.
4. Resource-delta replay is closed by fresh sequencer/storage same-node status/snapshot evidence.
5. API/viewer projection is closed by fresh same-window public RPC / storage status / viewer-facing explorer overview JSON evidence.
6. Same-world hosted entry is closed by fresh hosted_public_join launcher/viewer/pure API evidence against the same formal public_testnet world, with no manual checkpoint or data copy.

## Claim Boundary
Allowed:
- `formal public_testnet mechanism is documented`
- `current required-lane packet is complete`
- `all 11 formal public_testnet required lanes have pass evidence`
- `controlled public_testnet live-candidate claim is allowed by the script-generated readiness review`

Denied:
- `live public testnet is already online`
- `public validator admission is open`
- `mainnet ready`
- `mainnet-grade`
- `production OC settlement`

## Next Pass Requirements
No required-lane gap remains in this 11-lane packet. Further promotion beyond controlled `public_testnet` live-candidate claims must use a separate release decision path and must not infer mainnet-grade, production OC settlement, or permissionless validator onboarding from this evidence alone.
