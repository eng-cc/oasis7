# oasis7 主链级非全公网 P2P 覆盖网络架构（设计文档）

- 对应需求文档: `doc/p2p/network/mainnet-private-reachability-architecture.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

审计轮次: 1
## 设计目标
- 把“没有公网 IP 的节点如何成为主链级网络一等公民”从部署特例提升成框架层能力。
- 为 oasis7 冻结一套可同时承载 mixed-topology 与多链型数据面的统一 P2P substrate。
- 该 P2P substrate 是链上大世界状态底座的网络/数据面子线，不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 目标原则
1. 身份与地址解耦：节点身份由签名 peer record 与 node identity key 定义，而不是由瞬时 IP:port 定义。
2. Reachability 是连续谱，不是二元判断：`public`、`hybrid`、`private`、`relay_only`、`validator_hidden` 都是正式模式。
3. 路径优选但不路径绑定：系统优先 direct，其次 punched，再次 relay；业务层永远看到统一的逻辑连接。
4. 角色先于实现：validator/sentry/relay/full-storage/observer 的公开面、密钥边界和 lane 权限先冻结，再决定具体库接法。
5. 链型适配在上层：Ethereum/Cosmos 类 mesh gossip、HotStuff/committee direct、Solana 类 tree broadcast、blob/DA lane 都挂在同一 substrate 上。

## 协议栈分层
| 层 | 组件 | 责任 |
| --- | --- | --- |
| `L0 identity` | `node identity key`、signed peer record、role claim | 赋予节点链内身份、角色与可达性声明 |
| `L1 discovery` | bootnodes、DHT、rendezvous、peer record cache | 聚合候选 peer、形成多源发现视图 |
| `L2 reachability` | AutoNAT、observed addr、hole punch、relay reservation | 判定 directness 并维持可工作路径 |
| `L3 transport` | QUIC、TCP/Noise、mux、stream/session abstraction | 提供统一安全传输与多路复用 |
| `L4 peer manager` | diversity policy、score、quarantine、path selection | 控制谁能进入 active peer set，以及用哪条路径 |
| `L5 traffic lanes` | gossip、sync、blob/state、control | 给不同业务流量独立的 QoS、rate limit 和 peer subset |
| `L6 chain adapters` | mesh gossip、committee direct、tree broadcast、blob availability | 承载不同公共主链风格的数据面 |

## 节点角色模型
| 角色 | 对外暴露 | 必须能力 | 禁止能力 |
| --- | --- | --- | --- |
| `validator core` | 可为零公网入站 | outbound 到 sentry/relay、consensus signer、committee lane | 不应承担大规模公开入站或浏览器 public plane |
| `sentry` | 公网或 hybrid | inbound/outbound 转发、anti-DoS 缓冲、policy enforcement | 不持有 validator 共识 signer |
| `relay / anchor` | 公网 | relay reservation、bootstrap、AutoNAT assist、rendezvous | 不解释业务权限，不持有长期 signer |
| `full-storage` | public/hybrid/private 皆可 | header/block/state/blob sync、serve range/proof | 不得冒充 validator 或 governance signer |
| `observer-light` | public/private | 轻量 sync、只读 gossip、client session | 不得请求 validator-private control lane |

## Peer Record 模型
| 字段 | 说明 |
| --- | --- |
| `peer_id` | node identity 的稳定标识 |
| `chain_id` / `network_id` | 防止跨链、跨环境重放 |
| `role_mask` | validator core / sentry / relay / full-storage / observer-light 声明 |
| `reachability_class` | `public/hybrid/private/relay_only/validator_hidden` |
| `public_addrs` | 可公开 direct dial 的地址 |
| `relay_addrs` | 可通过 relay 抵达的地址或 reservation 引用 |
| `capability_lanes` | `consensus_gossip/sync/blob_state/control` 可服务能力；缺省时按角色回填默认 lane 集 |
| `ttl` / `sequence` | 刷新、撤销和地址漂移控制 |
| `signature` | 链域隔离签名 |

设计约束:
- peer record 默认短 TTL，reachability 变化必须允许快速刷新。
- 私网地址默认不公开；若需暴露给 sentry/relay allowlist，必须作为受限提示而不是公共字段。

## Peer Reachability Contract Normalization
本节是 2026-06-15 iroh-inspired follow-up 的设计落点。借鉴点只限于 identity-first address boundary、显式 selected path 与 reachability evidence；不引入 iroh 依赖，不替换 libp2p，也不采用 iroh relay/DNS/Pkarr 默认。

### Contract 目标
`PeerReachabilityContract` 是 oasis7 内部归一化视图。本阶段实现范围是 signed peer record 的 identity-first record normalization：把远端声明的 identity、role/deployment/reachability、capability lanes 与 ranked transport paths 收到一个内部 contract 中，避免 path ranking/dedupe 逻辑散落。目标态再逐步把 runtime-observed selected path、local deployment/role policy 与 network-tier publish policy 作为显式输入接入同一投影边界。

| 字段 | 来源 | 用途 |
| --- | --- | --- |
| `peer_id` | signed peer record / libp2p identity | 稳定身份，不随地址漂移 |
| `deployment_mode` | peer record；目标态叠加 local policy | 判定 `public/hybrid/private/relay_only/validator_hidden` 公开面 |
| `node_role` | peer record；目标态叠加 local role admission | 判定 validator/sentry/relay/full-storage/observer 权限 |
| `reachability_class` | peer record declaration；目标态叠加 local observation | 表达当前可达性分类；远端声明不可覆盖本地探测 |
| `capability_lanes` | peer record effective lanes | request peer selection 与 lane serve/request gate |
| `ranked_paths` | `TransportPath` normalization | 当前首版实现的 canonical path ranking/dedupe；`direct -> hole-punched -> relay` |
| `publish_policy` | 目标态：deployment/network-tier manifest | 决定哪些地址可公开、哪些只能作为 allowlist hint；本阶段不改变 publish policy |
| `selected_path` | 目标态：runtime path selector/status projection | 当前正在使用的 path kind/flavor 与 selected-at 时间；本阶段由 status projection 输出 |
| `claim_boundary` | matrix/status projection | 指出该 peer/path 能支持的 claim 类型，例如 `exact`, `proxy`, `manual_lab_required` |

### 真值链
1. `SignedPeerRecord` 继续是远端声明与签名真值。
2. `TransportPath` 继续是 libp2p path materialization 与排序真值。
3. 本阶段 `PeerReachabilityContract` 归一化 signed peer record 与 ranked `TransportPath`，不改变 wire schema、publish policy 或 network-tier admission。
4. request peer selection 可先消费 `ranked_paths` 投影；debug/status/matrix 继续消费现有 runtime/status truth，但不得重新定义 direct/hole-punched/relay 标签。
5. 目标态若需要 local policy、selected path age、transition reason 或 publish boundary，先扩展 contract 输入/投影，再选择性投影到 status；不要先新增第二套 status truth。

### 非目标与防重复真值
- 不把 libp2p `Multiaddr` 替换成 iroh address 类型。
- 不让 matrix 脚本或 triad report 反向定义 canonical reachability。
- 不把 relay reservation 等同于 `public reachable`。
- 不把 proxy drill 成功等同于 physical NAT/CGNAT 覆盖。
- 不导出 unbounded per-peer Prometheus labels；peer-level 明细只允许 bounded snapshot/top-N 或 debug-only artifact。

## Reachability 生命周期
1. 启动时通过 bootnode / relay / observed addr 获取自我外部视图。
2. Reachability service 判定 `unknown/public/private/symmetric_nat`。
3. 能 direct 的节点直接发布 public addrs。
4. 不能 direct 的节点尝试 hole punching；成功则发布 punched-capable addrs。
5. 仍失败则进入 relay reservation，发布 relay route 并将自身标记为 `relay_only` 或 `validator_hidden`。
6. path quality 持续观测；若 direct 恢复，优先切回 direct，但保留 relay 作为热备用。

## 路径选择与传输策略
| 优先级 | 路径 | 适用场景 | 说明 |
| --- | --- | --- | --- |
| `P0` | direct QUIC | public/hybrid、已恢复直连 | 低延迟主路径 |
| `P1` | hole-punched QUIC | NAT 但可打洞 | 对业务透明，仍视作 direct-like |
| `P2` | relay QUIC/TCP tunnel | CGNAT、企业内网、临时故障 | 必须预算和多样性控制 |
| `P3` | delayed sync fallback | 极端受限环境 | 只允许 sync，不得承担实时 validator lane |

设计约束:
- UDP 可作为 QUIC/datagram 加速面，但不得作为唯一真值 path。
- transport/session key 应频繁轮换，不等价为 node identity 或 consensus signer。

## Traffic Lanes
| Lane | 典型载荷 | peer subset | QoS 特征 |
| --- | --- | --- | --- |
| `consensus gossip lane` | proposal、vote、commit、finality hints | validator、sentry、少量高分 full node | 低抖动、低队头阻塞 |
| `sync lane` | header range、block range、state checkpoint | full-storage、sentry、private full | 可重试、支持长流 |
| `blob/state lane` | DistFS、DA、snapshot、proof | storage、archive、proof provider | 独立限速，不能压制 consensus |
| `control lane` | heartbeat、peer record refresh、reachability probe | role-aware | 高优先、低带宽 |

实现落点（2026-04-02 / P2PARCH-4）:
- `oasis7_proto::distributed_net` 现在冻结 `NetworkLane` / `NetworkLaneQosClass` / topic+protocol classifier，作为 lane registry 的共享真值。
- `PeerRecord` 现在显式支持 `capability_lanes`；若旧 record 未声明，则按 `node_role` 回填默认 lane capability，保持向后兼容。`observer-light` 只默认声明 `consensus_gossip + control`，不再被当作 `sync/blob_state` 服务提供者。
- `oasis7_net` 现在按 lane 选择 subscription inbox 配额，并在 req/resp 路径优先筛掉不具备对应 lane capability 的 peer record。
- `oasis7_node` 现在把 replication / consensus 的 topic-protocol 绑定提升为 traffic lane registry，并在 runtime config 的 `node_role_claim` 上执行 lane publish/subscribe/request/serve 权限校验；`observer-light` 仍可主动请求 `sync/blob_state`，但不会注册对应 serve handler，也不能通过 `feedback_p2p` 绕过 `blob_state` lane 的 publish/subscribe gate。

## Anti-Eclipse / Anti-Spam 基线
- Active peer set 至少来自两类 discovery source。
- 同一 operator、同一 ASN、同一 `/24` 的 active peer 占比默认上限 `25%`。
- validator_hidden 至少维持两条独立 ingress path，且不能同属一个 relay-domain。
- relay 采用 budget 和 quota；blob/state 流量不得吞噬 consensus/control 预算。
- 发现异常时，peer manager 要按阈值把节点置为 `suspect` 或 `blocked`，而不是继续乐观接纳。

实现落点（2026-04-07 / P2PARCH-5 第二个切片）:
- `oasis7_net` 新增 `PeerManagerPolicy` / `PeerManagerPeerHealth` / `PeerManagerHealthIssue` substrate，在 libp2p worker 内基于已发现 peer record 与 active transport path 计算本地 peer health snapshot。
- 当前已接线的 fail signatures 包含：`single-source active set`、单 peer `single-source discovery`、IPv4 `/24` 集中、relay-domain 集中、operator 集中、ASN 集中、relay budget 超限。
- runtime 默认把同一 `operator / ASN / /24 / relay-domain` 的 active peer 占比 `>25%` 视为 `suspect`，占比 `>=50%` 视为 hard-`blocked`，并在 peer health 中保留触发阈值。
- req/resp peer 选择现在会在 lane capability 过滤后，继续优先选择 `active/candidate` peer，把 `suspect` 压到最后，并直接排除 hard-`blocked` peer；只有 `MissingPeerRecord` / record-exchange-pending 这类 soft-`blocked` peer 会在没有更健康候选时作为 bootstrap fallback。
- discovery ingress 现在不再对 `RoutingUpdated` / rendezvous registration 暴露的裸地址做 speculative dial；runtime 会先拿到并校验 signed peer record，再按 peer health 决定是否拨号，避免 `MissingPeerRecord => Blocked` 语义被旁路。
- `suspect/blocked` peer 现在也不会污染 discovery dial dedupe 状态；同一地址若后续随更健康的 peer record 刷新回来，仍可重新进入首拨决策。
- active-set quarantine 现在已开始生效：当已连接 peer 刷新为 `suspect` 或已验证的 hard-`blocked` 时，runtime 会主动 `disconnect_peer_id`，并在 `ConnectionClosed` / `OutgoingConnectionError` 上抑制 failover 与 retry，避免 quarantined peer 被本地状态机立即拉回 active set。
- peer health 对外发布前会先剔除未准入 active peer：待校验的 `MissingPeerRecord` 连接与本轮判定出的 quarantined active peer 不再参与同轮 `/24`、relay-domain、relay budget 的健康统计，避免瞬时污染其他健康 peer。
- chain runtime 默认 peer record 现已支持 `source_operator/source_asn` 输入，peer manager 会在本地归一化标签后参与 diversity 统计。
- runtime 现已为 `blocked` peer 维护可调试的 block artifact，至少跨 peer-manager 重算保留 `peer_id/status/issues/path/operator/asn/first_blocked_at/last_blocked_at/last_cleared_at`；当前仍未升级为跨重启 banlist 或 release-gate 证据存储。
- `P2PARCH-5` 当前已按 runtime substrate milestone 收口；剩余 `mixed-topology / fail-signature` required/full 套件与 release evidence contract 不再阻断 `P2PARCH-5` 关闭，而是继续留在 `P2PARCH-6/7`。
- `P2PARCH-6` 现已新增 `scripts/p2p-mixed-topology-matrix.sh`：QA 会把 `private/validator_hidden/relay_only` 角色边界、bootstrap poisoning、relay exhaustion 与 path failover 组装成 `required` exact matrix，再把 triad/triad_distributed 的 disconnect/restart longrun 组装成 `full` proxy matrix。该 matrix 会明确标注 `proxy != dedicated sentry/NAT lab`，避免把当前可执行近似 drill 误写成最终 mixed-topology 实证。

## Path Behavior Matrix Taxonomy
Matrix 是 evidence/claim taxonomy，不是 runtime reachability truth。它的职责是把“当前证据能支持什么说法”写清楚。

| 维度 | 允许值 | 说明 |
| --- | --- | --- |
| `evidence_class` | `exact / proxy / manual_lab / real_env / unsupported` | `exact` 为 deterministic repo test；`proxy` 为当前可执行近似 longrun；`manual_lab` 需要专门 NAT/sentry lab；`real_env` 为同窗口真实网络证据；`unsupported` 明确不承诺 |
| `path_expectation` | `must_direct / may_direct_must_recover / must_relay / must_not_publish_public_direct / manual_lab_required / unsupported` | 描述粗粒度 claim；case-specific route sequence 留在 `expected_route` |
| `reachability_pair` | `public_direct / home_nat / cgnat / relay_only / validator_hidden / cloud_public / mixed_validator_observer` | 描述 topology pair 或角色组合 |
| `degradation_class` | `none / sentry_loss / relay_exhaustion / degraded_latency_loss / restart_pause_disconnect / bootstrap_poisoning` | 描述弱网、故障或攻击近似场景 |
| `claim_boundary` | `required_exact / full_proxy / physical_nat_pending / shared_window_partial / public_testnet_blocked_or_ready` | 直接限制对外或 release gate 语言 |

Gate 语义：
- `test_tier_required` 只接受 `coverage=exact` 且 deterministic 的 case。
- `test_tier_full` 可以包含 `proxy` 和 `real_env`，但必须在 summary 中写明哪些 claim 仍需要 `manual_lab` 或 dedicated evidence。
- `public_testnet` 或 `shared_devnet` 升级 claim 必须同时引用 matrix summary、same-window evidence 和 producer/QA pass-uplift decision；不能只改 case 状态。

## Reachability Status Observability Projection
Status observability 只投影 contract 与 runtime-selected path 的 bounded summary，默认接入现有 `/v1/chain/status.observability` 与 triad merged summary。

建议字段：
- `selected_path_kind`: `direct / hole_punched / relay / unknown`。
- `selected_path_age_ms` 或窗口摘要，避免 per-peer 高基数序列。
- `path_transition_counters`: `direct_to_relay`, `relay_to_direct`, `direct_to_hole_punched`, `hole_punched_to_relay`。
- `active_path_mix`: 当前 direct/hole-punched/relay active counts。
- `recent_fallback_reason`: bounded enum，禁止自由文本无限扩张。
- `reachability_confidence`: `observed_direct / relay_reserved / punched_recently / proxy_only / manual_lab_required` 等 operator-facing 摘要。

边界：
- byte split 只能在现有 metrics scope 可证明时输出；不得把 wire/control-plane bytes 说成 NIC-level overhead。
- status 不是第二套健康权威；release claim 仍回到 matrix/shared-network gate。

## 适配多链型的数据面
| 适配器 | 典型链型 | 说明 |
| --- | --- | --- |
| `MeshGossipAdapter` | Ethereum/Cosmos 风格 | 使用 topic mesh + req/resp sync |
| `CommitteeDirectAdapter` | HotStuff/BFT 风格 | 委员会成员之间建立更强的定向链接 |
| `TreeBroadcastAdapter` | 高吞吐 leader-fanout 风格 | 上层控制广播树和 fanout，不重写底层 reachability |
| `BlobAvailabilityAdapter` | DA / DistFS / proof network | 独立 blob/state lane 与预算 |

## 对 oasis7 的迁移含义
1. `oasis7_node` / `oasis7_net` 不再暴露“静态 UDP peer 列表就是主路径”的默认语义。
2. consensus、replication、DistFS 都通过统一的 traffic-lane API 请求逻辑连接，而不是各自决定 transport 真值。
3. 当前 triad / legacy shared-network rehearsal / hosted access 都转成 deployment profile，而不是继续依赖特例脚本。
4. 本地家宽节点默认走 `private` 或 `validator_hidden` 模式，不再被迫伪装成 `public`。

## 当前结论
- 当前允许:
  - `public-chain-grade private-reachability architecture is specified`
  - `mixed-topology is a first-class target`
  - `validator no-public-IP operation is architecturally allowed via sentry/relay model`
- 当前禁止:
  - `all nodes must expose public IP`
  - `single relay dependency counts as public-chain-grade`
  - `current implementation already satisfies this architecture end to end`
