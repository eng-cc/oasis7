# Shared Network Track Gate Summary

- Track: `shared_devnet`
- Candidate ID: `shared-devnet-live-reset-20260523-01`
- Candidate bundle: `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- Gate result: `pass`
- Promotion recommendation: `eligible_for_promotion`

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
| `shared_access` | `qa_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-shared-access-2026-05-23.md` | cloud shared endpoint, operator handoff, and same-window independent access evidence are now pinned for the 2026-05-23 candidate window |
| `multi_entry_closure` | `qa_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` | current candidate has same-window headed web plus no-ui plus pure-api closure evidence after the 2026-05-23 live reset |
| `mixed_topology_baseline` | `qa_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-2026-05-23.md` | current same-window mixed deployment evidence now includes live catch-up to `committed_height=1280` on all three validators plus a producer/QA pass uplift decision ref |
| `governance_live_drill` | `runtime_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-governance-live-drill-2026-05-23.md` | same-window post-reset governance pass/block/rejoin/restore drill is now pinned for the current candidate |
| `short_window_longrun` | `runtime_engineer` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-05-23.md` | current candidate now has same-window S9 and S10 short-window evidence after the longrun fixes and runtime rebuild |
| `rollback_target_ready` | `liveops_community` | `pass` | `doc/testing/evidence/shared-network-shared-devnet-rollback-contract-2026-05-23.md` | bootstrap_restore_ready fallback is now fully pinned for the current live-reset candidate |
