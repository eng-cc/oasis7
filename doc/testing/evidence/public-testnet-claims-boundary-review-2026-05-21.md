# public_testnet claims boundary review (2026-05-21)

## Meta
- owner_role: `qa_engineer`
- scope: live `public_testnet` claim boundary for `oasis7-public-testnet-parallel-20260518`
- lane_verdict: `claims_boundary_review=pass`
- aggregate_readiness_impact: `overall_readiness` remains blocked until `shared_devnet_pass` is satisfied

## Reviewed inputs
- `doc/testing/evidence/public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`
- `doc/testing/evidence/p2p-public-testnet-faucet-service-2026-05-19.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md`
- `doc/testing/templates/network-tier-public-testnet.example.json`

## QA verdict
- Verdict: `pass`
- Summary: repo-owned evidence is now sufficient to prove that the visible public wording can stay inside the `public_testnet/resettable/guarded faucet/non-mainnet` boundary without drifting into `mainnet_live`, `production_oc_settlement`, or `ready_for_live_candidate`.

## Allowed claims reviewed
- Allowed: describe the current network as a public `public_testnet` test surface with public RPC and explorer reachability.
- Allowed: state that the network is `resettable`, and that reset semantics were publicly announced through the reset-policy path already cited by the endpoint deploy evidence.
- Allowed: state that faucet access exists only as a guarded testnet faucet with explicit `amount`, `cooldown_secs`, and `oc:pk:` account-format constraints.
- Allowed: state that the current public endpoints are:
  - RPC: `http://39.104.204.172:6631/v1/chain/status`
  - Explorer: `http://39.104.205.67:6632/v1/chain/explorer/overview`
  - Faucet: `http://39.104.204.172:6681/`

## Denied claims reviewed
- Denied: `mainnet_live`
- Denied: `production_oc_settlement`
- Denied: `public validator admission is open`
- Denied: `ready_for_live_candidate`
- Denied: `shared_devnet_pass`
- Denied: any wording that treats faucet-distributed `OC` on this network as production-value settlement or as a no-reset/frozen network

## Evidence mapping
- Public RPC / explorer are reachable and already recorded as `pass` lane inputs in `public-testnet-live-candidate-endpoint-deploy-2026-05-19.md`.
- The same evidence explicitly keeps aggregate readiness at `block`, so this review does not widen the current promotion verdict.
- Guarded faucet boundary is independently evidenced by `p2p-public-testnet-faucet-service-2026-05-19.md`, including:
  - `amount = 1000000`
  - `cooldown_secs = 3600`
  - `target account format must be oc:pk:<64-hex>`
- The formal manifest/PRD contract still freezes `allowed_claims = public_testnet/resettable_test_network` and `denied_claims = mainnet_live/production_oc_settlement`.
- The companion runbook still forbids early claims such as `live public testnet is established`, `public validator onboarding is open`, and `production OC settlement`.

## Residual blockers outside this lane
- `shared_devnet_pass` remains `partial`, so this review does not unlock `ready_for_live_candidate`.
- Aggregate readiness should continue to remain `block` until the shared-devnet promotion prerequisite is satisfied.

## Final QA note
- This review is only a claims-boundary verdict.
- It does not certify long-window stability, shared-devnet promotion, or mainnet gating readiness.
