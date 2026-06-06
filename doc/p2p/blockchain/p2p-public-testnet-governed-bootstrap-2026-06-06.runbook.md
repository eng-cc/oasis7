# Public Testnet Governed Bootstrap Runbook (2026-06-06)

- 对应项目管理文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`
- 关联证据:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`
  - `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md`

审计轮次: 1

## 1. Purpose
1. This runbook is the operator-facing next step after signer-truth wiring and bootstrap artifact freezing.
2. It defines the clean rebuild path for the current honest `public_testnet` topology:
   - validator pair: `triad-testnet-sequencer`, `triad-testnet-storage`
   - observers: `triad-testnet-local`, `triad-testnet-fourth-local`
3. It does not claim that a healthy four-node network already exists. It defines how to build one from the fresh governed bootstrap artifacts.

## 2. Frozen bootstrap truth
- network-tier manifest:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- release candidate bundle:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
- genesis truth:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-genesis-2026-06-06.json`
- genesis validator registry:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json`
- bootstrap peers:
  - `doc/testing/evidence/public-testnet-governed-bootstrap-bootstrap-peers-2026-06-06.txt`

## 3. Topology contract
| node_id | role | genesis validator | bootstrap peer source | note |
| --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator | yes | yes | ECS/public seed peer |
| `triad-testnet-storage` | validator | yes | yes | ECS/public seed peer |
| `triad-testnet-local` | observer | no | after validator pair is healthy | existing local observer identity |
| `triad-testnet-fourth-local` | observer | no | after validator pair is healthy | fourth local observer identity |

## 4. Preconditions
0. Operator authorization for destructive reset is explicitly granted for this rebuild path.
   - The previous four-node `public_testnet` state may be fully deleted before bootstrap.
   - No historical execution world, replication root, or observer seed needs to be preserved for the fresh rebuild path.
1. Four target nodes start from empty or intentionally reset state directories.
2. The runtime binary used on validator bring-up matches the bundle hash pinned in:
   - `doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
3. The two validator hosts have:
   - a `node.env`
   - `NETWORK_TIER_MANIFEST_PATH` pointing to the governed bootstrap manifest
   - `GENESIS_VALIDATOR_REGISTRY_PATH` pointing to the governed bootstrap validator registry
4. Observer hosts must not invent a new validator registry. They join only after validator pair health is confirmed.

## 5. Bring-up order
1. Fully delete or reset the previous four-node state on the two validator hosts and two observer hosts.
2. Stage the same governed bootstrap manifest, genesis, bootstrap peers, and validator registry onto both validator hosts.
3. Start `triad-testnet-sequencer`.
4. Start `triad-testnet-storage`.
5. Wait until both validator nodes report:
   - `network_tier.tier=public_testnet`
   - `validator_policy.governance_mode=governance_registry`
   - `last_error=null`
   - non-zero `committed_height`
6. Only then attach observers:
   - `triad-testnet-local`
   - `triad-testnet-fourth-local`
7. Observers use the same `NETWORK_TIER_MANIFEST_PATH`, but no observer-specific genesis validator registry override should change validator truth.

## 6. Canonical validator start shape
Validator nodes should be launched through `scripts/p2p-triad-node-start.sh` with env containing:

```bash
NETWORK_TIER_MANIFEST_PATH=<staged>/public-testnet-governed-bootstrap-manifest-2026-06-06.json
GENESIS_VALIDATOR_REGISTRY_PATH=<staged>/public-testnet-governed-bootstrap-validator-registry-2026-06-06.json
```

The repo-owned start script will pass:
- `--network-tier-manifest`
- `--genesis-validator-registry`

and will not fall back to legacy `NODE_VALIDATORS_CSV` truth unless explicitly forced.

## 7. Verification checkpoints
### Validator pair
- `./scripts/network-tier-manifest.sh validate --manifest doc/testing/evidence/public-testnet-governed-bootstrap-manifest-2026-06-06.json`
- `./scripts/release-candidate-bundle.sh validate --bundle doc/testing/evidence/public-testnet-governed-bootstrap-bundle-2026-06-06.json`
- status checks on both validators should show:
  - `node_id`
  - `world_id=oasis7-public-testnet-governed-20260606`
  - `network_tier.tier=public_testnet`
  - `last_error=null`

### Observer attach
- observers should show:
  - `role=observer`
  - non-zero peer visibility
  - `network_committed_height` converging toward validator head

## 8. Failure boundaries
1. If validator pair cannot start from empty state with the governed registry, stop and treat it as a bootstrap artifact/runtime bug.
2. If validator pair is healthy but observers fail to attach, treat it as observer contract/runtime follow-up, not as a reason to mutate genesis validator truth.
3. If a node requires `NODE_VALIDATORS_CSV` to start the formal public tier, that is a regression against current governance-registry bootstrap design.

## 9. Next task boundary
This runbook is not itself the live operator execution record.

The actual bring-up should run under a dedicated task that records:
- exact staged paths per host
- exact runtime binary hash per host
- exact reset/backup evidence
- validator-pair health snapshots
- observer attach snapshots
- final four-node verdict
