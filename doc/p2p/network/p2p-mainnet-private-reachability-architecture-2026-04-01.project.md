# oasis7 主链级非全公网 P2P 覆盖网络架构（项目管理文档）

- 对应设计文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md`
- 对应需求文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`

审计轮次: 2
## 任务拆解（含 PRD-ID 映射）
- [x] P2PARCH-0 (PRD-P2P-024-A/B/C/D/E) [test_tier_required]: 新建“主链级非全公网 P2P 覆盖网络架构”专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。
- [x] P2PARCH-1 (PRD-P2P-024-A/B) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 node identity、signed peer record、bootnode/DHT/rendezvous 发现链路，并让业务层不再直接依赖静态 UDP peer truth。
  已落地 stable libp2p identity、signed peer record schema + DHT contract、默认 bootstrap/DHT/rendezvous discovery taxonomy，以及 query-driven peer acquisition（DHT discovery query + bootstrap cached peer list/record fallback + rendezvous register/discover 自动化）；rendezvous-discovered peer 继续经由 world/network/signature 校验后才进入候选集。
- [x] P2PARCH-2 (PRD-P2P-024-B/D) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 transport abstraction，统一 direct / hole-punched / relay path，并把 QUIC/TCP/Noise/mux 语义冻结到 substrate。
  已落地 transport substrate 收口：peer record 现在显式区分 `direct_addrs / hole_punch_addrs / relay_addrs`，runtime 会按 `direct QUIC -> direct TCP -> hole-punched QUIC/TCP -> relay-reserved` 排序与 failover；swarm 同时承载 direct transport 与 relay client transport，并记录 relay reservation / DCUtR 事件用于后续 reachability lifecycle。
- [x] P2PARCH-3 (PRD-P2P-024-C/D) [test_tier_required + test_tier_full]: `runtime_engineer` 落 `public / hybrid / private / relay_only / validator_hidden` deployment mode 与 `validator core / sentry / relay / full-storage / observer-light` 角色策略。
  已落地 role policy substrate：runtime config 新增显式 `deployment_mode + node_role_claim`，默认把 `sequencer/storage/observer` 映射到 `validator_core/full_storage/observer_light`，并允许 observer runtime 显式声明 `sentry/relay`。peer record 现在显式携带 `deployment_mode`，且会校验 deployment mode、network role 与 direct surface 的一致性，旧 `sequencer/storage/observer` peer record label 仍可兼容解析到新角色语义。
- [x] P2PARCH-4 (PRD-P2P-024-B/C) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 traffic lanes，把 consensus gossip、sync、blob/state、control 拆成独立 QoS 与 peer subset。
  已落地 lane/QoS substrate：`oasis7_proto::distributed_net` 现已冻结 `NetworkLane` / `NetworkLaneQosClass` / topic+protocol classifier；`PeerRecord` 新增 `capability_lanes` 并对 legacy record 做 role-based defaulting，且 `observer_light` 不再默认宣称 `sync/blob_state` 服务能力；`oasis7_net` 会按 lane 选择 subscription inbox 配额，并在 req/resp 选 peer 时优先过滤掉不具备对应 lane capability 的 peer；`oasis7_node` 已把 replication / consensus / `feedback_p2p` 绑定提升为显式 lane registry，并对 `node_role_claim` 执行 publish/subscribe/request/serve 权限校验，observer 只保留 data-lane request，不注册 data-lane serve handler，也不能通过 `feedback_p2p` 订阅或发布 `blob_state` lane topic。
- [x] P2PARCH-5 (PRD-P2P-024-B/E) [test_tier_required]: `runtime_engineer` 落 peer manager、anti-eclipse、diversity、relay budget 与 quarantine substrate。
  已完成第三个切片：在首个 peer-manager substrate 与 active-set quarantine enforcement 基础上，`oasis7_net` 已把 `source_operator/source_asn` 接入 peer record、peer manager 健康判定与 block artifact 跟踪。runtime 现在会对 `operator / ASN / subnet / relay-domain` 做正式阈值判定，并为 hard-`blocked` peer 保留跨重算可追溯的 block artifact；本任务的完成范围到 runtime substrate 与定向 required 验证为止，不再把 mixed-topology required/full 证据和 release-gate artifact 挂在 `P2PARCH-5` 名下。
- [x] fixed-triad-low-traffic-profile (PRD-P2P-024) [test_tier_required]: `runtime_engineer` 为固定 triad / ECS + 本机 observer 场景新增显式低流量运行档位，避免 AutoNAT、过快 redial/discovery 与过宽 gossip/feedback 预算在私有固定拓扑里制造无效流量。 Trace: .pm/tasks/task_e81d3dd8628747de9a96c6aa0eae6c82.yaml
  已落地 `--traffic-profile triad_low_traffic`：chain runtime 会在不改变 `release_default` 安全加固语义的前提下，同时收紧 `bootstrap_redial_interval_ms=10s`、`discovery_query_interval_ms=180s`、`peer record republish=30m`、`max_dynamic_gossip_peers=8`、`dynamic_gossip_peer_ttl=1h`、`feedback_p2p announce budget=8/8`，并对固定 triad 档位关闭 libp2p AutoNAT，避免本机 observer 与阿里云节点持续消耗控制面探测流量。
- [ ] P2PARCH-6 (PRD-P2P-024-D/E) [test_tier_required + test_tier_full]: `qa_engineer` 建立 mixed-topology 套件，覆盖家宽/NAT、CGNAT、relay exhaustion、sentry loss、bootstrap poisoning、path failover。
  已把 full-tier 从 dry-run 推进到真实 proxy execution：`scripts/p2p-mixed-topology-matrix.sh` 现在会把 shared-window / dedicated-lab / pass-uplift 外部证据与 blocker 语义写入 `summary.json/md`，proxy case 也不再依赖预编译 binary 或默认 561x 端口段。2026-04-07 latest full run（`doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md`）确认 7 个 exact case 全通过，same-window shared refs 已接入 summary，但 2 个 proxy longrun 仍以 `consensus_hash_divergence`、`committed_height_not_monotonic nodes=sequencer`、`known_peer_heads_zero_samples`、`http_failure_samples` 失败，因此当前仍停留在 `required_exact_ready=true / full_proxy_ready=false` 的 audited `partial`。
  同日新增真实三节点 follow-up（`doc/testing/evidence/p2p-private-observer-triad-follow-up-2026-04-07.md`）已确认本机 observer 在 `1` 本机 + `2` ECS 环境中修平 `known_peer_heads_zero_samples` 主签名：observer 现可持续看到 `known_peer_heads=1`、`network_committed_height>0`，并已把 `committed_height` 从 `0` 推到 `88`。该 real-run 同时把残余风险重新收敛到 `sequencer` 单点 stale execution state，而不再是 private observer 无法反向建链或 signer/replication allowlist 拒绝。
  2026-04-07 runtime follow-up（`doc/testing/evidence/shared-network-ecs-triad-upgrade-2026-04-07.md`）已把本机 observer 与两台 ECS 统一升级到同一版 `oasis7_chain_runtime`（release=`89860f6eb6d5`，sha256=`26a41315e0bcc34cad996ef73fa9289455f004393190fb65ce05217bc8c8e1dc`），因此“triad 版本不一致”已不再是当前 real-env blocker；升级后 observer 仍保持 `known_peer_heads=0 / network_committed_height=0 / committed_height=0`，说明下一步应继续沿 peering / reachability / observer ingest 路径排查。
  2026-04-08 reconfirm（`doc/testing/evidence/p2p-real-env-triad-reconfirm-2026-04-08.md`）随后把 real-env 主 blocker 进一步钉到 `sequencer_execution_stale_height`，并在仓库内补上 execution bridge 从 exact execution record 回收 stale state 的恢复路径与定向回归。
  同日 fresh rollout follow-up（`doc/testing/evidence/p2p-real-env-triad-stale-height-rollout-2026-04-08.md`）已将 `HEAD=f8b1baf97316` fresh build 的 `oasis7_chain_runtime`（sha256=`72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`）部署到两台 ECS，并通过 same-window snapshot（`.tmp/p2p_real_env_triad/20260408-132008/summary.json`）确认 sequencer 不再复现 `execution driver received stale height: context=57536 state=57560`。但当前 real-env triad 仍被新的 `sequencer` 单点签名阻断：`storage challenge gate network threshold unmet`，同时 observer 继续停在 `gap sync ... blob not found` 导致 `committed_height` 窗口内不推进。因此本专题的真实环境 blocker 已从 stale execution recovery 前移到 storage challenge / blob availability residual，而不是已经 uplift 成 `pass`。
  同日 blob-availability root-cause follow-up（`doc/testing/evidence/p2p-real-env-triad-blob-availability-root-cause-2026-04-08.md`）进一步确认：storage 的 `committed_height/network_committed_height` 可以单独跟着 consensus 前进，而 replication root 仍保持空目录；旧实现又把 replication apply/gap-sync 起点错误绑定到 `committed_height`，导致 storage/observer 一旦错过历史 replication 流就不会再主动回补，sequencer 随后又会被 `storage challenge gate` 卡住，形成真实环境死锁。后续同日二次 rollout 已把新 release `95ae1e3ee604-blob-route-fallback-20260408`（sha256=`0179d52afb91355821dcfbeb94c83c7bb10eb174fe1d81d41fbf16d27b26329a`）实装到本机 observer 与两台 ECS，并确认 observer 已恢复 `last_error=null` 且继续推进 `committed_height`，storage 也开始持续回补 `replication_commit_messages/store-blobs`。当前残余 blocker 已进一步收敛为：sequencer 仍因 `storage challenge gate network blob not found` 停在最新 hash，而 storage 仍在按历史高度顺序追平；因此 P2PARCH-6 继续维持 `blocked`，但已不再是 observer 无法自愈或 blob 路由直接选错 peer 的原始问题。
- [ ] P2PARCH-7 (PRD-P2P-024-E) [test_tier_required + test_tier_full]: `producer_system_designer` + `liveops_community` + `qa_engineer` 把 shared-network / release-train / claim gate 升级为 mixed-topology 正式门禁。
  已把 mixed-topology lane 升级为 shared-network required gate：`shared-network-track-gate.sh` 现在要求 `shared_devnet/mixed_topology_baseline`、`staging/mixed_topology_rehearsal`、`canary/mixed_topology_claim_review` 三条显式 lane；`shared-devnet-rehearsal.sh` 也会自动生成 mixed-topology gate note，并默认把仅有 `P2PARCH-6` matrix baseline 的窗口保持在 `partial`。
  当前已把 `mixed_topology_baseline` 从“缺草稿”推进成正式 `partial` evidence：`doc/testing/evidence/shared-network-shared-devnet-mixed-topology-draft-2026-04-03.md` 现在显式钉住 `P2PARCH-6` baseline、same-window shared-devnet follow-up/short-window 证据与 proxy 边界；`shared-devnet-blocker-packet.sh` 继续负责生成 `shared_access / mixed_topology_baseline / rollback_target_ready` 三份 blocker 文档，方便后续把 same-window mixed-topology 证据继续回填到 shared-devnet gate，而不冒充已 `pass`。若要正式升到 `pass`，除 same-window evidence 外还必须固定 producer/QA 审计通过的 pass-uplift decision ref。
- [x] P2PARCH-8 (PRD-P2P-024-F) [test_tier_required]: `producer_system_designer` 冻结用户层部署模式抽象：用户界面只暴露 `自动加入 / 私有安全 / 公网入口` 这类 `2~3` 个简单模式，默认由系统根据公网/NAT/打洞结果自动选择；底层继续保留 `deployment_mode/node_role` 正式语义。
- [x] P2PARCH-9 (PRD-P2P-024-F) [test_tier_required + test_tier_full]: `runtime_engineer` + `viewer_engineer` 把 AutoNAT / port reachability / hole-punch 结果接成用户层默认模式推荐，并为 `公网入口` 或其他高风险暴露职责补显式确认与高级设置覆盖。
  已落 full-tier launcher/viewer UX baseline：chain status 已把 requested/recommended/applied user mode、reachability evidence 与底层 role mapping 透传到 launcher；`oasis7_client_launcher` 现已展示三档 simple modes、检测依据和 `public_entry` accept/reject 路径，并以 `doc/testing/evidence/p2p-user-mode-launcher-ux-2026-04-07.md` 固化当前 test_tier_full 证据。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - 覆盖网络架构 verdict: `partial`
- 当前专题结论:
  - 已冻结目标态，不走 “先做一个 NAT patch 的 MVP” 路线。
  - `P2PARCH-1` 已落首个 identity/discovery substrate 切片：runtime 现在会从 node root key 派生稳定 libp2p identity，并发布/校验 signed peer record。
  - `P2PARCH-1` 已补齐 query-driven discovery acquisition：runtime 会周期性刷新 DHT peer discovery，并在 provider 查询只返回 self 或未命中 peer record 时，向已连接 bootstrap 拉取缓存 peer 列表/peer record，再按 world/network/signature 校验后并入候选集。
  - `P2PARCH-1` 已补齐 rendezvous 自动化：runtime 会在连接 bootstrap / rendezvous 节点后自动注册当前 namespace，并带 cookie 周期 discover 同 namespace registrations；发现结果仍需经 peer record 校验后才会入候选与拨号集合。
  - `P2PARCH-2` 已落首个 transport substrate 切片：runtime 现在会把 signed peer record 中的 `direct_addrs/relay_addrs` 显式提升成带 kind/security/mux 语义的 transport path 集合，并按 `direct -> relay` 顺序选路；当首选 path 失效时，会自动尝试下一条已知 path。
  - `P2PARCH-2` 已补 QUIC/TCP fallback 切片：swarm 现在同时承载 QUIC 与 TCP/Noise/Yamux，transport path 会按 `direct QUIC -> direct TCP -> relay` 排序；active path、failover 与 discovery 升级判断也会保留这一语义。
  - `P2PARCH-2` 已补 hole-punched / relay-reserved substrate：peer record 新增 `hole_punch_addrs`，listener materialization 会把 `/p2p-circuit` 地址单独沉淀进 `relay_addrs`；transport path 排序固定为 `direct QUIC -> direct TCP -> hole-punched QUIC/TCP -> relay-reserved`，active path 也会保留已知 path kind。
  - `P2PARCH-2` 已补 relay client transport 与 DCUtR 事件接线：swarm 现在可直接承载 `/p2p-circuit` relay transport，并在 reservation accepted 时刷新 peer record / provider 广告；hole-punch success/failure 事件已进入 runtime 观测面，后续只剩 reachability lifecycle 自动化与 mixed-topology 套件闭环。
  - `P2PARCH-3` 已落 deployment/role policy substrate：`NodeConfig` / chain runtime CLI / default peer record 现在显式承载 `p2p_deployment_mode` 与 `p2p_node_role`，不再把 deployment mode 只留给 operator 约定。
  - `P2PARCH-3` 已落 role admission：runtime 会校验 `sequencer -> validator_core`、`storage -> full_storage`、`observer -> observer_light|sentry|relay`，并拒绝 `validator_core + public/relay_only`、`sentry + 非 public/hybrid`、`relay + 非 public`、`validator_hidden + 非 validator_core` 这类无效组合。
  - `P2PARCH-3` 已落 exposed-surface contract：peer record 会显式校验 `private/relay_only/validator_hidden` 不得发布 `direct_addrs`，`validator_core` 也不得直接暴露 public direct surface；对 legacy `sequencer/storage/observer` peer record label 仍保持兼容解析，避免 discovery 面一次性断代。
  - `P2PARCH-4` 已落 lane taxonomy substrate：共享协议层现在显式区分 `consensus_gossip/sync/blob_state/control` 四条 lane，并冻结每条 lane 的 QoS class 与 topic/protocol classifier，不再把 lane 判断散落在业务字符串上。
  - `P2PARCH-4` 已落 peer capability substrate：peer record 现在可显式声明 `capability_lanes`，未声明时按 canonical `node_role` 自动回填默认 lane capability，保证 discovery/selection 面能平滑兼容旧 record；其中 `observer_light` 不再默认宣称 `sync/blob_state` 服务能力。
  - `P2PARCH-4` 已落 role-aware binding：runtime 会拒绝 `observer_light` 直接 publish consensus lane、服务 `sync/blob_state` data lane、或通过 `feedback_p2p` 订阅/发布 `blob_state` topic，以及 `relay` 请求或服务 data lane 这类明显越权的 lane 操作；replication fetch handler 注册也会受 `node_role_claim` gate 约束。
  - `P2PARCH-4` 已落 request peer subset：`fetch-commit/fetch-blob` 这类 req/resp 现在会优先筛选声明具备对应 lane capability 的 peer record，再发起 outbound request，避免继续对所有已连接 peer 一视同仁。
  - `P2PARCH-5` 已落首个 peer-manager substrate：`oasis7_net` 现在会基于已发现 peer record 与 active transport path 计算本地 peer health snapshot，并对 `single-source active set`、IPv4 `/24`、relay-domain 与 relay budget 超限发出 `suspect` 信号；request peer 选择会优先选择 `active/candidate`、把 `suspect` 压到最后并直接排除 `blocked` peer。
  - `P2PARCH-5` 已把 discovery ingress 接上首轮 enforcement：`RoutingUpdated` / rendezvous registration 不再绕过 signed peer record 校验直接拨号；`suspect/blocked` peer 也不会提前占用 discovery dial dedupe，使后续 record 升级后仍可重新进入拨号决策。
  - `P2PARCH-5` 已把 quarantine 接到 active connection：已连接的 `suspect` 与已验证 hard-`blocked` peer 现在会被主动断连，且 `ConnectionClosed` / `OutgoingConnectionError` 不再对这些 peer 继续 failover 或 retry；同轮 health 统计会先剔除未准入 active peer，避免坏连接瞬时污染其他健康 peer。
  - `P2PARCH-5` 已把 `source_operator/source_asn` 接进 runtime 默认 peer record、peer manager 健康快照与调试面；active peer set 现在会对 operator/ASN 与 `/24`、relay-domain 一样执行正式多样性阈值，并把 `>25%` 记为 `suspect`、`>=50%` 升级为 hard-`blocked`。
  - `P2PARCH-5` 已把 hard-`blocked` 结果沉淀为 process-durable block artifact：runtime 会跨 peer-manager 重算保留 `peer_id/status/issues/path/operator/asn/first_blocked_at/last_blocked_at/last_cleared_at`，并通过 debug API 暴露给后续 QA / release-gate 取证。
  - `P2PARCH-5` 现已按 runtime milestone 收口：本切片完成后，只保留 peer-manager substrate、operator/ASN 阈值、quarantine 与 process-durable block artifact 的实现真值；mixed-topology required/full 套件与 public claims gate 不再作为关闭 `P2PARCH-5` 的阻断项。
- `P2PARCH-6` 已落首个 mixed-topology validation matrix slice：QA 现在可用一个统一脚本同时编排 `required` exact cases（private/NAT policy、validator_hidden、relay_only、bootstrap poisoning、relay-budget detection、path failover）和 `full` proxy cases（triad/triad_distributed ingress-loss release drills），并把 `proxy != dedicated sentry/NAT lab` 作为证据口径显式写入 summary。
- `P2PARCH-6` 已把 latest full-tier 真跑到 proxy soak：matrix summary 现在会额外钉住 `required_exact_ready/full_proxy_ready/shared_network_pass_blockers` 等字段；latest live run 虽未通过 proxy gate，但已把 full-tier blocker 从“只停留在 dry-run”推进成“有实际 failure signatures 的 audited partial”。
  - `P2PARCH-8` 已冻结用户层部署抽象：后续产品默认应把正式角色藏在内部，普通用户只看到 `2~3` 个简单模式，且默认由系统自动选择。
  - `P2PARCH-9` 已继续推进 runtime user-mode recommender：在保留 CLI detection hint 覆盖通道的同时，runtime 现在也会把 live relay reservation、DCUtR 打洞结果与 active transport path kind 合并成默认推荐依据；`public_entry` 自动升级仍必须携带显式确认，chain runtime status payload 会按请求时的 live snapshot 重新计算 requested/recommended/effective user mode。
  - 当前实现仍未达到统一 substrate；triad 验证暴露的问题证明 topology 是真实 blocker，不再归类为单点部署细节。
  - 后续 workstream 必须优先收敛底层 framework，而不是继续在业务层追加静态 peer / UDP 兜底。

## 角色拆解
### P2PARCH-1 / runtime_engineer
- 输入:
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
  - `doc/p2p/node/node-net-stack-unification-readme.prd.md`
  - `doc/p2p/node/node-replication-libp2p-migration.prd.md`
- 输出:
  - peer record schema
  - discovery source taxonomy
  - node identity 与 consensus signer 分离方案
- 本轮已交付:
  - `peer_id` 从现有 node root key 稳定派生，避免 runtime 重启后 libp2p identity 漂移
  - `SignedPeerRecord` / `PeerRecord` schema、DHT key contract、签名校验与查询/发布接口
  - runtime 默认 peer record 组装与 `static_bootstrap` / `dht` discovery source 标记
  - query-driven peer acquisition：周期性 DHT peer discovery query、peer record 获取、bootstrap cached discovery peers/peer record fallback、发现后自动写回 kademlia candidate set
  - rendezvous namespace 自动注册与发现：连接 bootstrap / rendezvous peer 后自动 register/discover，并把注册结果经 world/network/signature 校验后写回候选集
- 完成定义:
  - 业务层不再把静态 peer 地址当作唯一网络真值
  - discovery 至少支持两类独立 source
  - 当前实现已具备 `static_bootstrap + dht + rendezvous` 三类 source

### P2PARCH-2 / runtime_engineer
- 输入:
  - P2PARCH-1 identity/discovery 结果
- 输出:
  - direct / punched / relay 统一 transport API
  - path ranking 与 failover 策略
- 本轮已交付:
  - libp2p runtime 内部 `transport path` substrate：把 peer record 的 `direct_addrs/relay_addrs` 物化为显式 path 集合，冻结当前 `kind/security/mux` 语义
  - direct-before-relay 选路：discovery/peer record 处理不再盲拨全部地址，而是先按显式 path ranking 选择首选 path
  - path failover：当首选 path 在连接建立前失败或已建立连接关闭时，runtime 会自动切到下一条已知 path
  - QUIC/TCP fallback：`build_swarm` 现在用 `OrTransport` 组合 QUIC 与 TCP/Noise/Yamux，并把 endpoint / peer record 都分类成统一的 `transport path` 语义
  - direct transport ranking 细化：当前优先级已固定为 `direct QUIC -> direct TCP -> relay`，discovery 发现更优 direct QUIC path 时可主动替换已有 direct TCP path
  - hole-punched / relay-reserved path 收口：peer record 与 active endpoint 现在都会显式保留 `HolePunched` / `RelayReserved` kind，并把 relay session 归一到 `RelayTunnel + Noise + Yamux` 语义
  - relay reservation substrate：listener relayed 地址会与 direct 地址分开发布，swarm 内部已承载 relay client transport，`/p2p-circuit` path 可直接拨号与监听
  - reachability event surface：runtime 现在会记录 relay reservation accepted / relay circuit / DCUtR success/failure 事件，并在 relay reservation 建立后触发 peer record / provider 刷新
- 完成定义:
  - direct -> punched -> relay 对业务透明
  - relay failure 可自动切换备用路径

### P2PARCH-3 / runtime_engineer
- 输入:
  - 本专题 deployment modes 与 role model
- 输出:
  - deployment config schema
  - role admission / exposed-surface policy
- 本轮已交付:
  - `NodeNetworkPolicy`：runtime config 现在显式区分共识 `NodeRole` 与 P2P `deployment_mode/node_role_claim`
  - chain runtime CLI 接线：新增 `--p2p-deployment-mode` / `--p2p-node-role`，默认从现有 `--node-role` 派生 `validator_core/full_storage/observer_light`
  - peer record schema 扩展：新增 `deployment_mode`，并把 `node_role` 解析升级为 canonical `validator_core/sentry/relay/full_storage/observer_light`，同时兼容旧 `sequencer/storage/observer` 标签
  - role admission / exposed-surface 校验：runtime config 与 signed peer record 都会拒绝无效 deployment-role 组合，以及 `private/relay_only/validator_hidden` 发布 direct public surface 的配置
- 完成定义:
  - `validator_hidden`、`relay_only` 成为正式配置，不再靠脚本约定
  - validator core 不再要求自身全公网暴露

### P2PARCH-4 / runtime_engineer
- 输入:
  - 统一 transport substrate
- 输出:
  - lane registry
  - consensus/sync/blob/control QoS policy
- 本轮已交付:
  - `NetworkLane` / `NetworkLaneQosClass` / topic+protocol classifier：把 `consensus gossip / sync / blob-state / control` 提升为共享协议层类型，而不是继续散落在 topic / protocol 字符串判断里
  - `capability_lanes` peer record schema：peer record 可显式声明 lane capability；legacy record 若未声明则按 `validator_core/sentry/relay/full_storage/observer_light` 自动回填默认值
  - lane-aware `oasis7_net` substrate：subscription inbox 配额按 lane 区分；req/resp 在 `fetch-commit/fetch-blob` 等路径会优先选择具备对应 lane capability 的 peer record
  - role-aware `oasis7_node` binding：replication/consensus 绑定提升成 traffic lane registry，并对 publish/subscribe/request/serve 四类操作做 `node_role_claim` gate
- 完成定义:
  - blob/state 流量不能拖垮 consensus/control
  - 不同链适配器只绑定 lane，不重写 substrate

### P2PARCH-5 / runtime_engineer
- 输入:
  - P2PARCH-1~4 产出
- 输出:
  - peer scoring
  - diversity policy
  - runtime block artifact substrate
- 本轮已交付:
  - `PeerManagerPolicy` / `PeerManagerPeerHealth` / `PeerManagerHealthIssue` substrate：冻结 `candidate/active/suspect/blocked` health 状态与本地 fail signatures
  - `oasis7_net` libp2p worker 接线：基于 `discovery_sources + active transport path` 计算 peer health snapshot，并把 `single-source active set`、IPv4 `/24`、relay-domain 与 relay budget 超限标成 `suspect`
  - request peer 健康优先级：req/resp 在 lane 过滤后会进一步优先选择 `active/candidate` peer，把 `suspect` 压到最后，并直接排除 `blocked`
  - discovery ingress enforcement：`RoutingUpdated` / rendezvous registration 不再对未校验地址做 speculative dial；只有 validated peer record 进入 `process_discovered_peer_record` 后，才会按 health 决定是否拨号
  - discovery dial dedupe 修正：`suspect/blocked` peer 不再提前写入 `dialed_discovery_addrs`，避免同地址在 record 升级为健康后失去首拨机会
  - active-set quarantine enforcement：已连接的 `suspect` 与已验证 hard-`blocked` peer 会被主动断连，并在 `ConnectionClosed` / `OutgoingConnectionError` 上抑制 failover / retry，避免 transport 状态机把 quarantined peer 立即拉回 active set
  - admitted active-set health view：对外发布的 peer health 会先剔除未准入 active peer，不再让待校验或 quarantined 连接在同轮 `/24`、relay-domain、relay budget 统计中瞬时污染其他健康 peer
  - operator / ASN diversity inputs：chain runtime 默认 peer record 现已支持 `source_operator/source_asn`，peer manager 会归一化标签并把 operator/ASN concentration 纳入 health snapshot
  - 正式阈值与 block 条件：`operator / ASN / subnet / relay-domain` 现统一采用 `>25% => suspect`、`>=50% => blocked` 的默认判定，并在 issue payload 中回传触发阈值
  - process-durable block artifact：runtime 会为 hard-`blocked` peer 维护跨 recompute 的 artifact，记录 `peer_id/status/issues/path/operator/asn/first_blocked_at/last_blocked_at/last_cleared_at`，供 debug/release 取证
- 完成定义:
  - operator/ASN/subnet/relay-domain 多样性具备正式阈值与 block 条件
  - hard-`blocked` peer 至少产出可追溯的 block artifact，并能被后续 QA / release-gate 消费
  - `P2PARCH-5` 的 required 验证仅要求 runtime 定向单测、文档与 PM 追踪闭环；mixed-topology / claims gate 证据转入 `P2PARCH-6/7`
  - 2026-04-15 runtime follow-up 已新增 `--traffic-profile triad_low_traffic`，把本机 observer + 两台 ECS 这类固定 bootstrap triad 的无效控制面流量收敛为显式 profile，而不是要求 operator 通过切换 `storage_profile` 去交换 release 安全语义。

### P2PARCH-6 / qa_engineer
- 输入:
  - P2PARCH-2~5 reachability / role / policy 结果
- 输出:
  - mixed-topology matrix
  - anti-eclipse / anti-spam fail signatures
  - chaos / failover / relay exhaustion 证据模板
- 本轮已交付:
  - `scripts/p2p-mixed-topology-matrix.sh`：统一输出 `required` exact + `full` proxy 两档 matrix summary
  - `scripts/p2p-mixed-topology-matrix-smoke.sh`：对 matrix case 装配与 summary 结构做快速 smoke
  - `testing-manual.md` S9B：补 mixed-topology 推荐命令、通过标准、产物路径、`proxy` 边界口径，以及 `evidence_contract/external_evidence` 机器可读字段
  - latest full-tier evidence：`doc/testing/evidence/p2p-mixed-topology-validation-matrix-2026-04-07.md` 已固化 7 个 exact case 全通过、2 个 proxy case 真实执行但失败的当前 blocker
  - `scripts/p2p-real-env-triad-snapshot.sh`：新增真实 triad 采样入口，统一抓取本机 observer、ECS sequencer、ECS storage 的 `systemctl + /healthz + /v1/chain/status`，并把 real-env blocker 收口成可复跑 summary
  - latest real-env evidence：`doc/testing/evidence/p2p-real-env-triad-reconfirm-2026-04-08.md` 已固化带远端认证的 same-window 三节点复采；当前结论为 `blocked`，但 blocker 已精确收敛为 `sequencer_committed_height_zero + sequencer_execution_stale_height`，而不是“云端整体不可见”或“observer 仍未接入”
  - matrix/runtime follow-up：proxy case 不再依赖预编译 binary 或默认 561x 端口段；`oasis7_chain_runtime` 也不再对 `observer` 无条件启用 `feedback_p2p`，避免与 `P2PARCH-4` lane gate 冲突
  - 当前已知可复用的真实环境：`1` 个本机节点 + `2` 个阿里云节点。该环境可作为后续 `P2PARCH-6` real-run 的第一批真机拓扑输入，用于补强“本机节点 + cloud public” mixed-topology drill、bootstrap poisoning、sentry loss、path failover、relay budget 与 release-proxy 类回归，而不必只停留在本地 proxy 近似。
  - 2026-04-07 real-run follow-up：private observer triad 已通过“reverse UDP hello seeding + mixed-root validator signer override + replication remote-writer allowlist”闭环 reachability。真实 observer 当前可从云端 storage peer 连续回放 commit，高位样本为 `latest_height=88 / committed_height=88 / network_committed_height=58086 / known_peer_heads=1 / last_error=null`；同窗口 residual blocker 仅剩 `triad-sequencer-a` 的 `execution driver received stale height`。
  - 2026-04-08 observer local follow-up：`doc/testing/evidence/p2p-real-env-observer-gap-sync-followup-2026-04-08.md` 说明当前本机 observer 已不再复现 `known_peer_heads=0 / network_committed_height=0`，而是稳定进入 `known_peer_heads=1 / network_committed_height>0` 状态；当前更贴近真实环境的 blocker 已切换成 replication gap-sync `blob not found`。同轮 runtime 已把 gap-sync `fetch-blob` 升级为 provider-aware DHT 路由，并在 provider route 暂时不可用时回退到普通 lane-aware request。由于该轮未带远端 ECS 凭据，same-window triad 仍需后续带认证复核。
  - 2026-04-08 blob-availability rollout follow-up：`doc/testing/evidence/p2p-real-env-triad-blob-availability-root-cause-2026-04-08.md` 已记录第二轮真实环境 rollout。当前三节点统一运行 release=`95ae1e3ee604-blob-route-fallback-20260408` / sha256=`0179d52afb91355821dcfbeb94c83c7bb10eb174fe1d81d41fbf16d27b26329a`；observer 已恢复 `last_error=null` 并把 `committed_height` 从 `16187` 推到 `16237`，storage 也把 `replication_commit_messages/store-blobs` 增长到 `1056/1056`。sequencer 旧的 `fetch-blob NetworkProtocolUnavailable` 已消失，但仍停在 `storage challenge gate network blob not found`，说明 triad residual 已进一步收敛到“storage 顺序回补太慢，sequencer challenge gate 只抽最新 blob”的 bootstrap 语义缺口。
  - 2026-04-08 challenge-gate fallback rollout：同一 evidence 已补记第三轮真实环境 rollout。当前三节点统一运行 release=`95ae1e3ee604-challenge-gate-fallback-20260408` / sha256=`a2cb5191cdb58cfa0b430369e0220666b5d18e22f4cf58b5b0d1a220f1370fea`；observer 在 60 秒窗口内继续把 `committed_height` 从 `17062 -> 17187` 并保持 `last_error=null`，storage 继续把 `replication_commit_messages/store-blobs` 从 `2003/2003 -> 2347/2347` 向前推进，但 `fetch-commit NetworkProtocolUnavailable` 仍让它停在 `committed_height=0 / network_committed_height=0`；sequencer 在同窗口内已从 `57536 -> 57538` 重新出块且 `last_error=null`，未再采到 `storage challenge gate`。当前 triad blocker 已进一步转移到 storage `fetch-commit` 可用性与 peer-head 重收敛，而不再是 challenge gate 抽样语义本身。
  - 2026-04-08 route/candidate follow-up：同一 evidence 已继续补记两轮实机 rollout。`95ae1e3ee604-inbound-endpoint-route-sanitize-20260408` / `sha256=b84b551e087a1e2b47dde4d8d62a71fc0100cc3980d94379e633d1c53657a6e6` 已修平 listener 入站临时源端口污染 Kademlia/transport route 的问题，实机复采确认 sequencer 与 storage 可以重新恢复彼此直连；`95ae1e3ee604-unsupported-peer-no-fallback-20260408` / `sha256=98b1b99878ba271af63ba4d5e72be1d6a42073e84a383d6d2012a30fb2e3c2de` 已阻止 runtime 把已知 `unsupported` 的 observer 重新拿来兜底做 `fetch-commit/fetch-blob` 请求。最新 same-window 失败签名已从误导性的 `fetch-commit NetworkProtocolUnavailable` 收敛成更准确的 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`，说明当前 real-env blocker 已进一步收缩到 startup ordering / retry window，而不是 observer lane capability、handler 注册或 transport route truth。
- 完成定义:
  - 家宽 / NAT / CGNAT / cloud mixed topology 均有 required/full 套件
  - `P2PARCH-5` runtime substrate 的 anti-eclipse / fail-signature 行为已由 required/full evidence 固化
  - 仅有 `1` 本机 + `2` 阿里云的真实环境时，可以推进 `P2PARCH-6` 的真实多节点证据，但除非额外补齐 NAT 类型、CGNAT、独立 operator/ASN 或 dedicated sentry/NAT lab 证据，否则不得把这组环境误写成 full truth 已覆盖全部边界

### P2PARCH-7 / producer_system_designer + liveops_community + qa_engineer
- 输入:
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - P2PARCH-6 mixed-topology evidence
- 输出:
  - shared-network mixed-topology release gate
  - claims allowlist / denylist 更新
- 完成定义:
  - `P2PARCH-5` block artifact 若要进入 release truth，必须在本任务内被升级为 gate-consumable evidence contract 或显式 denylist 结论
  - 未完成 mixed-topology shared-network 证据前，不得宣称 public-chain-grade P2P 已落地
  - 当前 `1` 本机 + `2` 阿里云的真实环境可作为 `P2PARCH-7` 的前置输入或 partial evidence，但单凭这组环境并不足以直接替代 shared-window shared-network gate、producer/QA pass-uplift decision ref，或 dedicated sentry/NAT lab 级更强 truth

### P2PARCH-8 / producer_system_designer
- 输入:
  - 本专题 deployment mode / role policy 目标态
  - 用户希望“全覆盖 + 默认完全自动”的部署 UX 约束
- 输出:
  - 用户层 `2~3` 档简单模式定义
  - 自动探测默认值与高风险职责确认边界
- 本轮已交付:
  - 冻结 `自动加入 / 私有安全 / 公网入口` 三档用户可见模式
  - 明确底层继续保留 `deployment_mode/node_role` 正式语义，不把安全边界直接暴露给普通用户
  - 明确默认行为是基于公网/NAT/打洞结果自动推荐，只有涉及 `公网入口` 等高风险职责时才允许显式确认或高级覆盖
- 完成定义:
  - 普通用户不必在默认路径上手动选择底层正式角色

### P2PARCH-9 / runtime_engineer + viewer_engineer
- 输入:
  - P2PARCH-1~4 reachability / role / lane substrate
  - P2PARCH-8 用户层模式定义
- 输出:
  - 自动探测到用户模式的映射器
  - 高风险职责确认 / 高级设置覆盖
- 本轮已交付:
  - `oasis7_node` 新增用户层模式、reachability auto-detection、recommendation/effective-policy contract，并把 `public_entry` 自动升级收口为显式确认门
  - chain runtime CLI/status payload 已能承载 requested/recommended/effective user mode 与探测依据，且在未显式传入原始 `deployment_mode/node_role` 时默认走用户模式推荐
  - game launcher / web launcher / launcher UI schema 已统一透传 `chain_p2p_user_mode` 与 `chain_p2p_accept_public_entry`，为后续 viewer UX 接推荐态与确认态留好接口
  - runtime status/recommender 现在会在 CLI 未显式覆盖对应字段时，自动吸收 libp2p live AutoNAT status、confirmed external direct addr / public-port reachability、relay reservation、DCUtR success/failure 与 active transport path kind，作为 `auto_join / private_safe / public_entry` 默认推荐依据；`public_entry` 不再只靠 relay / hole-punch hint 做推断
  - runtime follow-up 已补 stale relay reservation 收口：live snapshot 现在会跟随 relayed listen addr 的 `new/expired` 生命周期重算 `relay_reservation_active`，不再把旧 reservation 证据永久滞留在 status/recommender
  - chain status 已把“实际运行态”与“当前 live 推荐态”拆开：保留 `effective_user_mode` 表示按实时 reachability snapshot 计算出的当前有效推荐态，并新增 `applied_effective_user_mode` 表示 runtime 在启动时实际应用的用户模式；若节点是通过底层 `deployment_mode/node_role` 显式 override 启动，则继续以 `deployment_mode/node_role_claim` 作为实际运行态真值
  - `viewer_engineer` 已将 chain status 的 P2P recommendation payload 接入 launcher：`oasis7_web_launcher` 会把 requested/recommended/applied user mode、reachability evidence、底层 role mapping 一起透传给 `oasis7_client_launcher`
  - `oasis7_client_launcher` 已新增用户可见 P2P 模式卡片，明确区分“请求模式 / 自动推荐 / 实际运行态”，并显示 reachability、hole-punch、relay、probe-stable 与 rationale 摘要
  - chain runtime `/v1/chain/status` 现已额外暴露 `autonat_status/public_port_reachability/observed_public_addr/confirmed_external_direct_addrs`，便于 launcher / viewer 或运维脚本审计节点为何被判定为 `private_safe` 或 `public_entry`
  - launcher 高级配置已把 `chain_p2p_user_mode` 收口为 `auto_join / private_safe / public_entry` 三档 simple modes，并对 `public_entry` 增加显式确认门；未确认时拒绝启动高风险模式
  - `test_tier_full` 当前已以 `doc/testing/evidence/p2p-user-mode-launcher-ux-2026-04-07.md` 固化 launcher UX、confirm/reject 流程和用户模式语义对账
- 遗留:
  - CLI detection hint 继续保留为显式 override 通道；后续若 launcher/viewer 需要把 `autonat_status/public_port_reachability/observed_public_addr` 单独做成更强提示文案或图形标签，可在现有 status payload 基础上继续增强 UI，而不必再补底层探测链路
- 完成定义:
  - 系统可在默认启动路径中自动选择用户模式，并给出可审计的探测依据
  - launcher/viewer 必须提供 `public_entry` 接受 / 拒绝路径，并保证最终运行态与用户确认结果一致可读

## 依赖
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- `doc/p2p/network/readme-p1-network-production-hardening.prd.md`
- `doc/p2p/node/node-net-stack-unification-readme.prd.md`
- `testing-manual.md`

## 验收命令（本轮）
- `rg -n "validator_hidden|relay_only|signed peer record|AutoNAT|hole punch|relay reservation|gossip plane|blob-state plane|anti-eclipse|tree broadcast|committee direct|自动加入|私有安全|公网入口|deployment_mode|node_role|显式确认" doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md doc/p2p/README.md testing-manual.md`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher -- --nocapture`
- `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前状态: active
- 下一步: 先消化 `P2PARCH-6` latest full proxy failure signatures；当前 real run 已把 blocker 进一步收敛到 storage restart 后 startup window 内的 `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`、随之导致的 `committed_height/network_committed_height` 长时间不收敛，以及 sequencer `known_peer_heads=0` 的 peer-head 传播残留。在这些签名修平前继续保持 `full_proxy_ready=false`。
- 下一步: `P2PARCH-7` 继续保持 `partial`；same-window refs 已可作为 matrix 输入，但若要把 shared-devnet mixed-topology lane 提升为 `pass`，除修平 proxy failure signatures外还必须固定 producer/QA 审计通过的 pass-uplift decision ref。
- 最近更新: 2026-04-08
