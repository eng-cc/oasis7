# oasis7 主链级非全公网 P2P 覆盖网络架构（设计文档）

- 对应需求文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
- 对应项目管理文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md`

审计轮次: 1
## 设计目标
- 把“没有公网 IP 的节点如何成为主链级网络一等公民”从部署特例提升成框架层能力。
- 为 oasis7 冻结一套可同时承载 mixed-topology 与多链型数据面的统一 P2P substrate。

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
| `capability_mask` | gossip、sync、blob、control 可服务能力 |
| `ttl` / `sequence` | 刷新、撤销和地址漂移控制 |
| `signature` | 链域隔离签名 |

设计约束:
- peer record 默认短 TTL，reachability 变化必须允许快速刷新。
- 私网地址默认不公开；若需暴露给 sentry/relay allowlist，必须作为受限提示而不是公共字段。

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

## Anti-Eclipse / Anti-Spam 基线
- Active peer set 至少来自两类 discovery source。
- 同一 operator、同一 ASN、同一 `/24` 的 active peer 占比默认上限 `25%`。
- validator_hidden 至少维持两条独立 ingress path，且不能同属一个 relay-domain。
- relay 采用 budget 和 quota；blob/state 流量不得吞噬 consensus/control 预算。
- 发现异常时，peer manager 要把节点置为 `suspect` 而不是继续乐观接纳。

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
3. 当前 triad / shared-network / hosted-world 都转成 deployment profile，而不是继续依赖特例脚本。
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
