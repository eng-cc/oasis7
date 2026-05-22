# Public Testnet Local Observer Contract Sync (2026-05-22)

审计轮次: 2

## Summary
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - repo-owned local observer sync path: `pass`
  - live apply on this machine: `pass_with_followup_blocker`
- 关联现网:
  - local service: `oasis7-testnet-observer.service`
  - local stack root: `/opt/oasis7/p2p-testnet-local`
  - ECS contract sources:
    - `.tmp/p2p_testnet_reality/20260522-100229/nodes/sequencer_ecs/node.env`
    - `.tmp/p2p_testnet_reality/20260522-100229/nodes/storage_ecs/node.env`

## Why this exists
1. `public-testnet-ecs-freshness-audit-2026-05-22.md` 已证明本机 observer 仍停在旧的三 validator / 三 signer 合同，且未加载 `--network-tier-manifest`。
2. 2026-05-19 的 live endpoint deploy 只把两台 ECS 收口到 formal manifest，未把本机 `/opt/oasis7/p2p-testnet-local` 纳入同一条 repo-owned deploy 路径。
3. 因此本轮新增 `scripts/p2p-public-testnet-local-observer-sync.sh`，把“如何把 local observer 收敛到 current two-validator ECS contract”固定成单一脚本入口。

## Script contract
1. `render`:
  - 输入：`local node.env` + `sequencer/storage ECS node.env` + target `NETWORK_TIER_MANIFEST_PATH`
  - 输出：保留本机 `STATUS_BIND`、`PLAYER_ENTRY_*`、本地 data 路径与 `NODE_ID`，但把以下字段替换为 ECS two-validator 真值：
    - `NODE_VALIDATORS_CSV`
    - `NODE_VALIDATOR_SIGNERS_CSV`
    - `NODE_GOSSIP_PEERS_CSV`
    - `REPLICATION_NETWORK_BOOTSTRAP_PEERS_CSV`
    - `REPLICATION_REMOTE_WRITERS_CSV`
    - `POS_*` / reward / world timing 合同
    - `NETWORK_TIER_MANIFEST_PATH`
2. `apply`:
  - 在可写目标机上直接回写 `node.env`
  - 可选复制 live manifest
  - 可选安装 repo-owned `scripts/p2p-triad-node-start.sh`
  - 会先备份旧 `node.env` / manifest / start script`

## Verified command
1. Render against the fresh 2026-05-22 snapshot:
```bash
./scripts/p2p-public-testnet-local-observer-sync.sh render \
  --local-env .tmp/p2p_testnet_reality/20260522-100229/nodes/local_node/node.env \
  --sequencer-env .tmp/p2p_testnet_reality/20260522-100229/nodes/sequencer_ecs/node.env \
  --storage-env .tmp/p2p_testnet_reality/20260522-100229/nodes/storage_ecs/node.env \
  --manifest-path /opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json
```
2. Observed key output:
  - `NODE_VALIDATORS_CSV=triad-testnet-sequencer:100,triad-testnet-storage:100`
  - `NODE_GOSSIP_PEERS_CSV=39.104.204.172:6731,39.104.205.67:6732`
  - `REPLICATION_REMOTE_WRITERS_CSV=triad-testnet-sequencer:... ,triad-testnet-storage:...`
  - `NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all`
  - `PLAYER_ENTRY_ENABLE=1`
  - `NETWORK_TIER_MANIFEST_PATH=/opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json`
3. Offline apply verification also passed in a temp app root:
  - rewritten `node.env` created
  - `network-tier-public-testnet-live-candidate.json` copied into temp `config/`
  - repo-owned `start-node.sh` installed into temp `bin/`
  - backup copies written into temp `backups/`

## 2026-05-22 live apply
1. 本机会话随后拿到本机 sudo 权限，并已实际执行：
```bash
./scripts/p2p-public-testnet-local-observer-sync.sh apply \
  --local-env /opt/oasis7/p2p-testnet-local/config/node.env \
  --sequencer-env .tmp/p2p_testnet_reality/20260522-100229/nodes/sequencer_ecs/node.env \
  --storage-env .tmp/p2p_testnet_reality/20260522-100229/nodes/storage_ecs/node.env \
  --manifest-path /opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json \
  --manifest-source doc/testing/evidence/public-testnet-live-candidate-manifest-2026-05-22.json
systemctl restart oasis7-testnet-observer.service
```
2. live apply 期间脚本又补了两项必要修正：
  - manifest 安装时会把 `release_candidate_bundle_ref`、`genesis_ref`、`bootstrap_peer_ref` 本地化到 `/opt/oasis7/p2p-testnet-local/config/`
  - `REPLICATION_REMOTE_WRITERS_CSV` 改为复用 ECS `node.env` 的纯 hex allowlist
3. 当前 live status 已确认：
  - `systemctl is-active oasis7-testnet-observer.service` -> `active`
  - `network_tier.source_path=/opt/oasis7/p2p-testnet-local/config/network-tier-public-testnet-live-candidate.json`
  - `network_tier.tier=public_testnet`
  - `network_tier.bootstrap_peer_count=2`
4. 这证明本机 observer 的 formal manifest / two-validator contract 已经真实生效，而不是停留在离线 render。

## 2026-05-22 operator follow-up
1. 为了把“旧 observer 残留执行状态”也纳入 repo-owned 修复路径，脚本新增了：
```bash
./scripts/p2p-public-testnet-local-observer-sync.sh reset-state \
  --local-env /opt/oasis7/p2p-testnet-local/config/node.env
```
2. `reset-state` 会解析 `node.env` 中带 `$STACK_ROOT/...` 的实际路径，并备份后清空：
  - `EXECUTION_WORLD_DIR`
  - `EXECUTION_RECORDS_DIR`
  - `STACK_ROOT/output/node-distfs/$NODE_ID`
  - `STACK_ROOT/output/chain-runtime/$NODE_ID/reward-runtime-execution-bridge-state.json`
3. 该命令已在本机多次真实执行，备份目录写入：
  - `/opt/oasis7/p2p-testnet-local/backups/local-observer-state-reset-*`
4. 在此之后，两类较早期 blocker 已经不再是当前主阻断：
  - `fetch-commit authorization failed`
  - `replication writer switch must start at sequence 1`
5. 随后又核对并对齐了 runtime binary：
  - local current binary `sha256=2f836980834da470882fef4ca7ab0598c984acfc42565d574acf2cd19c474cfe`
  - ECS sequencer current binary `sha256=2f836980834da470882fef4ca7ab0598c984acfc42565d574acf2cd19c474cfe`
  - ECS storage current binary `sha256=2f836980834da470882fef4ca7ab0598c984acfc42565d574acf2cd19c474cfe`
6. 即使 binary 已与两台 ECS 对齐，本机仍在 height 15 失败：
  - runtime restart 一度先报 `execution driver restore snapshot ref ... BlobNotFound`
  - 当前 `/v1/chain/status` 持续报 `gap sync height 15 execution hash validation failed`
  - mismatch 真值：
    - `local_block=a7d0bf881a9bbb51404114ba45aab399645e3cad45371ed9e1490ed06761df74`
    - `peer_block=1ccaf35534e06f4238a50fd719eaffa2ca2fa23e841ec4b28171c0877efd7517`
    - `local_state=fd1dc428d79a813d808a21025fbe47579f8448242604975487a98305eb42ab37`
    - `peer_state=1e3b53a30f7e0bd4f464531dd716d17996c23e8d50b8cd56a6e180cd14e14717`
7. 同窗还确认 repo mirror 的 candidate bundle 仍写着另一份 runtime hash：
  - `/opt/oasis7/p2p-testnet-local/config/public-testnet-live-candidate-bundle-2026-05-22.json`
  - `runtime_build.sha256=d1046485ae71a794cf0f5fb78561bd6068363ca53aee3ccac384d831829c07e8`
8. 这说明当前剩余问题已不再是“local contract 未生效”或“binary 还没对齐”，而是更深一层的 release/runtime input drift。
9. 随后又做了一次更强的 live 复现：把本机 `STORAGE_ROOT` 也迁出并重启 observer。
  - backup: `/opt/oasis7/p2p-testnet-local/backups/storage-reset-20260522-164319`
  - 结果：local `execution_store_root` 已降到近空白状态，但 `/v1/chain/status` 仍立即回到同一条 `gap sync height 15 execution hash validation failed`
  - 这说明“仅仅是本机旧 CAS/blob 没清掉”不足以解释当前分叉，问题至少还包含上游 peer truth 或更深的执行恢复输入漂移。

## Remaining live blocker
1. local observer 现在可以加载 formal manifest、two-validator contract，并且 current runtime binary 也已与 ECS hash 对齐。
2. 当前 remaining blocker 是：
  - `shared_devnet_pass` 仍未满足
  - local observer 在 height 15 持续出现 execution hash mismatch
  - live current binary hash 与 mirrored candidate bundle `runtime_build.sha256` 仍不一致
  - 即使额外清空了本机 `STORAGE_ROOT`，同一条 height-15 mismatch 也会立即复现
3. 因此这条任务虽然已完成 local contract sync 与 repo-owned reset path，但还不能把本机 runtime 记成健康 `pass`，也不能把 aggregate `public_testnet` readiness 提升为可用。

## Boundaries
1. This task only closes the repo-owned local observer sync path.
2. It does not clear the remaining height-15 execution mismatch, nor the broader live-candidate release/runtime drift that still separates the local observer from current network execution truth.
3. Therefore even after this script lands and live apply completes, aggregate `public_testnet` readiness still remains `block`.
