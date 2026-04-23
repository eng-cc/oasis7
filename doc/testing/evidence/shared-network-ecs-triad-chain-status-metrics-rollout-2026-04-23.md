# Shared Network ECS Triad Chain Status Metrics Rollout (2026-04-23)

审计轮次: 1

## Meta
- 责任角色:
  - `qa_engineer`
- 协作角色:
  - `runtime_engineer`
- 当前结论:
  - `pass_candidate`
- world:
  - `shared-devnet-ecs-v1`
- related task:
  - `task_c2def8d52baa4fe5a1b1df64e19a6305`
- runtime commit:
  - `8e605366`
- runtime sha256:
  - `6c4f3a772b7f2de029919b3684828613d1ebbac0f5716b6c899adbb37501d14e`
- deployed release dir:
  - `8e605366-chain-status-metrics-20260423-095327`
- rollout source branch:
  - `task/world-runtime-chain-monitoring-gap-first-batch`
- triad snapshot summary:
  - `.tmp/p2p_real_env_triad_metrics_20260423/20260423-102200/summary.json`
- traffic summary:
  - `.tmp/p2p_real_env_traffic_monitor_20260423/latest_summary.json`
- inventory baseline:
  - `doc/testing/evidence/shared-network-ecs-triad-node-inventory-2026-03-30.md`
- prior rollout baseline:
  - `doc/testing/evidence/shared-network-ecs-triad-upgrade-2026-04-07.md`
- 最终快照时间:
  - `2026-04-23 10:22:25 CST`

## 本轮目标
1. 把 `chain-status-transaction-and-finality-metrics` 对 `/v1/chain/status` 的新增字段真正部署到 `1` 本机 observer + `2` 阿里云 ECS 三节点。
2. 用 same-window triad snapshot 重新确认 rollout 后三节点都能返回健康状态，并且 observer 不再停留在早前的 `network_committed_height=0 / known_peer_heads=0` 旧结论。
3. 额外冻结最近 `10` 分钟 traffic window，让新增链状态指标与现网复制流量证据处于同一批可审计材料里。

## 环境边界
1. 本轮真实环境仍是 `1` 本机 observer + `2` 个阿里云 ECS 节点，不是独立 sentry/NAT lab，也不等同于 `shared_access` 已关闭。
2. 本轮只证明三节点已经运行同一版带新 metrics contract 的 `oasis7_chain_runtime`，并留下 same-window snapshot 与 traffic summary；不单独证明 transfer submit/确认链路在当前窗口内发生过真实样本。
3. 早前一次临时 instant curl 曾读到 observer 旧状态；该结论已被 2026-04-23 10:22 CST 的 formal triad snapshot 覆盖，本文以后者为唯一真值。

## 部署摘要
1. 从 `task/world-runtime-chain-monitoring-gap-first-batch` 的 `HEAD=8e605366` 构建 `target/release/oasis7_chain_runtime`，并确认 `sha256=6c4f3a772b7f2de029919b3684828613d1ebbac0f5716b6c899adbb37501d14e`。
2. 将本机 `triad-observer-local` 的 `current` 切到 `/opt/oasis7/p2p-triad-local/releases/8e605366-chain-status-metrics-20260423-095327`，并确认 `oasis7-triad-observer.service=active`。
3. 将两台 ECS 的 `current` 切到 `/opt/oasis7/p2p-triad/releases/8e605366-chain-status-metrics-20260423-095327`，并确认 `oasis7-triad-sequencer.service=active`、`oasis7-triad-storage.service=active`。

## same-window triad 快照
### 本机 `triad-observer-local`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=56011 -> 56013`
  - `network_committed_height=56011 -> 56013`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`
- 结论:
  - observer 已经和 cloud pair 同窗推进，早前 `peer_heads_zero / network_committed_height_zero` 不再成立。

### ECS `triad-sequencer-a`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=56011 -> 56013`
  - `network_committed_height=56011 -> 56013`
  - `known_peer_heads=0 -> 0`
  - `last_error=null`
- 结论:
  - sequencer 本地链高与 network committed height 同窗推进，且没有 stale-height / execution error 残留。

### ECS `triad-storage-b`
- service / health:
  - `active`
  - `healthz_all_ok=true`
  - `status_fetch_all_ok=true`
- 3 个样本:
  - `committed_height=56012 -> 56013`
  - `network_committed_height=56012 -> 56013`
  - `known_peer_heads=1 -> 1`
  - `last_error=null`
- 结论:
  - storage 与 observer 一样维持 `known_peer_heads=1`，且和云端最新链高保持同窗可见。

## 最近 10 分钟 traffic window
- 聚合摘要:
  - `3` 个节点都有成功样本。
  - 总 payload: `762.35 KiB`
  - 总 libp2p substream wire bytes: `1.56 MiB`
  - 推导得到的 libp2p control-plane wire bytes: `842.06 KiB`
  - lane distribution: `udp=1.43%`，`libp2p=98.57%`
- payload 分布:
  - `observer_local`: `343.72 KiB`（`45.09%`）
  - `storage_ecs`: `231.93 KiB`（`30.42%`）
  - `sequencer_ecs`: `186.70 KiB`（`24.49%`）
- Top libp2p protocols:
  - `/aw/node/replication/fetch-commit/1.0.0`: `+407.62 KiB`
  - `/aw/node/replication/fetch-blob/1.0.0`: `+320.66 KiB`
- Top control-plane kinds:
  - `kademlia.outbound_query_progressed`: `+178`
  - `transport.connection_established`: `+34`
  - `transport.connection_closed`: `+17`
- 结论:
  - 当前窗口的主要线上流量仍然是复制面 payload，而不是 UDP gossip；新增 `/v1/chain/status` 指标可以和该 traffic summary 一起解释“链高在推进，但当前没有 transfer submit”与“复制请求/控制面在消耗多少字节”。

## 新增 `/v1/chain/status` metrics contract
### 已部署字段
1. 顶层 `transactions`：
   - `tracked_records`
   - `accepted_count`
   - `pending_count`
   - `confirmed_count`
   - `failed_count`
   - `timeout_count`
   - `inflight_count`
   - `oldest_inflight_age_ms`
   - `recent_confirmation_latency.{sample_count,avg_latency_ms,max_latency_ms,p50_latency_ms,p95_latency_ms}`
2. `consensus.recent_finality_latency`：
   - `sample_count`
   - `avg_latency_ms`
   - `max_latency_ms`
   - `p50_latency_ms`
   - `p95_latency_ms`
3. `consensus.pending_proposal`：
   - `height`
   - `slot`
   - `epoch`
   - `proposer_id`
   - `opened_at_ms`
   - `age_ms`
   - `action_count`
   - `action_payload_bytes`
   - `attestation_count`
   - `approved_stake`
   - `rejected_stake`
   - `required_stake`
   - `total_stake`
   - `approval_progress_bps`
4. `consensus.pending_consensus_actions`：
   - `queued_action_count`
   - `queued_payload_bytes`
   - `reserved_requeue_action_count`
   - `reserved_requeue_payload_bytes`
   - `available_capacity`
   - `max_capacity`
   - `submit_buffer_action_count`
   - `submit_buffer_payload_bytes`
   - `submit_buffer_max_capacity`

### live 状态抽样
#### 本机 `triad-observer-local`
- `/v1/chain/status` 抽样:
  - `committed_height=56052`
  - `network_committed_height=56052`
  - `known_peer_heads=1`
  - `transactions.*=0`，`recent_confirmation_latency.sample_count=0`
  - `recent_finality_latency.sample_count=0`
  - `pending_proposal=null`
  - `pending_consensus_actions.queued_action_count=0`
  - `pending_consensus_actions.submit_buffer_action_count=0`

#### ECS `triad-sequencer-a`
- `/v1/chain/status` 抽样:
  - `committed_height=56051`
  - `network_committed_height=56051`
  - `known_peer_heads=0`
  - `transactions.*=0`，`recent_confirmation_latency.sample_count=0`
  - `recent_finality_latency.sample_count=128`，`avg/max/p50/p95=0`
  - `pending_proposal=null`
  - `pending_consensus_actions.queued_action_count=0`
  - `pending_consensus_actions.submit_buffer_action_count=0`

#### ECS `triad-storage-b`
- `/v1/chain/status` 抽样:
  - `committed_height=56051`
  - `network_committed_height=56051`
  - `known_peer_heads=1`
  - `transactions.*=0`，`recent_confirmation_latency.sample_count=0`
  - `recent_finality_latency.sample_count=0`
  - `pending_proposal=null`
  - `pending_consensus_actions.queued_action_count=0`
  - `pending_consensus_actions.submit_buffer_action_count=0`

### 指标解读
1. `transactions.*=0` 是本轮窗口没有真实 transfer submit/confirm 样本，不是字段缺失；字段已经在三节点 live contract 中可直接返回。
2. `pending_proposal=null` 与 `pending_consensus_actions.*=0` 说明本轮抽样时没有未收敛 proposal、也没有 queue / submit buffer 积压。
3. `recent_finality_latency` 已在 contract 中上线；本轮只有 `sequencer` 已积累 `128` 个样本，observer / storage 当前仍为 `0`，这符合三节点角色差异与当前窗口行为。

## 结论
1. 三节点已经统一运行 `8e605366` 版本，且 `current` 符号链接、systemd 状态、same-window triad snapshot 都已冻结到同一份 evidence。
2. 这次 rollout 后，observer 在正式快照中已经恢复为 `known_peer_heads=1` 且 `network_committed_height` 跟随推进，因此不能再沿用旧的“observer 仍卡在 0”口径。
3. 新增 `/v1/chain/status` contract 已经在本机 observer、ECS sequencer、ECS storage 三端 live 可见；当前窗口虽然没有 transfer submit 流量，但已经足以区分“字段不存在”和“字段存在但值为 0/null”。

## 风险与后续
1. 本轮没有主动提交 transfer，因此 `transactions.confirmed_count > 0` 与 `recent_confirmation_latency.sample_count > 0` 还缺真实正样本，后续若要验证交易生命周期指标，应补带提交动作的 same-window 回放。
2. triad 当前只证明 `1` 本机 + `2` ECS 的已部署观测面，不能替代更强的 shared-network / multi-operator / ASN diversity 证据。
3. 若后续继续扩展 `/v1/chain/status` 字段，应沿用本文的冻结方式，同时保留 triad snapshot 与 traffic window，避免只写代码结论不写 live rollout 证据。
