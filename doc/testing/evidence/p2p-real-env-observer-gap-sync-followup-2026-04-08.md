# P2P Real-Env Observer Gap-Sync Follow-Up (2026-04-08)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `qa_engineer`
- 当前结论:
  - `partial`
- claim status:
  - `partial_with_gap_sync_blob_blocker`
- world:
  - `shared-devnet-ecs-v1`
- related task:
  - `task_91949e5879a8434db3a55d3012b76950`
- local run dir:
  - `.tmp/p2p_real_env_triad/20260408-102126`

## 本轮目标
1. 在不重新声明 shared-network `pass` 的前提下，确认当前本机 observer 是否仍复现 `known_peer_heads=0 / network_committed_height=0`。
2. 若该旧签名已消失，则把当前最真实、可复现的 blocker truthfully 改写成新的运行时症状。
3. 将本轮发现回写到 runtime 代码与测试，优先处理当前环境下最可验证、最可修复的路径。

## 执行命令
```bash
curl -fsS http://127.0.0.1:5633/v1/chain/status

./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 2 \
  --interval-secs 3 \
  --out-dir .tmp/p2p_real_env_triad

env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_gap_sync_ -- --nocapture
env -u RUSTC_WRAPPER cargo check -p oasis7_node
```

## 本机 observer 现状
- service:
  - `oasis7-triad-observer.service`
  - `active`
- status sample:
  - `latest_height=3638`
  - `committed_height=3638`
  - `network_committed_height=61399`
  - `known_peer_heads=1`
  - `last_error=node replication error: gap sync height 3639 failed after 3 attempts: attempt 3/3 failed: node replication error: gap sync height 3639 blob not found for hash b1f34095bb252bfd7b0a77480a6ed7c6d2adbe0a75635cae8d2b944fecdb2dfc`

## 采样结论
1. 旧的 `observer_known_peer_heads_zero / observer_network_committed_height_zero` 签名在本机当前状态下不再成立；observer 已经能看到 peer head 和 network committed height。
2. 当前本机最稳定、最直接的 blocker 已切换成 replication gap sync 路径上的 `blob not found`。
3. 由于本轮未提供远端 ECS SSH 凭据，`scripts/p2p-real-env-triad-snapshot.sh` 只完整采到本机 observer，云端两台被 summary 标成 `cloud_pair_service_unhealthy / cloud_pair_chain_not_visible`。这只能说明“本轮没有完成 same-window triad reconfirmation”，不能倒推云端服务真实失效。

## 本轮修复
1. `oasis7_node` 的 `sync_replication_height_once` 现在会像 storage challenge gate 一样，先按 DHT provider lookup 对 `fetch-blob` 做 provider-aware 请求，而不是继续盲打任意已连接 peer。
2. 当 provider route 暂时不可用、没有 connected providers，或返回 `NetworkProtocolUnavailable` 这类可回退错误时，gap sync 现在会回退到普通 lane-aware request，而不是直接把高度判死。
3. 新增 gap-sync 定向回归，固定两条语义：
   - `runtime_network_replication_gap_sync_prefers_dht_blob_providers`
   - `runtime_network_replication_gap_sync_falls_back_after_provider_route_unavailable`

## 验证结果
1. `env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_gap_sync_ -- --nocapture` 通过。
2. `env -u RUSTC_WRAPPER cargo check -p oasis7_node` 通过。
3. 本轮只验证了“当前本机 observer 已越过 peer-head-zero 阶段，且代码已补 provider-aware gap-sync blob routing”；尚未重新获取带远端云节点状态的 same-window triad pass/partial 结论。

## 对 P2PARCH-6 的意义
1. 当前真实环境下最优先的 runtime 问题已从“observer 是否接入成功”收敛为“gap sync blob 为什么找不到 provider-served content”。
2. 这使 `P2PARCH-6` 的下一步更加具体：优先继续沿 `content_hash -> provider publish freshness -> remote fetch-blob availability` 追真值，而不是再回到 reverse gossip / basic peering 层面重复排查。
3. 在拿到带云端同窗样本的新一轮 triad evidence 前，本轮仍只能记为 `partial`，不能把它升级成 shared-network `pass` 或 full mixed-topology truth。
