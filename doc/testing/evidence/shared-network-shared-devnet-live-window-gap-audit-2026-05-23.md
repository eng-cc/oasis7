# shared_devnet live window gap audit (2026-05-23)

## Meta
- owner_role: `qa_engineer`
- track: `shared_devnet`
- window_id: `shared-devnet-live-reset-20260523`
- candidate_id: `shared-devnet-live-reset-20260523-01`

## Confirmed current strengths
- Live triad runtime has recovered on a fresh chain:
  - `doc/testing/evidence/shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md`
- Current release/runtime contract is aligned across local observer and both ECS nodes:
  - release path `d104864026bb-triad-full-game-nodes-20260516-213138`
  - world id `shared-devnet-ecs-v1`
  - same three-validator contract on all nodes

## Lane-by-lane current gaps
### `shared_access`
- upgrade to `pass`
- reason:
  - old local loopback-only blocker has been removed by moving shared player entry onto the cloud sequencer public IPv4:
    - `http://39.104.204.172:4173/software_safe.html?ws=ws://39.104.204.172:5011&test_api=1`
    - `39.104.204.172:5011`
    - `39.104.204.172:5023`
    - `39.104.204.172:5631`
  - current same-window evidence is recorded in:
    - `doc/testing/evidence/shared-network-shared-devnet-shared-access-2026-05-23.md`
  - independent same-window access proof is now pinned from:
    - this workstation against the public viewer URL
    - the cloud storage host against the sequencer private address `172.26.53.91:4173`
  - operator/handoff truth is pinned in the task execution log for this same candidate window

### `multi_entry_closure`
- upgrade to `pass`
- reason:
  - this fresh post-reset candidate now has same-window closure across all three player-entry surfaces:
    - headed web:
      - `.tmp/shared-devnet-live-reset-20260523-01/multi-entry/web-fix2/post-onboarding-20260523-194738/post-onboarding-summary.json`
    - no-ui live TCP:
      - `.tmp/shared-devnet-live-reset-20260523-01/multi-entry/headless-fix5/post-onboarding-headless-20260523-194411/post-onboarding-headless-summary.json`
    - pure_api:
      - `.tmp/shared-devnet-live-reset-20260523-01/multi-entry/pure-api-fix/pure-api-required-20260523-193936/pure-api-summary.json`
  - all three routes now accept the current live shared-devnet entry contract where the initial stage may already be `post_onboarding`
  - the closure remains candidate-aligned with `shared-devnet-live-reset-20260523-01`

### `mixed_topology_baseline`
- upgrade to `pass`
- reason:
  - current same-window mixed deployment evidence is now pinned in:
    - `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-2026-05-23.md`
  - that packet reconciles the live topology change:
    - local `triad-observer-local` is currently configured as `NODE_ROLE=sequencer`
    - all three nodes participate in the same three-validator contract
  - same-window live recovery has now been demonstrated against the current candidate:
    - `2026-05-24 10:13 CST`: local + ECS sequencer reached `1277/1277/1277`, storage reached `1276/1277` with `pending_height=1277 status=committed`
    - `2026-05-24 10:13:55 CST`: local, ECS sequencer, and ECS storage all converged to `committed_height=1280`, `last_execution_height=1280`, `network_committed_height=1280`, `pending_height=null`, `last_error=null`
  - producer/QA `pass_uplift_decision_ref` is now pinned in the task execution log for this same window

### `governance_live_drill`
- upgrade to `pass`
- reason:
  - a same-window post-reset governance recheck now exists:
    - `doc/testing/evidence/shared-network-shared-devnet-governance-live-drill-2026-05-23.md`
  - the current frozen candidate world did not begin with populated governance finality registry state, but the same-window drill proved:
    - baseline manifest import succeeds
    - finality `signer03 -> signer04` pass rotation succeeds
    - degraded `2-of-2` finality is blocked
    - rejoin succeeds
    - restore to baseline returns `overall_status=ready_for_ops_drill`

### `short_window_longrun`
- upgrade to `pass`
- reason:
  - current post-reset live candidate now has same-window S9/S10 short-window evidence:
    - canonical evidence:
      - `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-05-23.md`
    - S9:
      - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s9/20260523-231118/summary.json`
    - S10:
      - `.tmp/shared-devnet-live-reset-20260523-01/shared-devnet-live-reset-20260523-longrun-fix2/longrun/s10/20260523-231621/summary.json`
  - the same-window rehearse/fix path also proved two operator-facing corrections:
    - `scripts/p2p-longrun-soak.sh` restart chaos now uses graceful `SIGINT` before fallback kill, avoiding reward-runtime state-root mismatch on sequencer restart
    - `target/debug/oasis7_chain_runtime` was rebuilt after the gap-sync fetch route fix, and the fresh S10 run no longer emitted `fetch-commit ErrUnsupported`

## Conclusion
- Current shared-devnet truth is no longer blocked on runtime liveness.
- `multi_entry_closure` is now closed for the freshly recovered live window.
- `shared_access` is now closed for the freshly recovered live window.
- `governance_live_drill` is now closed for the freshly recovered live window.
- `short_window_longrun` is now closed for the freshly recovered live window.
- `mixed_topology_baseline` is now also closed for the freshly recovered live window.
- Current same-window evidence supports rerunning the canonical shared-network gate with all required lanes at `pass`.
