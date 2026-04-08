# P2P Real-Environment Triad Blob Availability Root Cause (2026-04-08)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 当前结论:
  - `blocked`
- claim status:
  - `blocked`
- world:
  - `shared-devnet-ecs-v1`
- related task:
  - `task_fe194839357843bb813face2c42bb218`
- baseline evidence:
  - `doc/testing/evidence/p2p-real-env-triad-stale-height-rollout-2026-04-08.md`
- local code state:
  - `HEAD=95ae1e3e` + uncommitted fix in current worktree

## 本轮目标
1. 追清 `storage challenge gate network threshold unmet` 与 observer `gap sync ... blob not found` 是否属于同一条 blob-availability 根因。
2. 确认 storage 是“没收到 replication”“收到后被拒绝”，还是“共识高度前进但 replication 从未回补”。
3. 在仓库内补上能覆盖该死锁的 runtime regression test 和修复。

## 真实环境取证
### 版本与运行态
1. ECS `triad-sequencer-a` 与 `triad-storage-b` 当前都运行 `/opt/oasis7/p2p-triad/current/bin/oasis7_chain_runtime`，`sha256=72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`。
2. 本机 `triad-observer-local` 仍停在 `/opt/oasis7/p2p-triad-local/releases/89860f6eb6d5-observer-seed-signer-20260407-223911`，`sha256=004aaf7529a4c1e26be5150aaf87ac4b648e241f29295b2dc23824d516ea4785`；本轮尚未解决本机 root-owned 目录升级路径。

### storage 侧现场
1. `triad-storage-b` 的 replication root `/opt/oasis7/p2p-triad/output/node-distfs/triad-storage-b` 当前只有 `node_pos_state.json`，没有 `replication_guard.json`、`replication_writer_state_*`、`replication_commit_messages`、`store/files_index.json` 或 replication blobs。
2. `find /opt/oasis7/p2p-triad/output/node-distfs/triad-storage-b -maxdepth 4 -printf '%y %p\n'` 只返回:
   - `d /opt/oasis7/p2p-triad/output/node-distfs/triad-storage-b`
   - `f /opt/oasis7/p2p-triad/output/node-distfs/triad-storage-b/node_pos_state.json`
3. `curl http://127.0.0.1:5632/v1/chain/status` 显示 storage 当前 `committed_height=62984`、`network_committed_height=62984`、`known_peer_heads=1`，但 `storage.bytes_by_dir.replication_root=572`、`storage.blob_counts.replication_blobs=0`。
4. 这说明“高度已追上”不等于 replication 已持久化；storage 的共识高度可以单独前进，而 replication root 仍是空的。

### sequencer 侧现场
1. `triad-sequencer-a` 的 `last_error` 稳定收敛为:
   - `storage challenge gate network threshold unmet`
   - 样本 hash 包含 `2e78104f...`、`d739c1c9...`、`0a5f27fa...`
2. sequencer 本地 replication 产物存在且完整：
   - `replication_guard.json`
   - `replication_writer_state_triad-sequencer-a.json`
   - `replication_commit_messages/*`
3. 例如 `replication_commit_messages/00000000000000057535.json` 的 writer 真值为:
   - `writer_id=4e15e631d5d76029c502b4a36ec119670be72cb4fe342dd607ec9dce1ecc3cea`
   - `writer_epoch=1774878050664`
   - `sequence=53450`

### observer 侧现场
1. observer 当前稳定停在:
   - `known_peer_heads=1`
   - `network_committed_height>0`
   - `last_error=gap sync height ... blob not found`
2. 这说明 observer 主问题也不再是 peering/reverse path，而是同一条 blob availability residual。

## 代码根因
### 关键事实
1. `NodeRole::Storage` 不会本地产生 replication commit；`replicate_local_commits` 只对 `NodeRole::Sequencer` 打开。
2. `PosNodeEngine::apply_decision(...)` 会直接推进 `committed_height`，即使 replication 从未成功持久化。
3. `known_peer_heads` / `network_committed_height` 可以由 consensus commit 消息推进，不足以证明 replication topic 或 blob/store 已落盘。
4. sequencer 的 `broadcast_local_replication(...)` 在发布新 replication 前先执行 `enforce_storage_challenge_gate(...)`。

### 失效链路
1. storage 一旦错过历史 replication 流，但共识仍继续把 `committed_height` 推高，本地 replication root 仍可能保持空目录。
2. 旧实现把 `ingest_network_replications(...)` 的 apply 判定和 `sync_missing_replication_commits(...)` 的 gap-sync 起点都绑在 `committed_height` 上。
3. 结果是 storage/observer 只要“共识高度已追平”，runtime 就认为没有 replication gap，即便 replication 持久化高度仍是 `0`。
4. sequencer 同时因为 storage 没有 blob，继续卡在 `storage challenge gate`，从而停止发布新的 replication。
5. 这形成了一个真实环境死锁：
   - storage 没有历史 blob
   - runtime 不会主动按 replication 持久化游标回补
   - sequencer 又因为 storage 没有 blob 而停止继续发布

## 本轮修复
1. `oasis7_node` 新增独立的 replication 持久化游标 `replication_persisted_height`，不再复用 `committed_height`。
2. `ingest_network_replications(...)` 现在按 replication 已持久化高度决定“哪些高度允许直接 apply”。
3. `sync_missing_replication_commits(...)` 现在按 replication 已持久化高度起算 gap sync，即使本地 `committed_height` 已经更高，仍会继续回补缺失的 commit/blob。
4. `ReplicationRuntime` 新增 `latest_persisted_commit_height(...)`，统一从 hot commit mirror + cold index 读取当前已落盘高度。
5. `request_fetch_blob_with_route_fallback(...)` 已覆盖 provider route `found=false` / `NetworkProtocolUnavailable` 两类场景，provider-aware `fetch-blob` 失败时会继续尝试后续可用路由。
6. `enforce_storage_challenge_gate(...)` 新增 cold-start catch-up fallback：仍优先抽样最新 local blob，但若最新样本只是“远端暂时不可达/未补齐”，会沿 `storage_challenge_fallback_height` 退回到较旧的顺序样本，只要达到 network threshold 就允许 sequencer 继续发布 replication。

## 本地验证
已通过:

```bash
env -u RUSTC_WRAPPER cargo fmt --all
env -u RUSTC_WRAPPER cargo test -p oasis7_node replication_gap_sync_backfills
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_syncs_distfs_commit_files
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_gap_sync_falls_back_after_provider_route_not_found
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_network_replication_gap_sync_falls_back_after_provider_route_unavailable
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_falls_back_after_provider_route_not_found
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_falls_back_after_provider_route_unavailable
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_allows_when_network_matches_reach_threshold
env -u RUSTC_WRAPPER cargo test -p oasis7_node runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup
```

新增 regression:
- `tests::replication_gap_sync_backfills_when_consensus_height_already_advanced`
  - 复现 storage `committed_height=3`、`network_committed_height=3`、replication root 为空时，旧逻辑会直接跳过 gap sync 的问题。
  - 修复后该用例会把 `1..3` 的 commit/blob 从 fetch-commit/fetch-blob 路径完整补回。
- `tests::network_gap_sync_tests::runtime_network_replication_gap_sync_falls_back_after_provider_route_not_found`
  - 复现 provider-aware `fetch-blob` 命中 `found=false` 后旧逻辑不会继续尝试普通路由，observer 会卡在单个缺 blob peer 的问题。
- `tests::runtime_replication_storage_challenge_gate_falls_back_after_provider_route_not_found`
  - 复现 sequencer `storage challenge gate` 命中 provider route `found=false` 后不再换路由重试的缺口。
- `tests::runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup`
  - 直接验证 sequencer 在最新 challenge 样本远端不可达时，会继续探测更旧的已回补 blob，并在达到 threshold 后放行，不再被“最新 hash 还没补到”永久卡死。

## 2026-04-08 二次 rollout 结果
### 版本
1. 本轮把 `HEAD=95ae1e3ee604` 当前 worktree 的新 release `95ae1e3ee604-blob-route-fallback-20260408` 部署到本机 observer 与两台 ECS。
2. 三节点当前统一运行:
   - release: `95ae1e3ee604-blob-route-fallback-20260408`
   - sha256: `0179d52afb91355821dcfbeb94c83c7bb10eb174fe1d81d41fbf16d27b26329a`

### same-window 观察
1. 本机 `triad-observer-local` 已明显恢复：
   - `committed_height` 在 rollout 后继续从 `16187` 推进到 `16237`
   - `last_error=null`
   - 旧 `gap sync ... blob not found` 主签名已消失
2. ECS `triad-storage-b` 继续从空 replication root 向前回补：
   - rollout 后复采窗口内 `replication_commit_messages` / `store/blobs` 已增长到 `1056 / 1056`
   - `replication_root` 继续增大，说明回补链路仍在工作
3. ECS `triad-sequencer-a` 的失败签名发生收敛：
   - 旧 `storage challenge gate ... NetworkProtocolUnavailable { protocol: "/aw/node/replication/fetch-blob/1.0.0" }` 已不再出现
   - 当前只剩 `storage challenge gate network blob not found`

### 当前 residual
1. 这次二修已经证明“provider route 命中 `found=false` 就直接停”的问题真实存在，且修复后 observer 路径恢复。
2. 当前 triad 仍未完全恢复，因为 sequencer 的 challenge gate 仍抽样最新 blob，而 storage 仍按历史高度从 `1` 起顺序回补，短窗口内尚未补到这些最新 hash。
3. storage 侧 residual 也从“blob 路由选错 peer / 协议不可达 + blob 不可达”收敛成以 `fetch-commit` 间歇 `NetworkProtocolUnavailable` 为主的回补抖动；但该抖动当前不会阻止它继续累积 commit/blob 文件。

## 2026-04-08 三次 rollout 结果
### 版本
1. 在 challenge-gate fallback 修复落地后，重新构建 release `95ae1e3ee604-challenge-gate-fallback-20260408`。
2. 本轮三节点统一运行:
   - release: `95ae1e3ee604-challenge-gate-fallback-20260408`
   - sha256: `a2cb5191cdb58cfa0b430369e0220666b5d18e22f4cf58b5b0d1a220f1370fea`

### same-window 观察
1. 本机 `triad-observer-local` 保持恢复态并继续前进：
   - rollout 后首采 `committed_height=17062`
   - 20 秒窗口复采 `committed_height=17124`
   - 60 秒窗口复采 `committed_height=17187`
   - `known_peer_heads=1`
   - `last_error=null`
2. ECS `triad-storage-b` 继续顺序回补 replication backlog，但 `fetch-commit` 抖动仍在：
   - rollout 后首采 `replication_commit_messages/store-blobs = 2003/2003`
   - 20 秒窗口复采 `2193/2193`
   - 60 秒窗口复采 `2347/2347`
   - 同窗口 `last_error` 从 `gap sync height 1962 ... fetch-commit NetworkProtocolUnavailable` 漂移到 `gap sync height 2300 ... fetch-commit NetworkProtocolUnavailable`
   - 这说明 storage 仍在稳定累积历史 commit/blob，但远端 `fetch-commit` 可用性不足，导致它重启后还未重新收敛到正常共识高度
3. ECS `triad-sequencer-a` 的 challenge gate 语义在真实环境窗口内已出现解锁信号：
   - rollout 后首采 `committed_height=57536`
   - 60 秒窗口复采 `committed_height=57538`
   - `last_error=null`
   - 采样窗口内未再看到旧的 `storage challenge gate network blob not found`
4. 这说明 challenge-gate fallback 至少已经把“storage 只补到较旧高度时，sequencer 完全停住不再出块”的死锁打破；sequencer 在 storage 冷启动回补期间重新出现了前进能力。

### 当前 residual 更新
1. 当前 triad 的首要 blocker 已从 `storage challenge gate network blob not found` 进一步转移到 storage 侧 `fetch-commit NetworkProtocolUnavailable` 与 peer/head 重收敛缓慢。
2. sequencer 当前仍只有 `known_peer_heads=0`，说明三节点还没有恢复到稳定互见的完整 healthy topology；但它已经不再被 challenge gate 明确报错阻断。
3. triad 因 storage restart 后仍长期停在 `committed_height=0 / network_committed_height=0` 而继续维持 `blocked`，不能宣称真实三节点 fully recovered。

## 结论
1. 当前 real-env triad 的 `sequencer storage-challenge blob not found` 与 observer `gap sync blob not found` 已可归并到同一条 root cause：replication 持久化游标错误地复用了 `committed_height`。
2. 这不是单纯的 signer/allowlist 或 fetch-route 问题；storage 在真实环境下可能“共识高度正常，但 replication 根目录为空”，旧实现无法从这种状态自愈。
3. rollout 后可确认 observer 侧 real-env blocker 已解除，storage 也开始持续回补，因此本专题不再是“完全无进展”的 blocked。
4. 第三次 rollout 说明 challenge-gate catch-up fallback 已在真实环境窗口内产生正向效果：sequencer 不再报 `storage challenge gate`，并恢复了小幅出块前进。
5. 但 storage 侧 `fetch-commit NetworkProtocolUnavailable` 仍让它重启后停在 `committed_height=0 / network_committed_height=0`，因此 triad 级 verdict 仍只能维持 `blocked`，不能宣称真实三节点已恢复。

## 2026-04-08 路由/候选残留收口
### 新增实机诊断
1. `chain/status` 现已补充 replication debug payload：`local_peer_id`、`connected_peers`、`peer_healths`、`registered_protocols`、`unsupported_protocol_peers`、`recent_errors`。
2. 借此确认 triad peer 对应关系：
   - observer=`12D3KooWF7hkwdUYKSnmLzuWsvqgtSuAcpZfq6xKkstJV1oaMqjF`
   - storage=`12D3KooWG1GJTjZe9PSCcLY74quNxjoSv6yqFRxuJPF48pbvnbGu`
   - sequencer=`12D3KooWHpsWPJscCUtRPj6mk42eBhTeTcQwaxH45nQwimfuAmud`
3. diagnostics 同时确认 sequencer/storage 两端本地都已注册：
   - `/aw/node/replication/fetch-commit/1.0.0`
   - `/aw/node/replication/fetch-blob/1.0.0`
4. 因此当前 residual 不再是“sequencer 未挂 fetch handler”，而是 triad 启动窗口中的可用复制源选择与连接就绪时序。

### route sanitize rollout
1. 新 release `95ae1e3ee604-inbound-endpoint-route-sanitize-20260408`（`sha256=b84b551e087a1e2b47dde4d8d62a71fc0100cc3980d94379e633d1c53657a6e6`）把 listener 侧 `send_back_addr` 临时源端口从 Kademlia 路由学习链路中移除，同时让 active transport path 优先映射回 peer record 中的稳定 direct addr。
2. rollout 后 same-window 复采确认：
   - sequencer 与 storage 能重新恢复彼此直连，不再长期只剩 observer
   - storage 一度把 `last_error` 清成 `null`
3. 但 60 秒窗口后 storage 仍可能回落到 `fetch-commit NetworkProtocolUnavailable`，说明仅修正 route pollution 还不足以彻底消除早期错误请求路径。

### unsupported-peer-no-fallback rollout
1. 新 release `95ae1e3ee604-unsupported-peer-no-fallback-20260408`（`sha256=98b1b99878ba271af63ba4d5e72be1d6a42073e84a383d6d2012a30fb2e3c2de`）进一步收口 request path：若某 peer 已被 runtime 记为当前 protocol 的 `unsupported_protocol_peer`，后续不再把它重新作为兜底候选。
2. rollout 后 storage 的主失败签名发生了关键变化：
   - 旧签名：`fetch-commit NetworkProtocolUnavailable`
   - 新签名：`libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`
3. 这说明 runtime 已不再把 observer 误当成 `fetch-commit` 复制源；剩余问题已收敛为“启动窗口里虽然 observer + sequencer 都在 connected peers 中，但可用于 `fetch-commit` 的 healthy source 尚未就绪/未被选入”。

### 当前最终结论
1. 本轮已经修平两类真实残留：
   - inbound 临时端口污染导致的错误 transport route 学习
   - 已知 unsupported peer 仍被重复拿来做 replication request fallback
2. triad 当前 blocker 因此进一步收敛成 startup ordering / retry window 问题：
   - storage 重启后早期 gap-sync 窗口里，真正可服务 `fetch-commit` 的 peer 尚未 ready
   - runtime 现在会正确拒绝 observer fallback，于是错误呈现为 `no connected peers`
3. 这比之前的 `NetworkProtocolUnavailable` 更接近真值，也给后续修复指明了更窄的方向：应该继续追 `fetch-commit` 启动窗口的重试、head refresh、provider/source readiness 时序，而不是继续排查 observer lane capability、handler 注册或 transport route truth。

## 下一步
1. 单独复核 storage `fetch-commit` 间歇 `NetworkProtocolUnavailable` 的连接/peer 选择残留，确认它只是 bootstrap 期抖动，还是还需要给 `fetch-commit` 也补同类 provider-aware route retry / retryable classification。
2. 复采 sequencer `known_peer_heads=0` 的成因，确认它是 storage 冷启动期间的暂态，还是 restart 后 peer-head 传播本身还有残留阻断。
3. 下一轮复采仍要重点盯:
   - observer `committed_height` 是否继续单调推进且保持 `last_error=null`
   - storage `replication_commit_messages` / `store/blobs` 是否持续增长
   - sequencer 是否继续维持 `last_error=null` 且 `committed_height` 单调增长，而不是重新退回 `storage challenge gate`
