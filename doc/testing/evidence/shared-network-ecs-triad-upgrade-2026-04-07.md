# Shared Network ECS Triad Upgrade (2026-04-07)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `qa_engineer`
- 当前结论:
  - `partial`
- world:
  - `shared-devnet-ecs-v1`
- upgraded release:
  - `89860f6eb6d5`
- upgraded sha256:
  - `26a41315e0bcc34cad996ef73fa9289455f004393190fb65ce05217bc8c8e1dc`
- prior local observer release:
  - `44f8ec7f4c59`
- prior cloud release:
  - `cdd0dbf3553a`
- inventory baseline:
  - `doc/testing/evidence/shared-network-ecs-triad-node-inventory-2026-03-30.md`
- 最终快照时间:
  - `2026-04-07 21:27:00 CST`

## 变更摘要
1. 在当前仓库 `HEAD=89860f6eb6d5` 上重新构建 `oasis7_chain_runtime` release binary。
2. 将本机 `triad-observer-local` 与两台阿里云 ECS 的 `current` 符号链接统一切到 `releases/89860f6eb6d5`。
3. 三个 systemd service 全部重启并复核 `sha256`、`systemctl is-active` 与本地 `/v1/chain/status`。
4. 本轮完成后，“三节点版本不一致”已经不再是 blocker；但本机 observer 仍保持 `known_peer_heads=0 / network_committed_height=0 / committed_height=0`。

## 升级后节点版本
### 本机 `triad-observer-local`
- current:
  - `/opt/oasis7/p2p-triad-local/releases/89860f6eb6d5`
- sha256:
  - `26a41315e0bcc34cad996ef73fa9289455f004393190fb65ce05217bc8c8e1dc`
- service:
  - `oasis7-triad-observer.service`
  - `active`
- `/v1/chain/status`:
  - `role=observer`
  - `slot=57497`
  - `latest_height=1`
  - `committed_height=0`
  - `network_committed_height=0`
  - `known_peer_heads=0`
  - `last_error=null`

### ECS `triad-sequencer-a`
- current:
  - `/opt/oasis7/p2p-triad/releases/89860f6eb6d5`
- sha256:
  - `26a41315e0bcc34cad996ef73fa9289455f004393190fb65ce05217bc8c8e1dc`
- service:
  - `oasis7-triad-sequencer.service`
  - `active`
- `/v1/chain/status`:
  - `role=sequencer`
  - `slot=57560`
  - `latest_height=57516`
  - `committed_height=57516`
  - `network_committed_height=57516`
  - `known_peer_heads=0`
  - `last_error=null`

### ECS `triad-storage-b`
- current:
  - `/opt/oasis7/p2p-triad/releases/89860f6eb6d5`
- sha256:
  - `26a41315e0bcc34cad996ef73fa9289455f004393190fb65ce05217bc8c8e1dc`
- service:
  - `oasis7-triad-storage.service`
  - `active`
- `/v1/chain/status`:
  - `role=storage`
  - `slot=57566`
  - `latest_height=57566`
  - `committed_height=57566`
  - `network_committed_height=57566`
  - `known_peer_heads=1`
  - `last_error=null`

## 结论
1. 三个节点现在已经全部运行同一版 `oasis7_chain_runtime`，版本偏差已被排除。
2. 升级后 observer 仍旧没有看到 peer head，也没有拿到 network committed height，因此当前 real-env blocker 仍然存在。
3. 这说明 `P2PARCH-6` 当前剩余问题已经更清晰地收敛为“observer 接入/可达性/peering 路径问题”，而不是“版本不一致导致的未知行为差异”。

## 对后续排障的意义
1. 后续若继续看到 `observer_known_peer_heads_zero`，可以优先排查 peer wiring、入站/出站可达性、gossip 配置或运行态同步逻辑，而不必再把版本 skew 当成首要怀疑对象。
2. 这轮升级只解决版本一致性，不代表 mixed-topology real-run 已通过；`P2PARCH-6` 仍不能宣称 `pass`。
