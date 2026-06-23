# Public Testnet Five-Node Inventory (2026-06-23)

## Purpose
This evidence records the current operator-facing `public_testnet` deployment inventory after the 2026-06-23 package update and catch-up run.

It does not replace the frozen governed bootstrap topology in `doc/testing/evidence/public-testnet-governed-bootstrap-topology-2026-06-06.md`. That older evidence remains the bootstrap artifact truth for the two-validator / two-observer governed topology. This file records the current live managed fleet that operators must update and verify.

No secret values are recorded here.

## Current Managed Fleet
| node_id | role | host class | validator set | package platform | stack root | service manager | status endpoint |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `triad-testnet-sequencer` | validator / sequencer | ECS Linux | yes | `linux-x64` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-sequencer.service` | `http://127.0.0.1:6631/v1/chain/status` |
| `triad-testnet-storage` | validator / storage | ECS Linux | yes | `linux-x64` | `/opt/oasis7/p2p-testnet` | `oasis7-triad-storage.service` | `http://127.0.0.1:6632/v1/chain/status` |
| `triad-testnet-local` | observer | Linux LAN | no | `linux-x64` | `/opt/oasis7/p2p-testnet-local` | `oasis7-testnet-observer.service` | `http://127.0.0.1:6633/v1/chain/status` |
| `triad-testnet-windows-observer` | observer | Windows LAN | no | `windows-x64` | `C:\oasis7-deploy` | scheduled task `Oasis7Observer` | `http://127.0.0.1:5121/v1/chain/status` |
| `triad-testnet-fourth-local` | observer | local macOS | no | macOS/local runtime or `macos-x64` package | `$OASIS7_TESTNET_FOURTH_ROOT` | launchd `oasis7.testnet.fourth` | `http://127.0.0.1:19083/v1/chain/status` |

`$OASIS7_TESTNET_FOURTH_ROOT` is the operator-local macOS observer root used by the current deployment. The concrete local home path belongs in task execution evidence, not in reusable docs.

## Inventory Boundary
1. The managed fleet is five nodes: two ECS validators, the Linux LAN observer, the Windows observer, and the local macOS observer.
2. Old bootstrap staging directories such as `.tmp/testnet-local-node-bootstrap` and `.tmp/testnet-fourth-node-bootstrap` are not deploy targets unless they have a runtime binary, `CURRENT_VERSION`, deploy metadata, and a service definition.
3. The Windows observer must be updated from a Windows package artifact. Linux bundles are invalid for Windows.
4. The macOS local observer must be verified with its own runtime hash or artifact lineage, not the Linux runtime hash.

## Package Scope Rules
The `Testnet Packages` workflow uses `package_scope`:

| package_scope | artifacts | valid use |
| --- | --- | --- |
| `linux_only` | `linux-x64` | ECS validators and Linux LAN observer only |
| `linux_macos_x64` | `linux-x64`, `macos-x64` | ECS/Linux plus macOS package flow |
| `all_existing` | `linux-x64`, `macos-x64`, `windows-x64` | any update that includes the Windows observer |

## Update / Catch-Up SOP
1. Capture preflight truth for every managed node: `CURRENT_VERSION`, runtime hash or artifact lineage, service manager state, status endpoint, height, peer id, and current errors.
2. Download the CI artifacts for the selected `package_scope`; verify `BUILDINFO` and `SHA256SUMS`.
3. Update the validator pair first. Confirm both validators report fresh peer heads, aligned heights, and readiness before touching observer state.
4. Update observers one at a time. For each observer: stop service, back up state, replace runtime/package metadata, reset or reseed only when required, restart, and verify.
5. If a validator reports `execution driver peer mismatch`, recover the validator pair first. Do not seed observers from a pre-recovery validator state.
6. If observers were seeded before validator recovery, reseed those observers again from recovered storage/sequencer state.
7. Finish with a five-node health snapshot; do not use one ready node as a proxy for fleet readiness.

## Reseed Triggers
Reseed is required or strongly recommended when:

1. The node is beyond the retained checkpoint/gap-sync recovery window.
2. Status shows `execution driver peer mismatch`, `BlobNotFound`, long-running `no connected providers`, or persistent `consensus_peer_head_unavailable`.
3. Validator pair was reset or rebuilt and observers still hold old-chain state.
4. A long-stopped LAN/Windows/local observer is too stale to catch up automatically.

Package replacement alone does not require reseed. Reseed only after backup and only when status evidence shows the node cannot converge cleanly.

## Required Health Fields
Every deployment or catch-up report must record at least:

1. `node_id`
2. `running`
3. `last_error`
4. `readiness.status`
5. `readiness.failed_gates`
6. `consensus.committed_height`
7. `consensus.network_committed_height` or `consensus.network_head.height`
8. `consensus.last_execution_height`
9. `consensus.network_head.decision`
10. `replication.connected_peers`
11. `replication.local_peer_id`
12. `observability.alerts` and raw recent replication error counters as diagnostics

Raw recent replication errors are diagnostic noise unless they also coincide with failed readiness gates, height non-convergence, or sustained transport degradation.
