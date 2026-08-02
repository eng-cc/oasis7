# Legacy `shared_devnet` provenance authority

Status: historical/rehearsal record only. This is the single entry point for the retired `shared_devnet` evidence set; it is not a `public_testnet`, `mainnet`, release, or public-world readiness claim.

## Authority and boundary

- Current environment and network-tier authority: `doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md` and `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md`; current mechanism and readiness operations start from `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.runbook.md`.
- Current readiness and claim evidence: `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.md` and `doc/testing/evidence/public-testnet-claims-boundary-review-2026-07-06.md`.
- The retained legacy conclusion is: `shared_devnet` rehearsal reached `pass / eligible_for_promotion` on 2026-05-24, but that result does not promote, substitute for, or relax any formal `public_testnet` or `mainnet` gate.

## Replay identity and final gate state

The final historical run was `shared_devnet-20260524-101652`, for candidate `shared-devnet-live-reset-20260523-01` at source commit `d59e892ad1deb8cc612a56af67ce08e6c5d7ff97`. It recorded a dirty historical worktree, so it is replay/audit provenance rather than reproducible current release input.

The candidate bundle and lane ledger remain the machine-readable replay input:

- `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- `doc/testing/evidence/shared-network-shared-devnet-live-reset-lanes-2026-05-23.tsv`

Final lane disposition and retained evidence tuple:

| Lane | Owner | Status | Retained evidence |
| --- | --- | --- | --- |
| `candidate_bundle_integrity` | `qa_engineer` | `pass` | `shared-network-shared-devnet-live-reset-candidate-2026-05-23.json` |
| `shared_access` | `qa_engineer` | `pass` | `shared-network-shared-devnet-shared-access-2026-05-23.md` |
| `multi_entry_closure` | `qa_engineer` | `pass` | `shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md` |
| `mixed_topology_baseline` | `qa_engineer` | `pass` | `shared-network-shared-devnet-mixed-topology-2026-05-23.md` |
| `governance_live_drill` | `runtime_engineer` | `pass` | `shared-network-shared-devnet-governance-live-drill-2026-05-23.md` |
| `short_window_longrun` | `runtime_engineer` | `pass` | `shared-network-shared-devnet-short-window-pass-2026-05-23.md` |
| `rollback_target_ready` | `liveops_community` | `pass` | `shared-network-shared-devnet-rollback-contract-2026-05-23.md` |

The retained underlying evidence supplies the candidate identity, health/topology, access handoff, restore contract, commands, timestamps, and decision refs; the deleted generated summaries were repeatable renderings of those inputs.

## Capture chronology and Git provenance

All five captures are retained in Git at pre-retirement tree
`7d8bb4a7569e60f88d96124d0b604466b480705c`; the listed tree object is the
digest for its three generated files (`candidate_validation.json`,
`summary.json`, and `summary.md`).

| Capture | Gate / promotion | Pre-retirement Git tree |
| --- | --- | --- |
| `shared_devnet-20260523-191122` | `partial` / `hold_promotion` | `5ec4b20bd7a6a28836f980ac87ed38cf2fddf9fe` |
| `shared_devnet-20260523-191232` | `partial` / `hold_promotion` | `9b97eb05d6aaf94b18562811172a798ef7689f53` |
| `shared_devnet-20260523-194826` | `partial` / `hold_promotion` | `d60af4f4a0a44822fbb5dd9892e491bf32e44288` |
| `shared_devnet-20260523-214249` | `partial` / `hold_promotion` | `6c4de3a840875f931e21c9398e3454e2c18e4641` |
| `shared_devnet-20260524-101652` | `pass` / `eligible_for_promotion` | `9c89918b1d4542ff7f38adbfd8f030f0cee07858` |

## Retained underlying record

- 2026-03-30--2026-04-23 ECS triad predecessor topology and rollout snapshots: `shared-network-ecs-triad-node-inventory-2026-03-30.md`, `shared-network-ecs-triad-upgrade-2026-04-07.md`, and `shared-network-ecs-triad-chain-status-metrics-rollout-2026-04-23.md`. They preserve time-bound node roles, service status, topology, and deployed-binary observations; they are not final-run gate inputs and cannot establish current `public_testnet` or `mainnet` readiness.
- 2026-03-24 dry-run, promotion/hold, follow-up, and short-window records: `shared-network-shared-devnet-dry-run-2026-03-24.md`, `shared-network-shared-devnet-promotion-record-2026-03-24.md`, `shared-network-shared-devnet-incident-2026-03-24.md`, `shared-network-shared-devnet-follow-up-window-2026-03-24.md`, `shared-network-shared-devnet-follow-up-promotion-record-2026-03-24.md`, `shared-network-shared-devnet-follow-up-incident-2026-03-24.md`, `shared-network-shared-devnet-short-window-pass-2026-03-24.md`, `shared-network-shared-devnet-short-window-promotion-record-2026-03-24.md`, and `shared-network-shared-devnet-short-window-incident-2026-03-24.md`.
- 2026-05-23 candidate, access, topology, governance, longrun, and recovery records: `shared-network-shared-devnet-live-window-gap-audit-2026-05-23.md`, `shared-network-shared-devnet-shared-access-2026-05-23.md`, `shared-network-shared-devnet-mixed-topology-2026-05-23.md`, `shared-network-shared-devnet-governance-live-drill-2026-05-23.md`, `shared-network-shared-devnet-short-window-pass-2026-05-23.md`, `shared-network-shared-devnet-rollback-contract-2026-05-23.md`, and `shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md`.

## Retirement and recovery posture

`doc/testing/evidence/generated-shared-network-gates/` was retired on 2026-07-26: its five generated snapshots (15 files) had no inbound callers and duplicated the retained bundle, lane ledger, and evidence records while embedding stale absolute worktree paths. No current script may consume this legacy candidate as a manifest tier or release-candidate track; those scripts reject `shared_devnet` and direct new rehearsal work to `public_testnet_rehearsal`.

To audit the historical result, start with this authority, validate the retained candidate bundle and ledger against their recorded refs, then inspect the lane-specific retained record above. To recover or run a new rehearsal, use the formal public-testnet runbook and create a new current candidate; do not restore the deleted generated snapshots or restart the retired network as a substitute for a new evidence epoch.
