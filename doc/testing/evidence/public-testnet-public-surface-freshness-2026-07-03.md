# Public Testnet Public Surface Freshness (2026-07-03)

## Meta
- owner_role: `tpm` integration
- task: GitHub issue #1855 / `task_bb618bcf2fbe4a889bfa18f9927af90b`
- scope: fresh public RPC / explorer / faucet observation for current formal `public_testnet`
- companion JSON: `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.json`
- sample window: `2026-07-03T15:44:03Z` to `2026-07-03T15:44:43Z`
- verdict: public RPC and explorer are fresh/progressing; faucet is blocked

## Observed Public Surface
| Surface | URL | Result |
| --- | --- | --- |
| RPC | `http://39.104.204.172:6631/v1/chain/status` | `200` twice; `ok=true`; `world_id=oasis7-public-testnet-governed-20260606`; height `1082 -> 1083`; `liveness.status=ok`; `readiness.status=ready`; `sync.status=synced` |
| Explorer | `http://39.104.205.67:6632/v1/chain/explorer/overview` | `200` twice; `ok=true`; `world_id=oasis7-public-testnet-governed-20260606`; height `1082 -> 1083` |
| Faucet | `http://39.104.204.172:6681/` | connection failed twice; HTTP code `000`; no fresh guarded faucet response |

## Lane Impact
- `public_rpc_ready`: can move from `partial` to `pass` for public reachability/freshness.
- `explorer_public_ready`: can move from `partial` to `pass` for public reachability/freshness.
- `faucet_guard_ready`: must move from `partial` to `block`; the endpoint is not reachable in the fresh window.
- `runtime_bootstrap`: remains `block`; this sample proves public status progress, not full bootstrap/pass closure.
- `ready_for_live_candidate`: remains denied.

## Deployment Caveats
1. The live node still reports `network_tier.status=rehearsal`.
2. The live node still exposes an older 7-lane `network_tier.required_gates` set containing `shared_devnet_pass`, while the repo current readiness contract uses 11 active required lanes.
3. The faucet endpoint is not currently reachable, so no public-faucet-open claim is allowed.

## Claim Boundary
Allowed:
- `fresh public RPC and explorer surfaces are reachable and progressing for governed public_testnet`
- `faucet is currently blocked/unreachable`
- `overall public_testnet readiness remains blocked`

Denied:
- `public faucet is open`
- `ready_for_live_candidate`
- `live public testnet is fully open`
- `public validator admission is open`
- `production OC settlement`
