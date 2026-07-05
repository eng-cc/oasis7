# Public Testnet Node Deploy Evidence - 2026-07-05

Task: `task_951dfee0cd644772b52fbe84320883eb`

## Scope

- User request: sync/rebase `main`, then deploy public testnet nodes.
- Worktree HEAD after main sync: `767e7d2a6125073f5c2dce20479a2322075345a5`.
- GitHub Actions package run: `28731071030`.
- Package artifact: `testnet-package-linux-x64-0.0.0+testnet.138.767e7d2a6125`.
- Package version: `0.0.0+testnet.138.767e7d2a6125`.
- Runtime SHA-256: `87305d8bc6885f194a101c46e6e77884d3d0e97b1103ee298813c9adac68788c`.

## Local Package Verification

Commands:

```bash
gh workflow run testnet-packages.yml --ref main \
  -f ref_or_sha=767e7d2a6125073f5c2dce20479a2322075345a5 \
  -f build_profile=release \
  -f package_scope=linux_only

gh run watch 28731071030 --interval 30 --exit-status

gh api repos/eng-cc/oasis7/actions/artifacts/8088855231/zip \
  > .tmp/public-testnet-node-deploy-20260705/artifacts/testnet-package-linux-x64-0.0.0+testnet.138.767e7d2a6125.zip

unzip -o -q \
  .tmp/public-testnet-node-deploy-20260705/artifacts/testnet-package-linux-x64-0.0.0+testnet.138.767e7d2a6125.zip \
  -d .tmp/public-testnet-node-deploy-20260705/artifacts/linux-package-unzipped

sha256sum -c linux-x64-SHA256SUMS
```

Result:

- Testnet Packages run `28731071030` completed successfully for commit `767e7d2a6125073f5c2dce20479a2322075345a5`.
- `linux-x64-BUILDINFO` recorded `package_version=0.0.0+testnet.138.767e7d2a6125`.
- `sha256sum -c linux-x64-SHA256SUMS` returned OK for all package outputs, including `oasis7-linux-x64-bundle.tar.gz`.
- Local bundle SHA-256: `3b53542e40fd28fda11ebd24054dce4c9f4965cae38882d9f9f321f2fdee9829`.

## Deployment Targets

| Role | Host | Service | Status URL |
| --- | --- | --- | --- |
| sequencer | `root@39.104.204.172` | `oasis7-triad-sequencer.service` | `http://39.104.204.172:6631/v1/chain/status` |
| storage | `root@39.104.205.67` | `oasis7-triad-storage.service` | `http://39.104.205.67:6632/v1/chain/status` |

Credential handling:

- The operator-supplied Aliyun ECS credential file was used only as an SSH credential source.
- Secret material was not written to this evidence.

## Upgrade Commands

Each host received:

- `/tmp/oasis7-package-28731071030/oasis7-linux-x64-bundle.tar.gz`
- `/tmp/oasis7-package-28731071030/p2p-public-testnet-package-node-upgrade.sh`
- `/tmp/oasis7-package-28731071030/public-testnet-governed-bootstrap-manifest-2026-06-06.json`

The remote bundle SHA matched the local bundle SHA on both ECS validators:

```text
3b53542e40fd28fda11ebd24054dce4c9f4965cae38882d9f9f321f2fdee9829  oasis7-linux-x64-bundle.tar.gz
```

Sequencer command shape:

```bash
bash /tmp/oasis7-package-28731071030/p2p-public-testnet-package-node-upgrade.sh \
  --node-root /opt/oasis7/p2p-testnet \
  --bundle-tar /tmp/oasis7-package-28731071030/oasis7-linux-x64-bundle.tar.gz \
  --package-version 0.0.0+testnet.138.767e7d2a6125 \
  --commit 767e7d2a6125073f5c2dce20479a2322075345a5 \
  --run-id 28731071030 \
  --artifact-ref testnet-package-linux-x64-0.0.0+testnet.138.767e7d2a6125/oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime \
  --systemd-service oasis7-triad-sequencer.service \
  --restart-service \
  --post-restart-status-url http://127.0.0.1:6631/v1/chain/status \
  --post-restart-timeout-secs 120
```

Storage command was identical except for `--systemd-service oasis7-triad-storage.service` and `--post-restart-status-url http://127.0.0.1:6632/v1/chain/status`.

## Final Deployment Truth

Final public status capture:

- `sequencer`: `.tmp/public-testnet-node-deploy-20260705/final-verify/sequencer-status.json`
- `storage`: `.tmp/public-testnet-node-deploy-20260705/final-verify/storage-status.json`
- captured at: `2026-07-05T06:29:02Z`

Final SSH truth capture:

- `sequencer`: `.tmp/public-testnet-node-deploy-20260705/final-ssh-truth/sequencer-ssh-truth.txt`
- `storage`: `.tmp/public-testnet-node-deploy-20260705/final-ssh-truth/storage-ssh-truth.txt`
- captured at: `2026-07-05T06:38:54Z` and `2026-07-05T06:38:55Z`

SSH truth command shape:

```bash
ssh <target> 'set -e; hostname; date -u; systemctl is-active <service>; \
  cat /opt/oasis7/p2p-testnet/CURRENT_VERSION; \
  readlink -f /opt/oasis7/p2p-testnet/current; \
  cat /opt/oasis7/p2p-testnet/DEPLOYED_BUILDINFO; \
  sha256sum /opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime; \
  inspect governed bootstrap bundle and manifest JSON under /opt/oasis7/p2p-testnet/config'
```

Per-host final SSH truth:

| Role | Hostname | Service | Active | Current version | Current release | Runtime SHA-256 |
| --- | --- | --- | --- | --- | --- | --- |
| sequencer | `iZhp3a7zk3e3ur69iv1wvgZ` | `oasis7-triad-sequencer.service` | `active` | `0.0.0+testnet.138.767e7d2a6125` | `/opt/oasis7/p2p-testnet/releases/0.0.0+testnet.138.767e7d2a6125` | `87305d8bc6885f194a101c46e6e77884d3d0e97b1103ee298813c9adac68788c` |
| storage | `iZhp34imf4xt7nxrf5v5h6Z` | `oasis7-triad-storage.service` | `active` | `0.0.0+testnet.138.767e7d2a6125` | `/opt/oasis7/p2p-testnet/releases/0.0.0+testnet.138.767e7d2a6125` | `87305d8bc6885f194a101c46e6e77884d3d0e97b1103ee298813c9adac68788c` |

Per-host `DEPLOYED_BUILDINFO` and governed bundle metadata both report:

- `run_id=28731071030`
- `commit=767e7d2a6125073f5c2dce20479a2322075345a5`
- `package_version=0.0.0+testnet.138.767e7d2a6125`
- `runtime_sha256=87305d8bc6885f194a101c46e6e77884d3d0e97b1103ee298813c9adac68788c`
- `runtime_size=102995208`
- `runtime_build.ref=testnet-package-linux-x64-0.0.0+testnet.138.767e7d2a6125/oasis7-linux-x64-bundle.tar.gz!/bin/oasis7_chain_runtime`
- `runtime_build.path=/opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime`
- `runtime_build.resolved_path=/opt/oasis7/p2p-testnet/current/bin/oasis7_chain_runtime`

Per-host governed bootstrap manifest truth:

- `network_id=oasis7-public-testnet-governed-20260606`
- `chain_id=oasis7-public-testnet-governed-20260606`
- `tier=public_testnet`
- `status=rehearsal`
- `promotion_policy.required_gates`: current 11-lane set

Final public status:

| Role | running | last_error | readiness.status | live required gates | network_id | chain_id | local_peer_id | connected peers |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| sequencer | `true` | `null` | `ready` | 11 | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `12D3KooWMyPapumCaTABq27umWdHqXDr8AoTse21eMVnXeJEsbNp` | 4 |
| storage | `true` | `null` | `ready` | 11 | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `12D3KooWAuNCCEDu7CdUUDwALuAhuLekZHgVWxAYp4Ag5ti79fJj` | 4 |

Final chain/head fields:

| Role | committed_height | network_committed_height | last_block_hash | last_execution_height | last_execution_block_hash | last_execution_state_root | network_head.height | network_head.block_hash |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| sequencer | `0` | `0` | `genesis` | `0` | `null` | `null` | `null` | `null` |
| storage | `0` | `0` | `genesis` | `0` | `null` | `null` | `null` | `null` |

Final world resource fields:

| Role | world_id | chain_id | seed_manifest_hash | committed_chunk_count | last_delta_commit_height | readiness_status | failed_gates |
| --- | --- | --- | --- | --- | --- | --- | --- |
| sequencer | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `1825dfd676b7bc65529ec82cb609af379fa8947a1236848336895e0bafa74377` | `2` | `1` | `not_ready` | `world_resource_world_id_mismatch`, `world_resource_chain_id_mismatch`, `world_resource_delta_commit_hash_missing`, `world_resource_delta_height_mismatch` |
| storage | `oasis7-public-testnet-governed-20260606` | `oasis7-public-testnet-governed-20260606` | `1825dfd676b7bc65529ec82cb609af379fa8947a1236848336895e0bafa74377` | `2` | `1` | `not_ready` | `world_resource_world_id_mismatch`, `world_resource_chain_id_mismatch`, `world_resource_delta_commit_hash_missing`, `world_resource_delta_height_mismatch` |

The live 11 required lanes are:

```text
public_rpc_ready
explorer_public_ready
faucet_guard_ready
reset_policy_announced
runtime_bootstrap
claims_boundary_review
world_resource_provenance_ready
provider_resource_provenance_ready
resource_delta_replay_ready
api_viewer_projection_ready
same_world_hosted_entry_ready
```

## Remaining Blockers

This deployment closes the ECS validator package/runtime drift and live 7-lane manifest drift. It does not make `public_testnet` ready for live-candidate claims.

Remaining observed blockers:

- consensus is still at `committed_height=0`, `network_committed_height=0`, `last_execution_height=0`
- head/execution fields remain unadvanced: `last_block_hash=genesis`, `last_execution_block_hash=null`, `last_execution_state_root=null`, `network_head.height=null`, `network_head.block_hash=null`
- `world_resource.readiness_status=not_ready`
- `world_resource.failed_gates`:
  - `world_resource_world_id_mismatch`
  - `world_resource_chain_id_mismatch`
  - `world_resource_delta_commit_hash_missing`
  - `world_resource_delta_height_mismatch`
- faucet endpoint was connection-refused during preflight and remains a separate deployment lane
- LAN and Windows observer SSH were not reachable from the current network during preflight

## Script Fix Included

During deployment verification, `scripts/p2p-public-testnet-package-node-upgrade.sh` was found to rewrite `runtime_build.git_commit` and `runtime_build.sha256`, but not `runtime_build.package_version` or `runtime_build.run_id`.

The script now writes both fields, and `scripts/p2p-public-testnet-package-node-upgrade.test.sh` asserts them.

Verification:

```bash
bash scripts/p2p-public-testnet-package-node-upgrade.test.sh
```

Result:

```text
ok: package node upgrade pins current runtime hash into governed bootstrap bundle
```
