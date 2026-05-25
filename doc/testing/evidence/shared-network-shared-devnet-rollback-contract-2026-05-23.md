# Shared Network Rollback Contract Refresh (2026-05-23)

审计轮次: 1

## Meta
- `track`: `shared_devnet`
- `window_id`: `shared-devnet-live-reset-20260523`
- `candidate_id`: `unfrozen_live_triad_after_reset`
- `owner`:
  - `runtime_engineer`

## Current live truth
- 2026-05-23 已对 live shared-devnet triad 执行受控冷重建，详见：
  - `doc/testing/evidence/shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md`
- 当前三节点在 fresh chain 上继续推进，说明“从受控备份点清空并恢复 shared-devnet triad”已具备真实可执行路径，而不再只是模板占位。

## Audited restore material
- `fallback_class`:
  - `bootstrap_restore_ready`
- `fallback_candidate_bundle_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-live-reset-candidate-2026-05-23.json`
- `fallback_gate_ref`:
  - `doc/testing/evidence/generated-shared-network-gates/shared_devnet-20260523-191122/summary.md`
- `fallback_owner_ref`:
  - `.pm/tasks/task_c52321688c6b4ea09a59e7d5db749190.execution.md`

## Restore steps now pinned
- Local backup roots:
  - `/opt/oasis7/p2p-triad-local/backups/20260523-183947-chain-reset`
  - `/opt/oasis7/p2p-triad-local/backups/20260523-184329-bridge-reset`
  - `/opt/oasis7/p2p-triad-local/backups/20260523-184825-join-observer-reset`
- ECS backup roots:
  - `/opt/oasis7/p2p-triad/backups/20260523-183948-chain-reset`
  - `/opt/oasis7/p2p-triad/backups/20260523-184331-bridge-reset`
  - `/opt/oasis7/p2p-triad/backups/20260523-184332-bridge-reset`
  - `/opt/oasis7/p2p-triad/backups/20260523-184545-solo-sequencer-reset`
  - `/opt/oasis7/p2p-triad/backups/20260523-184729-join-storage-reset`
- `restore_steps_ref`:
  - `doc/testing/evidence/shared-network-shared-devnet-triad-reset-recovery-2026-05-23.md`
- `restoration_scope`:
  - runtime execution state
  - execution records
  - storage CAS
  - node distfs replication roots
  - reward runtime execution bridge state

## Verified restore sequence
1. Preserve node identity/config, but rotate out live runtime data roots.
2. If restart still reports stale execution heights, also rotate out `reward-runtime-execution-bridge-state.json`.
3. Do not empty-boot all three nodes together; recover in order:
  - sequencer
  - storage
  - observer
4. Re-verify same-window progress on all three nodes before claiming the restore usable.

## Verdict
- `lane_result`:
  - `pass`
- `reason`:
  - first shared-devnet `pass` is still allowed to use a `bootstrap_restore_ready` fallback
  - the current live-reset window now has all five required fields pinned: fallback bundle, fallback gate, fallback owner, restore steps, and restoration scope
  - this fallback does not promise “rollback to a previous formal shared-devnet pass candidate”; it promises audited recovery back to the current live-reset candidate truth

## Residual caveat
- this `pass` is specifically the allowed first-pass `bootstrap_restore_ready` flavor
- if a later window wants to claim rollback against a previous formal shared-devnet pass candidate, it must replace this fallback with that later historical pass bundle/gate truth

## Notes
- This evidence upgrades rollback truth from “bootstrap fallback is only a placeholder idea” to “bootstrap fallback has a real, recently exercised recovery contract”.
- It does not override the formal rule in `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md`: first shared-devnet `pass` still needs the fallback bundle/gate/owner/steps/scope set to align with the same candidate/window.
