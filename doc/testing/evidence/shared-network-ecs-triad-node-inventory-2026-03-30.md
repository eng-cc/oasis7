# Shared Network ECS Triad Node Inventory (2026-03-30)

审计轮次: 1

## Meta
- 责任角色:
  - `runtime_engineer`
- 协作角色:
  - `liveops_community`
- 当前结论:
  - `partial`
- world:
  - `shared-devnet-ecs-v1`
- runtime release:
  - `44f8ec7f4c59`
- runtime sha256:
  - `1204b5d66cd3b741e0de06a9bc13fb095511f7e79519f041b54c18a5ce530e5e`
- 最终快照时间:
  - `2026-03-30 21:58:15 CST`

## 变更摘要
1. 在阿里云两台 ECS 上部署同一版 `oasis7_chain_runtime`，形成 `sequencer + storage` 双节点拓扑。
2. 停掉旧机遗留 `oasis7-web-launcher.service`，避免旧链路与新链路共存。
3. 在本机部署 `observer` 节点，通过 systemd 常驻并连向云上两个 gossip 入口。
4. 修正新 ECS `storage` 节点最初误配的 `NODE_AUTO_ATTEST_FLAG=--node-no-auto-attest-all`，改为 `--node-auto-attest-all` 后重启服务。

## 节点清单
### 云上老机器
- host:
  - `39.104.204.172`
- node_id:
  - `triad-sequencer-a`
- role:
  - `sequencer`
- service:
  - `oasis7-triad-sequencer.service`
- app_root:
  - `/opt/oasis7/p2p-triad`
- status_bind:
  - `127.0.0.1:5631`
- gossip_bind:
  - `0.0.0.0:5611`
- peers:
  - `39.104.205.67:5612`
- validators:
  - `triad-sequencer-a:50`
  - `triad-storage-b:30`
- auto_attest:
  - `enabled`
- legacy_service:
  - `oasis7-web-launcher.service`
  - `disabled + inactive`

### 云上新机器
- host:
  - `39.104.205.67`
- node_id:
  - `triad-storage-b`
- role:
  - `storage`
- service:
  - `oasis7-triad-storage.service`
- app_root:
  - `/opt/oasis7/p2p-triad`
- status_bind:
  - `127.0.0.1:5632`
- gossip_bind:
  - `0.0.0.0:5612`
- peers:
  - `39.104.204.172:5611`
- validators:
  - `triad-sequencer-a:50`
  - `triad-storage-b:30`
- auto_attest:
  - `enabled`
- 备注:
  - 初始部署误配为 `--node-no-auto-attest-all`，已在 2026-03-30 21:57 CST 修正并重启。

### 本机
- host:
  - `scc-ThinkBook-14-G5-IRH`
- confirmed_ip:
  - `192.168.124.8`
- node_id:
  - `triad-observer-local`
- role:
  - `observer`
- service:
  - `oasis7-triad-observer.service`
- app_root:
  - `/opt/oasis7/p2p-triad-local`
- status_bind:
  - `127.0.0.1:5633`
- gossip_bind:
  - `0.0.0.0:5613`
- peers:
  - `39.104.204.172:5611`
  - `39.104.205.67:5612`
- validators:
  - `triad-sequencer-a:50`
  - `triad-storage-b:30`
- auto_attest:
  - `disabled`
- 备注:
  - 当前仅作为 side observer 接入，不纳入正式 validator set。

## 验证快照
### 云上老机器 `triad-sequencer-a`
- service:
  - `enabled`
  - `active`
- healthz:
  - `{"ok":true}`
- `/v1/chain/status`:
  - `observed_at_unix_ms=1774879074102`
  - `slot=85`
  - `latest_height=85`
  - `committed_height=85`
  - `network_committed_height=85`
  - `last_error=null`

### 云上新机器 `triad-storage-b`
- service:
  - `enabled`
  - `active`
- healthz:
  - `{"ok":true}`
- `/v1/chain/status`:
  - `observed_at_unix_ms=1774879073882`
  - `slot=84`
  - `latest_height=6`
  - `committed_height=6`
  - `network_committed_height=6`
  - `last_error=null`
- 状态解释:
  - 该节点在修正 auto-attest 配置后已重新开始出块/提交，但仍处于重启后的追高阶段，尚未追平 `sequencer` 的 `height=85`。

## 修正后复核
- 2026-03-30 约 22:03 CST 再次检查 `triad-storage-b`：
  - `slot=111`
  - `latest_height=33`
  - `committed_height=33`
  - `network_committed_height=33`
  - `last_error=null`
- 结论:
  - `storage` 节点在修正 auto-attest 之后持续追高，说明服务并未卡死；后续只需继续观察其是否最终追平云上主链高度。

## 追高问题记录
- 2026-03-30 22:06:58 CST 到 22:08:01 CST 连续采样：
  - `triad-sequencer-a`:
    - `committed_height=130 -> 132 -> 134 -> 136`
  - `triad-storage-b`:
    - `committed_height=52 -> 54 -> 55 -> 57`
  - 两边 `last_error` 均为 `null`
- 当前判断:
  - `storage` 节点仍在持续追高，但追赶速度低于 `sequencer` 的出块推进速度，因此高度差没有明显收敛。
- 后续建议:
  - 将该问题单独作为“`storage` catch-up 速度偏慢”排查项处理，重点看同步/回放路径、资源限制和链数据补齐策略。

### 本机 `triad-observer-local`
- service:
  - `enabled`
  - `active`
- healthz:
  - `{"ok":true}`
- `/v1/chain/status`:
  - `observed_at_unix_ms=1774879072760`
  - `slot=15`
  - `latest_height=1`
  - `committed_height=0`
  - `network_committed_height=0`
  - `last_error=null`
- 状态解释:
  - 本机已收到远端时钟/链头信号，但截至快照时仍未追上云上正式链高。

## 当前结论
- 云上双节点部署已经落地，旧机器遗留 launcher 已停用，新版 runtime 已由 systemd 托管。
- 正式 validator set 仍以云上两节点为准，本机只作为观察者附着。
- 本轮三节点记录已经冻结到本文档与 `doc/devlog/2026-03-30.md`。

## 风险与后续
1. 本机只确认到私网地址 `192.168.124.8`，公网可达性未验证，因此暂不提升为正式 validator。
2. 新 ECS `storage` 节点刚完成 auto-attest 修正，虽然已经恢复持续追高，但目前仍存在 catch-up 速度慢于 `sequencer` 出块速度的问题，需后续单独排查。
3. 若后续要把本机升级为正式第三验证节点，需先确认公网入站或可验证的 P2P 可达方案，再把两台云机静态 peer/validator 清单升级为三节点版本。
