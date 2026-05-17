# public_testnet rehearsal record template

- tier: `public_testnet`
- rehearsal_window: `<YYYY-MM-DD HH:MM CST>`
- manifest_ref: `doc/testing/templates/network-tier-public-testnet.example.json`
- release_candidate_bundle_ref: `<path-or-tag>`
- rpc_ref: `<https://.../rpc>`
- explorer_ref: `<https://.../explorer>`
- faucet_ref: `<https://.../faucet>`
- reset_policy: `resettable`
- validator_admission: `allowlist_or_governed_candidate`
- claims_boundary:
  - allowed: `public_testnet`, `resettable_test_network`
  - denied: `mainnet_live`, `production_oc_settlement`

## Required gates
- [ ] `shared_devnet_pass`
- [ ] `public_rpc_ready`
- [ ] `faucet_guard_ready`
- [ ] `reset_policy_announced`

## Runtime verification
- [ ] `oasis7_chain_runtime --network-tier-manifest <path>` boots successfully
- [ ] `/v1/chain/status` reports `network_tier.tier=public_testnet`
- [ ] bootstrap peers resolve from `bootstrap_peer_ref`

## Evidence refs
- [ ] public RPC smoke
- [ ] explorer smoke
- [ ] faucet smoke
- [ ] reset policy announcement
- [ ] incident / rollback note if applicable

## Verdict
- rehearsal_verdict: `<pass|partial|block>`
- open_blockers:
  - `<blocker-1>`

## Follow-up
- readiness_review_command:
  - `./scripts/network-tier-public-testnet-readiness.sh --manifest <manifest> --lanes-tsv <lanes.tsv>`
