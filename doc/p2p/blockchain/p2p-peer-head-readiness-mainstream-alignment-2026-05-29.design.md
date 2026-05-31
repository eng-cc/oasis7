# oasis7 peer-head readiness 主流公链对齐方案（设计文档）

- 对应需求文档: 待补
- 对应项目管理文档: 待补

审计轮次: 1

## 背景
2026-05-28 testnet 三节点运行中出现过短时高度视图断层：两个 ECS 节点已到同一 head，本地 observer 一度只看到自己的较低 network head。后续三端 block hash 重新对齐，根因不是持久链分叉，而是本地 NAT/private observer 与 ECS 之间的 libp2p 连接抖动导致 peer head 可见性短时丢失。

已补的 P0 行为是：`known_peer_heads=0` 时不再把本地 committed height 伪装成可信 network head，并在 `/v1/chain/status` 中暴露 `ready=false`、`network_head_available=false`、`consensus_peer_head_unavailable`。

本方案定义下一阶段“一步到位”的目标态，使 oasis7 的节点健康模型接近主流公链常见处理方式：liveness、readiness、sync、peer visibility、head quorum、head freshness、hash/root 一致性和网络可达性分别建模。

## 设计目标
- 拆清 `local head`、`peer advertised head`、`quorum network head`、`finalized/committed head`、`sync target` 的语义。
- 用 peer-head quorum 与 freshness TTL 取代单纯的 `known_peer_heads > 0`。
- 把 block hash / execution block hash / state root 一致性纳入 readiness 判定，避免“高度相同但内容不同”的假健康。
- 把连接抖动、protocol errors、NAT/private reachability 与 peer scoring 联动到 readiness。
- 保持 `/v1/chain/status` 向后兼容，同时新增机器可判定字段，给 liveops、viewer、release gate 和 alerting 共用。

## 非目标
- 本方案不把当前 testnet 直接声明为主流公链级成熟。
- 本方案不引入第二套共识协议，也不改变 PoS commit 规则。
- 本方案不要求当前三节点 testnet 具备 permissionless validator admission。

## 主流公链对齐矩阵
| 能力 | 主流公链常见做法 | oasis7 当前 | 目标态 |
| --- | --- | --- | --- |
| Liveness/readiness 分离 | RPC 存活和可服务状态分开 | 已实现 `liveness.status`、`readiness.status`、`sync.status` 顶层对象 | 保持独立 API，不再把进程存活等同可服务 |
| Network head | 来自 peer status / gossip / finalized checkpoint | 已实现 `consensus.network_head` source/decision/quorum/freshness/stake 字段 | readiness 层已能区分 count quorum、stake quorum 与 stake fallback |
| Peer head quorum | 多 peer head 聚合，按 stake/role/score 权重 | 已按 network tier manifest、role 和 validator stake snapshot 计算 quorum | mainnet validator 使用 stake-weighted readiness quorum；stake 映射缺失时不得假绿 |
| Freshness | peer head 过期后不参与 sync target | 已按 tier/role policy 输出并使用 TTL | `readiness.policy.peer_head_ttl_ms` 机器可读 |
| Hash/root 一致性 | 同高度 head 必须 hash/root 一致，否则 fork/degraded | 已按 height/hash/execution hash/state root 分 bucket，同高度冲突 critical | slash 执行仍需治理/共识专项承接 |
| Peer scoring | timeout/reconnect/protocol failure 降分 | libp2p request peer selection 已按 score 排序并暴露 `request_peer_scores` | readiness quorum 仍可继续接 peer score 过滤 |
| NAT/reachability | private/NAT/relay 状态影响可服务性 | 已按 observer/validator-like role 接 readiness policy | mainnet validator 使用 `public_direct_or_governed_relay` readiness policy |
| Sync state | catching_up/synced/unknown 明确 | 已实现 `sync.status=synced/catching_up/stalled/stale_peer_view/unknown/conflicting` | stalled sync 已按 tier policy 判定 |

## API 目标模型
在现有 `/v1/chain/status` 保持旧字段的基础上扩展以下结构。

### 顶层
| 字段 | 含义 |
| --- | --- |
| `ok` | readiness 是否可服务。只对运维/客户端入口有效，不再等同于进程存活 |
| `running` | liveness 层进程/worker 是否运行 |
| `liveness.status` | `ok/critical`，描述 runtime 是否活着 |
| `readiness.status` | `ready/not_ready`，描述节点是否可作为当前角色服务 |
| `readiness.failed_gates[]` | readiness 失败 gate code 列表，直接来自 observability alerts |
| `readiness.policy` | 当前 tier/role 的 TTL、lag、stall、quorum、relay、slashing readiness 策略 |
| `sync.status` | `synced/catching_up/stalled/stale_peer_view/unknown/conflicting` |
| `observability.ready` | readiness 布尔结果 |
| `observability.status` | `ok/warn/critical` 汇总状态 |

### `consensus.network_head`
```json
{
  "source": "peer_quorum | peer_single | self_only | unknown",
  "height": 123,
  "block_hash": "...",
  "execution_block_hash": "...",
  "execution_state_root": "...",
  "observed_peer_count": 2,
  "required_peer_count": 2,
  "quorum_mode": "count | stake_weighted | count_fallback_stake_unavailable",
  "fresh_peer_count": 2,
  "stale_peer_count": 0,
  "conflicting_peer_count": 0,
  "observed_stake": 74,
  "required_stake": 67,
  "total_stake": 100,
  "stake_quorum_met": true,
  "freshness_ttl_ms": 15000,
  "decision": "ready | degraded | critical"
}
```

### `consensus.peer_heads[]`
每个 peer head 必须从“调试信息”升级成 readiness 输入：
```json
{
  "node_id": "triad-testnet-sequencer",
  "peer_id": "12D3...",
  "height": 123,
  "block_hash": "...",
  "execution_block_hash": "...",
  "execution_state_root": "...",
  "committed_at_ms": 1779972002853,
  "observed_at_ms": 1779972003000,
  "age_ms": 147,
  "fresh": true,
  "score": 92,
  "role": "validator_core"
}
```

### `observability.alerts`
新增稳定 code：
| Code | Severity | 条件 |
| --- | --- | --- |
| `consensus_peer_head_unavailable` | warn/critical | 没有 fresh peer head |
| `consensus_peer_head_quorum_missing` | warn/critical | fresh peer head 数低于 tier/role 阈值 |
| `consensus_peer_head_stale` | warn | 有 peer head 但超过 TTL |
| `consensus_peer_head_conflict` | critical | 同高度 hash/root 冲突或同 peer head 回退异常 |
| `consensus_stake_quorum_missing` | critical | mainnet stake-weighted peer head quorum 未达标 |
| `consensus_stake_quorum_unavailable` | critical | mainnet validator 无 validator stake snapshot，只能 count fallback |
| `consensus_network_lag` | warn/critical | quorum network head 高于本地 committed |
| `consensus_sync_stalled` | critical | lag 超过 policy 且 last commit age 超过 stalled 窗口 |
| `replication_transport_unstable` | warn/critical | 连接关闭/重连/timeout 速率超过窗口阈值 |
| `p2p_reachability_degraded` | warn | 节点角色要求 public/direct，但当前 unknown/private/relay 不达标 |
| `mainnet_slashing_evidence_only` | warn | mainnet slashing 当前只作为 readiness evidence，不执行协议惩罚 |

## Readiness 判定
Readiness 由分层 gate 组成。任一 critical gate 失败，`ok=false`。

| Gate | 输入 | Ready 条件 | 失败状态 |
| --- | --- | --- | --- |
| `LIVENESS` | process/RPC/running | runtime running 且 status endpoint 可响应 | critical |
| `PEER_CONNECTIVITY` | connected/active peer count | 满足 tier 最小 active peers | warn/critical |
| `PEER_HEAD_QUORUM` | fresh peer heads | `fresh_peer_count >= required_peer_count` | warn/critical |
| `STAKE_HEAD_QUORUM` | validator stake snapshot + peer head validator_id | mainnet validator `observed_stake >= required_stake` | critical |
| `HEAD_CONSISTENCY` | height/hash/root buckets | quorum bucket 唯一且无 critical conflict | critical |
| `SYNC_LAG` | local committed vs quorum head | lag <= tier 阈值 | warn/critical |
| `SYNC_STALLED` | lag + last commit age | lag 未消除且 last commit age 未超过 stalled 窗口 | critical |
| `REPLICATION_STATE` | persisted height/gap blocked | 无 blocked gap，state gap 为 0 | critical |
| `TRANSPORT_STABILITY` | error rate/reconnect rate | error/reconnect rate 低于窗口阈值 | warn/critical |
| `REACHABILITY` | NAT/relay/autonat/public port | 满足当前 role 的 reachability policy | warn |

## Quorum 策略
不同网络 tier 使用不同策略，避免 dev/test/mainnet 混用同一阈值。

| Tier | Role | required fresh peer heads | TTL | Lag 阈值 | 说明 |
| --- | --- | --- | --- | --- | --- |
| `local_devnet` | single/local | 0 | n/a | 0 | 单节点允许 self-only |
| `shared_devnet` | observer | 1 | 15000ms | 2 heights | 内部集成可容忍短暂追赶 |
| `public_testnet` | observer | 1 | 10000ms | 1 height | 必须能看到至少一个 validator head |
| `public_testnet` | validator_core | 2 或 `min(n-1,2)` | 10000ms | 0-1 height | 三节点时要求看到另外两个或按配置降级 |
| `mainnet` | observer | 2 | 5000ms | 1 height | 观察节点需要稳定 quorum view |
| `mainnet` | validator_core | governance/stake-weighted quorum | 5000ms | 0 height | 使用 NodeConsensusSnapshot validator stake truth；缺失时 critical fallback |

三节点 testnet 初期可配置为：
- observer: required=1，缺失则 `ready=false`。
- validator_core: required=1 作为 transitional，升级目标 required=2。
- 当 head hash/root conflict 时，无论 required 数是否满足，都 critical。

## Head 聚合算法
1. 读取本地 `committed_height/hash/execution_hash/state_root`。
2. 读取 peer head map，过滤 stale head。
3. 对 fresh heads 按 `(height, block_hash, execution_block_hash, execution_state_root)` 分 bucket。
4. 选择最高高度、最高权重 bucket 作为 candidate network head。
5. 如果同高度存在多个 hash/root bucket，标记 `consensus_peer_head_conflict=critical`。
6. 如果 candidate 高度低于本地高度，只作为 peer view，不降低本地 committed truth；`network_head.source` 可为 `peer_quorum_behind_local`。
7. 如果没有 fresh peer head：
   - replication disabled/local tier: `source=self_only`。
   - replication enabled/shared/public/mainnet: `source=unknown`，`ready=false`。
8. mainnet validator 下，如果 peer head 可映射到 `validator_id` 与 `validator_stakes`，按 observed stake 判定 quorum；如果 stake truth 缺失，输出 `count_fallback_stake_unavailable` 并 `ready=false`。

## Peer Score 与 Transport 稳定性
peer score 初始 100，窗口内事件扣分：
| 事件 | 扣分 | TTL |
| --- | --- | --- |
| `ConnectionClosed` 非正常高频 | -5 | 60s |
| outbound request timeout | -10 | 60s |
| `InsufficientPeers` | network-level penalty | 30s |
| protocol unsupported/mismatch | -25 | 5m |
| head stale | -10 | TTL 窗口 |
| hash/root conflict | block/critical | manual or epoch reset |

分数影响：
- `<70`: candidate，不计入 quorum，但可作为 fallback fetch peer。
- `<40`: suspect，不计入 readiness。
- `<20`: blocked/cooldown。

现有 `protocol_retry_cooldown_peers`、`transport_retry_cooldown_peers` 继续保留，但需要把结果反馈到 readiness 和 peer selection。

## NAT 与节点角色
| Role | Reachability 要求 |
| --- | --- |
| observer/private | 可 outbound 连接，允许 private/unknown，但必须有 stable persistent peers |
| validator_core/public_testnet | 推荐 public/direct；若 private/NAT，必须显式 persistent peer + relay/bootnode fallback |
| mainnet validator_core | public/direct 或正式 relay policy；unknown reachability 不可 ready |

对本次 testnet 的直接影响：
- 本地 NAT observer 可作为 observer，但不能作为 validator_core readiness 样本。
- 如果 `observed_public_addr=null` 且 `relay_available=false`，需要要求 persistent peer 稳定性或降级 role。
- mainnet validator 的 relay-only readiness 要求 governed relay redundancy；单 relay 或 unknown public reachability 不可 ready。

## 落地顺序
### Phase 0: 已完成的 P0 修正
- `known_peer_heads=0` 暴露 `consensus_peer_head_unavailable`。
- `observability.ready=false`。
- 顶层 `ok=false`。

### Phase 1: Head freshness 与 API 扩展
- 在 `NodePeerCommittedHead` 增加 `observed_at_ms`。
- status payload 增加 `consensus.network_head`。
- 增加 stale peer head 过滤和 TTL 配置。
- 单测覆盖：fresh/stale/unknown/self-only。

### Phase 2: Quorum 与 hash/root 一致性
- 增加 `PeerHeadQuorumDecision`。
- 实现 height/hash/root bucket 聚合。
- `consensus_peer_head_conflict` critical。
- 三节点 testnet gate 要求 observer 至少 1 个 fresh peer head，validator 至少 1/2 个按 transitional config。

### Phase 3: Transport stability 与 peer score
- 对 `recent_errors` 从简单列表升级为窗口计数和分类计数。
- 将 `ConnectionClosed`、`InsufficientPeers`、timeout 速率转成 `replication_transport_unstable`。
- peer score 进入 request peer selection 和 readiness quorum 过滤。

### Phase 4: Reachability policy
- 将 p2p reachability、relay、external direct addr 与 node role policy 接线。
- NAT/private observer 使用 observer readiness policy。
- validator_core 在 public/mainnet tier 下必须满足 public/direct 或显式 relay/persistent peer policy。

### Phase 5: Gate 与 liveops
- 更新 `scripts/p2p-real-env-triad-snapshot.sh`、`scripts/p2p-real-env-observability-summary.py`。
- 新增三节点 readiness gate：
  - `peer_head_quorum_ok`
  - `head_consistency_ok`
  - `transport_stability_ok`
  - `reachability_policy_ok`
- runbook 明确“ok=false/ready=false”处理路径。

### Phase 6: Mainnet-grade readiness policy
- mainnet validator peer head quorum 使用 stake-weighted 判定。
- TTL、lag、stalled sync 窗口由 tier/role policy 输出并参与 readiness。
- mainnet validator reachability 使用 `public_direct_or_governed_relay` policy。
- slashing 在本阶段作为 evidence-only readiness gate 暴露，不执行协议惩罚。

### Phase 7: Misbehavior evidence 与 peer-head 隔离
- 对同一 validator 同高度不同 commit hash / execution root / action root 记录 `commit_equivocation` 证据。
- 将作恶 validator 加入 quarantine，移出 peer-head quorum，后续 gossip/replication head 不再计入 readiness。
- status payload 输出 evidence count、quarantined validator count、slashable stake preview。
- triad snapshot 和 observability summary 增加 `consensus_misbehavior_evidence_present`、`consensus_validator_quarantined` failure signatures。

### Phase 8: Validator stake proof chain
- 对 validator set 生成 deterministic `validator_set_hash`。
- 对 validator stake/player/signer 绑定生成 Merkle leaf、`validator_stake_root` 和 per-validator proof。
- mainnet stake-weighted readiness 要求 stake map、stake root 和 proof count 一致，否则进入 `count_fallback_stake_unavailable`。
- equivocation evidence 绑定 `validator_stake_root`，使 slash preview 能关联到具体 stake truth。

### Phase 9: Slashing intent bridge
- 将 consensus misbehavior evidence 转成治理层可执行的 slashing intent。
- intent 字段对齐 `World::apply_identity_penalty`：`target_agent_id`、`evidence_hash`、`reason`、`slash_stake`、`appeal_window_ticks`。
- readiness/status 输出 pending slashing intent，triad 和 observability summary 增加 `consensus_slashing_intent_pending`。
- 当前阶段不在 gossip ingest 内直接修改 world ledger；真正扣罚由治理提交 `apply_identity_penalty` 执行，以避免 node consensus 层绕过世界治理权限。

### Phase 10: Slashing receipt reconciliation
- consensus snapshot 增加 governance slashing receipt 结构。
- status payload 按 `evidence_hash` 匹配 intent 和 receipt。
- 有 applied receipt 的 intent 不再计入 pending slashing intent。
- triad 和 observability summary 输出 receipt/applied receipt count。

### Phase 11: Execution-world slashing receipt bridge
- `/v1/chain/status` 在构造 payload 前只读加载 execution world。
- 将 `World::governance_identity_penalties()` 中与 consensus intent `evidence_hash` 匹配的记录映射为 `slashing_receipts`。
- receipt 注入不产生副作用，不在 status 查询期间发起治理交易或直接修改 consensus 状态。
- 已应用、已申诉、申诉被拒的 governance penalty 均视为已进入治理执行链路，可清除对应 pending intent。

## 当前实现状态
截至本方案同分支实现：
- Phase 0 已完成。
- Phase 1 已完成：`NodePeerCommittedHead.observed_at_ms` 已进入 snapshot，`consensus.network_head` 已输出 source/decision/fresh/stale/conflict/quorum 字段；顶层 `liveness/readiness/sync` API 已实现。
- Phase 2 已完成：fresh peer head quorum 与同高度 hash/root conflict 已进入 readiness；required peer count 已按 `network_tier_manifest` 和 role 计算；conflict 为 critical。
- Phase 3 已完成基础版：基于 `replication.recent_errors`、protocol retry cooldown 和 transport retry cooldown 计算 `transport_stability_score`，低于阈值输出 `replication_transport_unstable` 并使 `ready=false`；libp2p replication request selection 已接入 request peer score，失败降分、成功回升、低分 peer 后置但保留为单 peer 网络的最后恢复路径。
- Phase 4 已完成 readiness 版：observer 允许 active peer/relay；validator-like role 要求 public/direct 或 relay，否则输出 `p2p_reachability_degraded` 并使 `ready=false`。
- Phase 5 已完成基础版：`p2p-real-env-observability-summary.py` 已识别 `network_head`、transport stability 和 reachability policy；`p2p-real-env-triad-snapshot.sh` 已采集 `liveness/readiness/sync`、`consensus.network_head`、transport/reachability readiness，并输出 `peer_head_quorum_not_ready`、`network_head_conflict`、`replication_transport_unstable`、`p2p_reachability_degraded`、`node_not_ready` failure signatures。
- Phase 6 已完成 readiness/gate 版：mainnet validator stake-weighted quorum、stake fallback 阻断、tier TTL/lag/stalled sync policy、mainnet governed relay readiness policy、slashing evidence-only 边界已进入 status payload、triad snapshot 和 observability summary。
- Phase 7 已完成 evidence/isolation 版：commit equivocation 会生成可归责证据、quarantine validator、从 peer-head quorum 中剔除，并通过 status payload、triad snapshot、observability summary 暴露 evidence/quarantine/slashable stake preview。
- Phase 8 已完成 proof 版：节点共识 snapshot 输出 validator set hash、stake root 和 per-validator Merkle proof；mainnet stake quorum 依赖 stake proof truth；misbehavior evidence 绑定 stake root；triad 和 observability summary 可识别 `validator_stake_proof_unavailable`。
- Phase 9 已完成 intent 版：节点共识 snapshot 输出 `slashing_intents`，每条 intent 可映射到 `apply_identity_penalty` 参数；status payload、triad 和 observability summary 均能暴露 pending slashing intent。
- Phase 10 已完成 receipt 版：节点共识 snapshot 支持 `slashing_receipts`，status payload 根据 receipt 消除 pending intent，并输出 receipt/applied receipt count。
- Phase 11 已完成 status bridge 版：chain status server 会从 execution world 的 governance identity penalty 记录自动生成 matching slashing receipt；该路径是只读对账，不绕过治理层执行扣罚。

## 测试计划
| Test | 覆盖 |
| --- | --- |
| unit: no peer heads | replication enabled 时 `ready=false` |
| unit: stale peer heads | stale 不计 quorum |
| unit: peer head quorum ok | required peer heads 达标 |
| unit: hash/root conflict | 同高度不同 hash/root critical |
| unit: local ahead of peer view | 不误判回滚，但标 peer view stale/behind |
| integration: three-node transient disconnect | 断连窗口 readiness 降级，恢复后自动回绿 |
| integration: NAT observer | private observer 允许但不作为 validator readiness |
| script: triad snapshot | 汇总报告能输出 unknown/quorum/conflict/stability |
| unit: mainnet stake quorum | stake-weighted quorum 未达标时 degraded/not ready |
| unit: mainnet stake unavailable | stake truth 缺失时 critical fallback，不允许 count quorum 假绿 |
| unit: stalled sync | mainnet lag 超过窗口输出 `consensus_sync_stalled` |
| unit: mainnet relay/slashing boundary | governed relay redundancy 与 slashing evidence-only alert |
| unit: commit equivocation | 同一 validator 同高度双签时记录证据、隔离 validator、从 peer-head quorum 剔除 |
| unit: misbehavior observability | evidence/quarantine 进入 critical alert 与 slashable stake preview |
| unit: validator stake proof | validator set hash、stake root、per-validator proof 非空且 evidence 绑定 stake root |
| unit: slashing intent | evidence 生成 governance identity penalty intent，status 暴露 pending intent |
| unit: slashing receipt | matching receipt applied 后 pending intent 清零 |
| unit: execution-world slashing receipt bridge | 持久化 governance identity penalty 会在 status 查询前注入 matching receipt |

## 兼容性
- 旧字段 `consensus.network_committed_height`、`consensus.known_peer_heads`、`observability.network_height_lag` 保留。
- 新客户端优先读 `observability.ready` 和 `consensus.network_head`。
- 旧客户端继续能解析 status，但可能只看到 `ok=false`。
- `ok` 的语义从“非 critical”收紧为 readiness；这是有意的运维语义修正。

## 风险与回退
- 风险: `ok=false` 可能让旧 smoke gate 变红。
  - 回退: gate 应改读 `running/liveness` 判断进程存活，读 `ready` 判断可服务。
- 风险: 三节点小网络 quorum 太严导致频繁 false negative。
  - 回退: public_testnet transitional config 允许 observer required=1，validator required=1，后续升到 required=2。
- 风险: NAT/private 节点在家庭网络下长期 degraded。
  - 回退: 明确 observer role 或配置 governed relay/persistent peer，不把它当 validator_core。
- 风险: mainnet readiness 已能记录 evidence、隔离 validator、输出 slashable stake preview、绑定 stake proof root，并生成治理 slashing intent；但不会在 gossip ingest 期间直接扣罚 stake。
  - 回退: 在对外口径中只声明 evidence/isolation/stake proof/slashing intent；真正 stake 扣罚以治理层 `apply_identity_penalty` receipt 为准。

## Producer 结论
当前补丁已把“peer head 不可见却健康”的 P0 缺口推进到主流公链式 readiness/gate 模型：peer-head quorum、stake-weighted mainnet quorum、validator stake proof root、freshness TTL、tier lag/stalled sync、hash/root 一致性、transport stability、mainnet relay policy、reachability policy、commit equivocation evidence、validator quarantine、governance slashing intent、slashing receipt reconciliation 和 execution-world receipt bridge 均已成为独立 API/gate。仍未声称 gossip ingest 会直接扣罚 stake；真正扣罚以治理层 `apply_identity_penalty` 执行和 receipt 为准。
