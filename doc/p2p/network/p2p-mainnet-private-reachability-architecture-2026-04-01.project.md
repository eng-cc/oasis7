# oasis7 主链级非全公网 P2P 覆盖网络架构（项目管理文档）

- 对应设计文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md`
- 对应需求文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`

审计轮次: 1
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
- [ ] P2PARCH-5 (PRD-P2P-024-B/E) [test_tier_required + test_tier_full]: `runtime_engineer` + `qa_engineer` 落 peer manager、anti-eclipse、diversity、relay budget 与 quarantine 信号。
  进行中第二个切片：在首个 peer-manager substrate 基础上，`oasis7_net` 现在开始把 `suspect` 与已验证的 hard-`blocked` 升级成 active-set quarantine enforcement。已连接 peer 一旦因 source diversity、subnet、relay-domain 或 relay budget 规则进入 quarantine，runtime 会主动断连，并在 `ConnectionClosed` / `OutgoingConnectionError` 路径抑制 failover / retry，避免 quarantined peer 立刻被本地 transport 状态机拉回；同轮 health 统计也会剔除未准入 active peer，避免瞬时污染其他健康 peer。
- [ ] P2PARCH-6 (PRD-P2P-024-D/E) [test_tier_required + test_tier_full]: `qa_engineer` 建立 mixed-topology 套件，覆盖家宽/NAT、CGNAT、relay exhaustion、sentry loss、bootstrap poisoning、path failover。
  已落首个 executable mixed-topology matrix slice：`scripts/p2p-mixed-topology-matrix.sh` 会把 `private/validator_hidden/relay_only` role boundary、bootstrap poisoning、relay-budget detection 与 path failover 收成 `required` exact cases，并把 triad/triad_distributed 的 disconnect/restart/release-chaos 收成 `full` proxy cases；matrix 明确输出 `summary.json/md` 与 per-case command/log 目录，并显式标注当前 `proxy` case 只是 sentry-loss / mixed-topology live recovery 的近似 drill，不等价于 dedicated sentry/NAT lab。`full` live evidence 仍待后续继续执行。
- [ ] P2PARCH-7 (PRD-P2P-024-E) [test_tier_required + test_tier_full]: `producer_system_designer` + `liveops_community` + `qa_engineer` 把 shared-network / release-train / claim gate 升级为 mixed-topology 正式门禁。
  已把 mixed-topology lane 升级为 shared-network required gate：`shared-network-track-gate.sh` 现在要求 `shared_devnet/mixed_topology_baseline`、`staging/mixed_topology_rehearsal`、`canary/mixed_topology_claim_review` 三条显式 lane；`shared-devnet-rehearsal.sh` 也会自动生成 mixed-topology gate note，并默认把仅有 `P2PARCH-6` matrix baseline 的窗口保持在 `partial`。

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
  - `P2PARCH-6` 已落首个 mixed-topology validation matrix slice：QA 现在可用一个统一脚本同时编排 `required` exact cases（private/NAT policy、validator_hidden、relay_only、bootstrap poisoning、relay-budget detection、path failover）和 `full` proxy cases（triad/triad_distributed ingress-loss release drills），并把 `proxy != dedicated sentry/NAT lab` 作为证据口径显式写入 summary。
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

### P2PARCH-5 / runtime_engineer + qa_engineer
- 输入:
  - P2PARCH-1~4 产出
- 输出:
  - peer scoring
  - diversity policy
  - anti-eclipse / anti-spam fail signatures
- 本轮已交付:
  - `PeerManagerPolicy` / `PeerManagerPeerHealth` / `PeerManagerHealthIssue` substrate：冻结 `candidate/active/suspect/blocked` health 状态与本地 fail signatures
  - `oasis7_net` libp2p worker 接线：基于 `discovery_sources + active transport path` 计算 peer health snapshot，并把 `single-source active set`、IPv4 `/24`、relay-domain 与 relay budget 超限标成 `suspect`
  - request peer 健康优先级：req/resp 在 lane 过滤后会进一步优先选择 `active/candidate` peer，把 `suspect` 压到最后，并直接排除 `blocked`
  - discovery ingress enforcement：`RoutingUpdated` / rendezvous registration 不再对未校验地址做 speculative dial；只有 validated peer record 进入 `process_discovered_peer_record` 后，才会按 health 决定是否拨号
  - discovery dial dedupe 修正：`suspect/blocked` peer 不再提前写入 `dialed_discovery_addrs`，避免同地址在 record 升级为健康后失去首拨机会
  - active-set quarantine enforcement：已连接的 `suspect` 与已验证 hard-`blocked` peer 会被主动断连，并在 `ConnectionClosed` / `OutgoingConnectionError` 上抑制 failover / retry，避免 transport 状态机把 quarantined peer 立即拉回 active set
  - admitted active-set health view：对外发布的 peer health 会先剔除未准入 active peer，不再让待校验或 quarantined 连接在同轮 `/24`、relay-domain、relay budget 统计中瞬时污染其他健康 peer
- 仍待补齐:
  - operator / ASN 多样性输入与阈值
  - `blocked` 到持久 banlist / release-gate 级 block artifact 的升级
  - `qa_engineer` mixed-topology 与 fail-signature required/full 套件
- 完成定义:
  - operator/ASN/subnet/relay-domain 多样性具备正式阈值与 block 条件

### P2PARCH-6 / qa_engineer
- 输入:
  - P2PARCH-2~5 reachability / role / policy 结果
- 输出:
  - mixed-topology matrix
  - chaos / failover / relay exhaustion 证据模板
- 本轮已交付:
  - `scripts/p2p-mixed-topology-matrix.sh`：统一输出 `required` exact + `full` proxy 两档 matrix summary
  - `scripts/p2p-mixed-topology-matrix-smoke.sh`：对 matrix case 装配与 summary 结构做快速 smoke
  - `testing-manual.md` S9B：补 mixed-topology 推荐命令、通过标准、产物路径与 `proxy` 边界口径
- 完成定义:
  - 家宽 / NAT / CGNAT / cloud mixed topology 均有 required/full 套件

### P2PARCH-7 / producer_system_designer + liveops_community + qa_engineer
- 输入:
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - P2PARCH-6 mixed-topology evidence
- 输出:
  - shared-network mixed-topology release gate
  - claims allowlist / denylist 更新
- 完成定义:
  - 未完成 mixed-topology shared-network 证据前，不得宣称 public-chain-grade P2P 已落地

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
- `rg -n "validator_hidden|relay_only|signed peer record|AutoNAT|hole punch|relay reservation|gossip plane|blob-state plane|anti-eclipse|tree broadcast|committee direct" doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md doc/p2p/prd.md doc/p2p/project.md doc/p2p/prd.index.md doc/p2p/README.md`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前状态: active
- 下一步: 继续执行 `P2PARCH-7` 的 shared-network mixed-topology live evidence，把 `shared_devnet` 的 mixed-topology lane 从 baseline 提升到 same-window `pass`，再决定是否需要 dedicated sentry/NAT lab 来替换当前 proxy live drills。
- 最近更新: 2026-04-03
