# shared_devnet triad cold reset recovery (2026-05-23)

## Meta
- owner_role: `runtime_engineer`
- scope: recover the live shared-devnet triad after the local observer and ECS sequencer/storage all stopped making forward progress on old chain history
- aggregate_readiness_impact: clears the current runtime-stall incident, but does not by itself upgrade formal `shared_devnet_pass`

## Failure that triggered the reset
- Pre-reset live truth had stopped on divergent historical state:
  - local observer had been restored to `committed_height=733`, but still trailed the live chain badly
  - ECS sequencer/storage no longer formed a recoverable same-history source for fresh seed/catch-up
- Seed-lineage spot checks confirmed the available backups were not safe “resume this exact chain” inputs:
  - `scripts/p2p-public-testnet-seed-lineage-check.sh` reported `lineage_verdict=divergent` for `733` vs `2515`
  - the older `75773` backup only produced `inconclusive`, not a clean lineage match

## Controlled reset actions
1. Backed up and rotated runtime state on all three nodes.
  - local backup root: `/opt/oasis7/p2p-triad-local/backups/20260523-183947-chain-reset`
  - ECS backup root: `/opt/oasis7/p2p-triad/backups/20260523-183948-chain-reset`
2. Cleared per-node runtime state while preserving node identity/config.
  - `output/node-distfs/<node_id>`
  - `data/execution-world`
  - `data/execution-records`
  - `data/storage`
  - `data/execution-world-simulator-mirror` when present
3. After the first restart still failed with stale-height restore errors, also backed up and cleared:
  - local: `/opt/oasis7/p2p-triad-local/backups/20260523-184329-bridge-reset`
  - ECS sequencer: `/opt/oasis7/p2p-triad/backups/20260523-184331-bridge-reset`
  - ECS storage: `/opt/oasis7/p2p-triad/backups/20260523-184332-bridge-reset`
  - cleared path: `output/chain-runtime/<node>/reward-runtime-execution-bridge-state.json`
4. Switched from “all three empty-boot together” to staggered recovery:
  - sequencer solo reset: `/opt/oasis7/p2p-triad/backups/20260523-184545-solo-sequencer-reset`
  - storage join reset: `/opt/oasis7/p2p-triad/backups/20260523-184729-join-storage-reset`
  - observer join reset: `/opt/oasis7/p2p-triad-local/backups/20260523-184825-join-observer-reset`

## Root causes confirmed during recovery
- Old chain history was not self-consistent enough to keep seeding from the retained stores.
- `reward-runtime-execution-bridge-state.json` was a second persistence layer that kept re-injecting stale execution heights even after the obvious data directories were cleared.
- Simultaneous cold boot of three empty nodes reproduced `latest state root mismatch tick=1`; bringing validators back in a staggered order avoided that bootstrap race.

## Post-reset live verification
- First same-window sample after staggered rebuild:
  - local observer: `committed_height=24`, `network_committed_height=24`, `known_peer_heads=1`, `last_error=null`
  - ECS sequencer: `committed_height=24`, `network_committed_height=24`, `known_peer_heads=2`, `last_error=null`
  - ECS storage: `committed_height=24`, `network_committed_height=24`, `known_peer_heads=1`, `last_error=null`
- Delayed recheck about 12 seconds later:
  - local observer: `26/26`, `last_error=null`
  - ECS sequencer: `27/27`, `last_error=null`
  - ECS storage: `26/27`, `last_error=null`

## Conclusion
- The old shared-devnet chain history was intentionally discarded.
- The live triad is now back on a fresh shared-devnet chain and is making forward progress again across all three nodes.
- This clears the immediate runtime blocker behind the recent `shared_devnet`/testnet follow-up.
- Formal `shared_devnet_pass` is still not satisfied, because the remaining gap is now back to process/lane evidence:
  - `shared_access`
  - `rollback_target_ready`
  - `mixed_topology_baseline`
