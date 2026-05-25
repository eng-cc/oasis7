# Shared Network Track Gate Summary

- Track: `shared_devnet`
- Candidate ID: `shared-devnet-live-reset-20260523-01`
- Candidate bundle: `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- Gate result: `partial`
- Promotion recommendation: `hold_promotion`

## Required Lanes
- `candidate_bundle_integrity`: `present`
- `shared_access`: `present`
- `multi_entry_closure`: `present`
- `mixed_topology_baseline`: `present`
- `governance_live_drill`: `present`
- `short_window_longrun`: `present`
- `rollback_target_ready`: `present`

## Lane Status Table

| Lane | Owner | Status | Evidence | Note |
| --- | --- | --- | --- | --- |
| `candidate_bundle_integrity` | `qa_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json` | current live-reset candidate bundle validates for the 2026-05-23 triad |
| `shared_access` | `qa_engineer` | `partial` | `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` | current player entry is still loopback-only and no independent operator/access proof is pinned |
| `multi_entry_closure` | `qa_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` | current candidate has same-window headed web plus no-ui plus pure-api closure evidence after the 2026-05-23 live reset |
| `mixed_topology_baseline` | `qa_engineer` | `partial` | `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md` | only audited partial mixed-topology baseline exists; no current same-window uplift decision is pinned |
| `governance_live_drill` | `runtime_engineer` | `partial` | `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` | historical governance drill exists but no same-window post-reset governance recheck is pinned for this candidate |
| `short_window_longrun` | `runtime_engineer` | `partial` | `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` | fresh post-reset candidate has not yet re-run current S9 or S10 short-window evidence |
| `rollback_target_ready` | `liveops_community` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-rollback-contract-2026-05-23.md` | bootstrap_restore_ready fallback is now fully pinned for the current live-reset candidate |
