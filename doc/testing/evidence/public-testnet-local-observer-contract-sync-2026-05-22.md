# Public Testnet Local Observer Contract Sync (2026-05-22)

审计轮次: 1

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

## Remaining live blocker
1. local observer 仍未追平链高，当前最新错误是：
  - `fetch-commit authorization failed: requester_public_key_hex=8971af...`
2. 这说明当前 remaining blocker 已从“本机未加载 manifest / 合同漂移”切换成“ECS fetch-commit 授权面不接受本机 requester key”。
3. 因此这条任务虽然已完成 local contract sync，但还不能把本机 runtime 记成健康 `pass`。

## Boundaries
1. This task only closes the repo-owned local observer sync path.
2. It does not clear the new local fetch-commit authorization blocker, nor the separate ECS sequencer blocker `execution driver missing predecessor record for non-contiguous committed height`.
3. Therefore even after this script lands and live apply completes, aggregate `public_testnet` readiness still remains `block`.
