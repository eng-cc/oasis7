# Public Testnet Governed Reset Policy Announcement (2026-07-03)

## Meta
- owner_role: `producer_system_designer`
- review_roles: `liveops_community`, `qa_engineer`
- task: GitHub issue #1855 / `task_bb618bcf2fbe4a889bfa18f9927af90b`
- network_id: `oasis7-public-testnet-governed-20260606`
- chain_id: `oasis7-public-testnet-governed-20260606`
- lane_verdict: `reset_policy_announced=pass`
- aggregate_readiness_impact: does not unlock `ready_for_live_candidate`

## Announcement
`oasis7-public-testnet-governed-20260606` is a resettable `public_testnet` rehearsal network.

The network may be reset, rebuilt, reseeded, or replaced while formal readiness lanes remain incomplete. Testnet OC and testnet state on this network have no production settlement value and no no-reset or mainnet permanence guarantee.

## Allowed Claims
- `public_testnet`
- `resettable_test_network`
- `guarded_testnet_faucet` only when the faucet endpoint is actually reachable and guarded
- `non-mainnet value semantics`
- `governed-bootstrap rehearsal`

## Denied Claims
- `mainnet_live`
- `production_oc_settlement`
- `ready_for_live_candidate`
- `public faucet is open` while the current faucet endpoint is unreachable
- `public validator admission is open`
- `no-reset commitment`

## Evidence Binding
- current manifest: `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- current lane packet: `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- current public surface sample: `doc/testing/evidence/public-testnet-public-surface-freshness-2026-07-03.md`

## Residual Risk
This announcement closes the governed-network reset-policy reference gap only. It does not prove faucet reachability, runtime bootstrap, same-world hosted entry, API/viewer projection, resource provenance, or resource-delta replay.
