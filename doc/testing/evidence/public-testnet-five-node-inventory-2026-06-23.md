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
| `triad-testnet-fourth-local` | observer | local macOS arm64 | no | `macos-arm64` | `$OASIS7_TESTNET_FOURTH_ROOT` | concrete launchd target `system/oasis7.testnet.fourth` or `gui/<uid>/oasis7.testnet.fourth` | `http://127.0.0.1:19083/v1/chain/status` |

`$OASIS7_TESTNET_FOURTH_ROOT` is the operator-local macOS observer root used by the current deployment. The concrete local home path belongs in task execution evidence, not in reusable docs.

## Inventory Boundary
1. The managed fleet is five nodes: two ECS validators, the Linux LAN observer, the Windows observer, and the local macOS observer.
2. Old bootstrap staging directories such as `.tmp/testnet-local-node-bootstrap` and `.tmp/testnet-fourth-node-bootstrap` are not deploy targets unless they have a runtime binary, `CURRENT_VERSION`, deploy metadata, and a service definition.
3. The Windows observer must be updated from a Windows package artifact. Linux bundles are invalid for Windows.
4. The macOS local observer must use a checksummed `macos-arm64` DMG and verify native `aarch64-apple-darwin` identity where the host supplies `lipo` or `file`; it must never be accepted from the Linux runtime hash.

## Package Scope Rules
The `Testnet Packages` workflow uses `package_scope`:

| package_scope | artifacts | valid use |
| --- | --- | --- |
| `linux_only` | `linux-x64` | ECS validators and Linux LAN observer only |
| `linux_macos_x64` | `linux-x64`, `macos-x64` | x64 macOS fleet only; not valid for the current arm64 observer |
| `linux_macos_arm64` | `linux-x64`, `macos-arm64` | Linux fleet plus the current Apple Silicon observer |
| `all_existing` | `linux-x64`, `macos-x64`, `windows-x64` | existing x64/Windows contract; it does not include `macos-arm64` |

Full managed-fleet packaging therefore requires coordinated `all_existing` plus `linux_macos_arm64` runs against the same requested ref/commit. Operators must verify each run's BUILDINFO/SHA256SUMS and record the two run IDs; one `all_existing` run is not evidence that the arm64 observer was packaged.

## Deployment-Closure Health Contract
Use `p2p-public-testnet-fleet-health.py --managed-five-node` for this managed-fleet deployment closure. The preset requires exactly these logical identities, once each: `sequencer` (ECS 204 provider), `storage` (ECS 205 provider), `linux-lan-observer`, `windows-observer`, and `macos-observer`. It rejects omissions, duplicates, renamed identities, and unknown nodes before it writes health evidence. The collector's generic mode is for explicitly non-closure diagnostics only and cannot support a managed-fleet healthy conclusion.

Managed closure additionally requires both canonical providers to expose `chain_proof.latest_execution_checkpoint` with `schema_version >= 2`, a non-empty `checkpoint_id`, a 64-hex `manifest_hash`, and positive `height`. The two providers must report the same checkpoint ID and manifest hash, with an absolute height delta no greater than one. Missing/null/v1/malformed/mismatched/incompatible provider evidence blocks the closure; generic collector output is not a substitute.

## Update / Catch-Up SOP
1. Capture preflight truth for every managed node: `CURRENT_VERSION`, runtime hash or artifact lineage, service manager state, status endpoint, height, peer id, and current errors.
2. Download the CI artifacts for the selected `package_scope`; verify `BUILDINFO` and `SHA256SUMS`.
3. Update the validator pair first. Confirm both validators report fresh peer heads, aligned heights, readiness, and the compatible v2 provider checkpoint contract above before touching observer state.
4. Generate the rollout plan with canonical provider `status_url` entries. Every non-provider Linux, Windows, and macOS observer execution path runs the same provider checkpoint gate immediately before its mutation command; do not invoke an underlying package primitive directly. A failed gate means repair/redeploy the provider pair or wait for validated checkpoint publication, then regenerate/re-run the observer plan—never bypass it with a restart or a copied validator state.
5. Update observers one at a time. For the macOS arm64 observer, preflight the package/host/launchd target and back up runtime/config/provenance first; require the launchd plist `Label` to equal the label in its concrete target; stop the original service; then create and verify the persistent-state backup while it is stopped. State roots must be unique, non-nested, physically resolved directories strictly below the physically resolved node root with no symlink component, and must not overlap runtime, config, deployment provenance, or launchd plist paths. Binary or provenance mutation is forbidden until both backup phases pass; every bootstrap must re-confirm the exact launchd target before endpoint health acceptance.
6. Generate the macOS operator script through `p2p-public-testnet-package-rollout.py`. A bootout error or stopped-state backup failure must restart the untouched original service and verify `/healthz` reports `{"ok":true}` while `/v1/chain/status` reports `{"running":true}`. Status remains the authority-failure inspection surface. Any post-mutation failure must first copy the failed state under the attempt root, then replace active state with the stopped pre-upgrade snapshot, restore runtime/config/provenance, bootstrap the validated launchd domain, and verify those endpoint-specific fields. Authority failure always emits state-sync escalation, including when rollback fails.
7. If a validator reports `execution driver peer mismatch`, recover the validator pair first. Do not seed observers from a pre-recovery validator state.
8. If observers were seeded before validator recovery, reseed those observers again from recovered storage/sequencer state.
9. Finish with a five-node health snapshot using `--managed-five-node`; do not use one ready node as a proxy for fleet readiness.

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
