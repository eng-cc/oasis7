# public_testnet claims boundary review (2026-07-06)

## Meta
- owner_role: `qa_engineer` / `producer_system_designer` / `liveops_community`
- scope: current governed `public_testnet` 11-lane readiness packet for `oasis7-public-testnet-governed-20260606`
- lane_verdict: `claims_boundary_review=pass`
- aggregate_readiness_impact: controlled `public_testnet` live-candidate claims are allowed only when `scripts/network-tier-public-testnet-readiness.sh` returns `gate_result=pass` and `live_candidate_allowed=true`

## Reviewed inputs
- `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv`
- `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md`
- `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- `doc/testing/evidence/public-testnet-same-world-hosted-entry-2026-07-05.json`
- `scripts/network-tier-public-testnet-readiness.sh`

## QA / Product / LiveOps verdict
- Verdict: `pass`
- Summary: the current packet may claim a controlled, resettable `public_testnet` live-candidate only after all 11 formal required lanes pass the readiness script. This updates the older 2026-05-21 boundary that denied `ready_for_live_candidate` while the prior packet was still blocked.

## Allowed claims reviewed
- Allowed: `formal public_testnet mechanism is documented`
- Allowed: `current required-lane packet is complete`
- Allowed: `all 11 formal public_testnet required lanes have pass evidence`
- Allowed: `controlled public_testnet live-candidate claim is allowed by the script-generated readiness review`
- Allowed: the network is resettable, non-mainnet, and uses guarded testnet faucet boundaries.

## Denied claims reviewed
- Denied: `live public testnet is already online`
- Denied: `mainnet ready`
- Denied: `mainnet-grade`
- Denied: `mainnet_live`
- Denied: `production OC settlement`
- Denied: `public validator admission is open`
- Denied: `public validator onboarding open`
- Denied: `permissionless validator onboarding`
- Denied: any wording that treats faucet-distributed `OC` on this network as production-value settlement or as a no-reset/frozen network.

## Evidence mapping
- The readiness script owns aggregate promotion wording. It must return `gate_result=pass`, `readiness_verdict=ready_for_live_candidate`, and `live_candidate_allowed=true` before the controlled live-candidate claim is allowed.
- The `claims_boundary_review` lane does not independently promote the network. It only confirms the claim envelope for the current aggregate readiness result.
- The same-world hosted entry evidence must keep `manual_checkpoint_or_data_copy_used=false`, and its raw chain status sample must prove node readiness plus `world_resource` readiness.
- Mainnet, production settlement, and validator admission/onboarding remain out of scope even when the controlled public-testnet live-candidate claim is allowed.

## Residual risk
- This review does not certify mainnet-grade security, production OC settlement, full light-client security, multi-client equivalence, or permissionless validator operations.
- External wording must keep the controlled/resettable/non-mainnet boundary visible.
