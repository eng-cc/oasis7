# Shared Network Mixed-Topology Gate Evidence

审计轮次: 1

## Meta
- `window_id`:
  - `shared-devnet-live-reset-20260523`
- `track`:
  - `shared_devnet`
- `candidate_id`:
  - `shared-devnet-live-reset-20260523-01`
- `owner`:
  - `qa_engineer`

## Candidate Truth
- `candidate_bundle_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- `candidate_gate_summary_ref`:
  - `doc/testing/evidence/generated-shared-network-gates/shared_devnet-20260524-101652/summary.md`

## Mixed-Topology Inputs
- `baseline_evidence_ref`:
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md`
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md`
  - `doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md`
- `same_window_shared_evidence_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md`
  - `doc/testing/evidence/shared-network-shared-devnet-shared-access-2026-05-23.md`
  - `doc/testing/evidence/shared-network-shared-devnet-short-window-pass-2026-05-23.md`
- `proxy_drill_ref`:
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-03.md`
  - `doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md`

## Validation
- `validated_by`:
  - `qa_engineer + producer_system_designer`
- `validated_at`:
  - `2026-05-24 10:14:18 CST`
- `validation_expectations`:
  - `same-window mixed deployment evidence is pinned against the current candidate truth`
  - `the current local node role is reconciled against live config instead of assuming observer-only topology`
  - `proxy drill evidence remains labeled approximation, not dedicated sentry/NAT lab truth`
  - `producer/QA pass uplift is only allowed after the local validator catches up to the cloud live head in the same window`

## Current Mixed Deployment Truth
- `environment_shape`:
  - `1` local workstation node + `2` Alibaba Cloud ECS nodes
- `topology_boundary`:
  - this is a real mixed deployment footprint, but not a dedicated sentry/NAT/CGNAT lab
- `role_reconciliation`:
  - local node `triad-observer-local` is no longer observer-only in config
  - local `/opt/oasis7/p2p-triad-local/config/node.env` pins `NODE_ROLE=sequencer`
  - local status reports `role=sequencer`, `node_role_claim=validator_core`
  - all three nodes now share the same validator set contract:
    - `triad-observer-local:100`
    - `triad-sequencer-a:100`
    - `triad-storage-b:100`
- `same_window_status_samples`:
  - catch-up recovery sample, `2026-05-24 10:13 CST`:
    - local workstation `127.0.0.1:5633`:
      - `committed_height=1277`
      - `last_execution_height=1277`
      - `network_committed_height=1277`
      - `known_peer_heads=2`
      - `last_error=null`
    - ECS sequencer `127.0.0.1:5631`:
      - `committed_height=1277`
      - `last_execution_height=1277`
      - `network_committed_height=1277`
      - `known_peer_heads=2`
      - `last_error=null`
    - ECS storage `127.0.0.1:5632`:
      - `committed_height=1276`
      - `last_execution_height=1276`
      - `network_committed_height=1277`
      - `pending_height=1277`
      - `pending_status=committed`
      - `attestation_count=2`
      - `approved_stake=200`
      - `last_error=null`
  - stable same-window convergence sample, `2026-05-24 10:13:55 CST`:
    - local workstation `127.0.0.1:5633`:
      - `committed_height=1280`
      - `last_execution_height=1280`
      - `network_committed_height=1280`
      - `pending_height=null`
      - `last_error=null`
    - ECS sequencer `127.0.0.1:5631`:
      - `committed_height=1280`
      - `last_execution_height=1280`
      - `network_committed_height=1280`
      - `pending_height=null`
      - `last_error=null`
    - ECS storage `127.0.0.1:5632`:
      - `committed_height=1280`
      - `last_execution_height=1280`
      - `network_committed_height=1280`
      - `pending_height=null`
      - `last_error=null`

## What This Window Proved
- the current shared-devnet candidate does have real same-window mixed deployment evidence, not just April proxy-only artifacts
- local and cloud nodes agree on:
  - `world_id=shared-devnet-ecs-v1`
  - replication protocol registration
  - three-validator contract
- the local node no longer stalls behind the cloud live head; after syncing from the live `triad-storage-b` state, all three validators converged at the same live height in the same window
- the window now includes an explicit producer/QA uplift decision ref instead of relying on free-text optimism

## Verdict
- `lane_result`:
  - `pass`
- `reason`:
  - same-window mixed deployment evidence is now paired with same-window catch-up and continued head advancement: the local workstation validator, ECS sequencer, and ECS storage node all converged to `committed_height=1280 / network_committed_height=1280 / last_execution_height=1280` with `pending_height=null` and `last_error=null`, so the lane now satisfies the current candidate's shared-window mixed-topology baseline.
- `pass_uplift_decision_ref`:
  - `.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md (2026-05-24 10:14:18 CST qa_engineer recheck + 2026-05-24 10:14:54 CST producer_system_designer uplift approval)`

## Explicit Caveats
- proxy drill evidence remains approximation, not dedicated sentry/NAT lab truth
- this `pass` only upgrades the current `shared_devnet` mixed-topology lane for `shared-devnet-live-reset-20260523-01`; it does not upgrade public-claims readiness or dedicated-lab coverage

## Notes
- This packet supersedes the old April explanation that only said “no current same-window mixed-topology evidence is pinned.”
- It does not claim observer-only topology; current repo truth is a mixed deployment of three validator-core sequencer nodes across one local workstation and two ECS hosts.
- It still does not upgrade the lane to dedicated lab truth or public-claims readiness.
