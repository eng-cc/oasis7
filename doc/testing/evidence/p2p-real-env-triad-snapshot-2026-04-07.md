# P2P Real-Environment Triad Snapshot (2026-04-07)

审计轮次: 1

## Meta
- 责任角色:
  - `qa_engineer`
- 协作角色:
  - `runtime_engineer`
- 当前结论:
  - `partial`
- claim status:
  - `partial_with_observer_blocker`
- world:
  - `shared-devnet-ecs-v1`
- snapshot run dir:
  - `.tmp/p2p_real_env_triad/20260407-205218`
- snapshot summary:
  - `.tmp/p2p_real_env_triad/20260407-205218/summary.json`
- inventory baseline:
  - `doc/testing/evidence/shared-network-ecs-triad-node-inventory-2026-03-30.md`
- 最终快照时间:
  - `2026-04-07 20:52:50 CST`

## 环境边界
1. 本轮真实环境仍是 `1` 个本机 observer + `2` 个阿里云 ECS 节点，不是 dedicated sentry/NAT lab，也不是 NAT/CGNAT 全覆盖真值。
2. 云上两台 ECS 继续承担正式 validator set；本机 `triad-observer-local` 只作为 observer 接入，不纳入 validator set。
3. 本轮目标是把 `P2PARCH-6` 从“只知道有这组真机”推进成“已经真实采到 mixed-topology triad 的 live samples 和 failure signatures”，不是宣称 full truth 已通过。

## 执行命令
```bash
P2PARCH6_SEQ_SSH_PASSWORD='***' \
P2PARCH6_STORAGE_SSH_PASSWORD='***' \
./scripts/p2p-real-env-triad-snapshot.sh \
  --samples 4 \
  --interval-secs 5 \
  --out-dir .tmp/p2p_real_env_triad
```

## 节点清单
### 本机 observer
- node_id:
  - `triad-observer-local`
- role:
  - `observer`
- service:
  - `oasis7-triad-observer.service`
- status_bind:
  - `127.0.0.1:5633`
- gossip peers:
  - `39.104.204.172:5611`
  - `39.104.205.67:5612`

### ECS sequencer
- host:
  - `39.104.204.172`
- node_id:
  - `triad-sequencer-a`
- role:
  - `sequencer`
- service:
  - `oasis7-triad-sequencer.service`
- status_bind:
  - `127.0.0.1:5631`
- gossip peers:
  - `39.104.205.67:5612`

### ECS storage
- host:
  - `39.104.205.67`
- node_id:
  - `triad-storage-b`
- role:
  - `storage`
- service:
  - `oasis7-triad-storage.service`
- status_bind:
  - `127.0.0.1:5632`
- gossip peers:
  - `39.104.204.172:5611`

## 采样摘要
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `slot=57275 -> 57277`
  - `latest_height=1 -> 1`
  - `committed_height=0 -> 0`
  - `network_committed_height=0 -> 0`
  - `known_peer_heads=0 -> 0`
  - `last_error=null`
- 结论:
  - observer 进程存活，但在本轮窗口内没有看到任何 peer head，也没有看到 network committed height，无法算作 mixed-topology 接入已打通。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `latest_height=57344`
  - `committed_height=57344`
  - `network_committed_height=57344`
  - `known_peer_heads=0`
  - `last_error=null`
- 结论:
  - sequencer 本机链高可见且稳定，没有 crash 或直接 HTTP 失败信号；本轮窗口内未见新的 committed height 增长，但不影响确认云上主链仍在高位运行。

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 4 个样本:
  - `latest_height=57357 -> 57359`
  - `committed_height=57357 -> 57359`
  - `network_committed_height=57357 -> 57359`
  - `known_peer_heads=1`
  - `last_error=null`
- 结论:
  - storage 在本轮窗口内继续推进链高，说明云上双节点至少保留了一侧可观测的 live progress signal。

## Failure Signatures
- `observer_known_peer_heads_zero`
- `observer_network_committed_height_zero`
- `observer_committed_height_not_advancing`

## 诊断结论
1. 这组真实环境已经足够给 `P2PARCH-6` 留下第一批 real-run 证据，因为它证明了“云上链高可见 + 本机 observer 接入失败”这组 mixed-topology live signature 可以被稳定复现。
2. 本轮不能把结果写成 `pass` 或 `full truth ready`。唯一合理口径是：`partial_with_observer_blocker`。
3. 基于本轮抓取到的 `node.env`，云上两台 ECS 只把彼此写进静态 `NODE_GOSSIP_PEERS_CSV`，本机 observer 则单向把两台 ECS 写成 peer。这里可以推断当前 real env 仍偏向“云上双节点正式链 + 本机附着观察”，而不是完成了对称的三节点 mixed-topology peering。

## 对 P2PARCH-6 的意义
1. `P2PARCH-6` 不再只依赖本地 proxy longrun；现在已经有真实 `1` 本机 + `2` ECS triad 的 live snapshot、执行脚本和 failure signatures。
2. 当前 real-run 证明的 blocker 已经从“有没有真机”收敛成“observer 为何持续 `known_peer_heads=0 / network_committed_height=0`”。这比“没有真实环境”更具体，也更接近下一步排障入口。
3. 在修平 observer blocker 之前，不能把这组环境拿去冒充 `shared-network pass` 或 `mixed_topology full truth`；它只足够支撑 `P2PARCH-6` 的 real-env baseline 进入 audited partial。

## 后续建议
1. 下一轮优先排查 observer 与云上节点之间是否需要显式双向静态 peer、额外公开入站、或其它 gossip 可达性条件，而不是继续重复本地 proxy 近似。
2. 若要把这组真实环境进一步升级为更强证据，至少要看到 observer 样本里 `known_peer_heads > 0` 且 `network_committed_height > 0`，并在连续窗口内留到 committed height 推进。
3. 即便 observer 接通，这组环境仍然只覆盖“本机 + cloud public”一类 mixed-topology；NAT 类型、CGNAT、独立 operator/ASN、多 sentry/dedicated lab 仍需额外证据。
