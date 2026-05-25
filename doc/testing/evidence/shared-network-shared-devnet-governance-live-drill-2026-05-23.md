# shared_devnet governance live drill (2026-05-23)

## Meta
- owner_role: `runtime_engineer`
- track: `shared_devnet`
- window_id: `shared-devnet-live-reset-20260523`
- candidate_id: `shared-devnet-live-reset-20260523-01`
- target slot: `governance.finality.v1`

## Inputs
- candidate bundle:
  - `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- frozen world snapshot:
  - `.tmp/shared-devnet-live-reset-20260523-01-world`
- drill working copy:
  - `.tmp/shared-devnet-live-reset-20260523-01-governance-world`
- baseline manifest:
  - operator-local `oasis7-governance-batch-20260323-01/public_manifest.json`
- replacement path:
  - `signer03 -> signer04`
- replacement public key:
  - `0125630c7502ed27a93e4ed3007eb49d111cb97612bc23f7452cfa29af94acbb`
- drill bundle:
  - `.tmp/shared-devnet-live-reset-20260523-01/governance-finality-signer04`

## Command
```bash
./scripts/governance-registry-live-drill.sh \
  --source-world-dir .tmp/shared-devnet-live-reset-20260523-01-governance-world \
  --baseline-manifest <operator-local oasis7-governance-batch-20260323-01/public_manifest.json> \
  --slot-id governance.finality.v1 \
  --replace-signer-id signer03 \
  --replacement-signer-id signer04 \
  --replacement-public-key 0125630c7502ed27a93e4ed3007eb49d111cb97612bc23f7452cfa29af94acbb \
  --out-dir .tmp/shared-devnet-live-reset-20260523-01/governance-finality-signer04
```

## Result summary
- drill bundle summary:
  - `.tmp/shared-devnet-live-reset-20260523-01/governance-finality-signer04/summary.json`
  - `.tmp/shared-devnet-live-reset-20260523-01/governance-finality-signer04/summary.md`
- phase truth:
  - `baseline_pre.audit_rc=1`
  - `pass_case.expectation_met=true`
  - `block_case.expectation_met=true`
  - `rejoin_case.expectation_met=true`
  - `restore.expectation_met=true`

## Important observation
- The frozen post-reset candidate world did not start with a populated governance finality signer registry.
- Baseline pre-audit therefore failed with:
  - `oasis7_governance_registry_audit failed: world is missing governance finality signer registry`
- This is current-window operational truth after the 2026-05-23 cold rebuild, not a drill script mismatch.

## What the same-window drill proved
1. The current candidate window can import the baseline governance manifest into the post-reset frozen world.
2. The finality rotation path `signer03 -> signer04` still passes under `2-of-3`.
3. The degraded `2-of-2` finality case is still blocked by the failover audit gate.
4. Rejoin back to the rotated `2-of-3` manifest succeeds.
5. Restoring the baseline manifest returns the world to:
   - `overall_status=ready_for_ops_drill`
   - `manifest_match_pass=true`
   - `overall_single_failure_tolerance_pass=true`

## Verdict
- lane result: `pass`
- reason:
  - a same-window post-reset governance recheck has now been executed against the current candidate truth
  - the drill exercised pass / block / rejoin / restore semantics on the current frozen shared-devnet world
  - the restored world ends in `ready_for_ops_drill`, which is sufficient to close the `governance_live_drill` lane for this window

## Scope boundary
- This evidence closes the shared-network lane `governance_live_drill` for the current candidate window.
- It does not claim that the post-reset candidate world already had governance registry state preloaded before the drill.
- It does not replace broader governance custody / ceremony / operator process evidence outside this lane.
