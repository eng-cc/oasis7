# oasis7 主链级非全公网 P2P 覆盖网络架构（2026-04-01）

- 对应设计文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md`
- 对应项目管理文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md`

审计轮次: 1
## 1. Executive Summary
- Problem Statement: 当前 triad 实测已经证明，家宽 / NAT / CGNAT / 企业内网节点无法稳定接收入站 UDP 时，现有“节点最好有公网地址”假设会直接把本机、边缘机房和大量运营环境排除在主路径之外。若继续把公网可达当成默认前提，oasis7 很难对标公共主链常见的 mixed-topology 现实。
- Proposed Solution: 冻结一套 public-chain-grade 的 P2P 目标态，把 `identity`、`addressability`、`transport`、`discovery`、`relay/overlay` 与 `consensus/data plane` 解耦，使 validator / full / storage / observer 节点在 `public / hybrid / private / relay_only / validator_hidden` 多种部署模式下都能成为一等公民。
- Success Criteria:
  - SC-1: 目标态明确支持 `public / hybrid / private / relay_only / validator_hidden` 五种部署模式，且 validator 不再以“必须有公网 IP”作为前置条件。
  - SC-2: Reachability 设计明确包含 `AutoNAT + hole punching + relay reservation + signed peer record` 四条能力，而不是继续依赖静态 UDP 邻居。
  - SC-3: 协议层明确拆出 `gossip plane / sync plane / blob-state plane / control plane`，不同链型只替换上层适配器，不重写底层可达性框架。
  - SC-4: 安全边界明确冻结 `validator core / sentry / relay / full-storage / observer-light` 的角色和密钥边界，禁止 relay/browser/operator public plane 持有长期共识 signer。
  - SC-5: 文档明确给出 anti-eclipse、anti-spam、peer diversity、relay budget 与 shared-network 验证要求，后续实现可直接据此拆任务和测门禁。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要一套不退回 MVP、直接对标公共主链的网络目标态。
  - `runtime_engineer`: 需要知道底层 P2P core 应该抽象到什么层，哪些能力必须做成框架层，而不是继续散落在 runtime 业务里。
  - validator / sentry operator: 需要明确什么时候节点必须公开暴露，什么时候只需要 outbound reachability。
  - home / CGNAT / enterprise operator: 需要在没有公网 IP 的条件下仍然能加入网络、同步状态并承担受限角色。
  - `qa_engineer`: 需要把 mixed-topology、relay fallback、anti-eclipse 和 shared-network 变成正式验证矩阵。
- User Scenarios & Frequency:
  - 每次有人提出“某个节点没有公网 IP，是否还能成为正式节点”时，先看本专题。
  - 每次设计 validator / sentry / relay 部署形态时，先看本专题角色边界。
  - 每次网络层计划从点状补丁进入框架重构时，先以本专题冻结目标态和非目标。
  - 每次 shared network / release train / chaos drill 设计新门禁时，把本专题当作流量分层和 reachability 真值。
- User Stories:
  - PRD-P2P-024-A: As a `producer_system_designer`, I want one public-chain-grade private-reachability architecture, so that oasis7 不会因为家宽/NAT 现实被迫退回“全员公网节点”假设。
  - PRD-P2P-024-B: As a `runtime_engineer`, I want identity, discovery, transport and relay semantics separated from consensus logic, so that the framework can support different main-chain data planes without重写底层可达性。
  - PRD-P2P-024-C: As a validator operator, I want one explicit `validator core / sentry / relay` model, so that signing节点可以隐藏在私网后面，公开面只承担转发和抗攻击吸收。
  - PRD-P2P-024-D: As a private-node operator, I want hole punching / relay / overlay fallback frozen as first-class paths, so that no-public-IP 节点仍可稳定同步与参与网络。
  - PRD-P2P-024-E: As a `qa_engineer`, I want anti-eclipse, peer diversity and mixed-topology test gates defined up front, so that public-chain-grade claims are tied to evidence rather than直觉。
- Critical User Flows:
  1. Flow-P2P-PRA-001: `private validator boot -> outbound connect to bootstrap/sentry -> reachability service classifies self as private -> reserve relay or attach sentry -> advertise signed peer record without exposing home IP -> join consensus/control lanes`
  2. Flow-P2P-PRA-002: `private full/storage node boot -> attempt direct reachability -> hole punch succeeds则升级 direct path；失败则保留 relay path -> sync plane 仍可追块/追状态 -> gossip 只按本角色订阅必要 topic`
  3. Flow-P2P-PRA-003: `public node receives peer record -> verify chain/domain/signature -> apply diversity policy -> assign gossip/sync/data lanes -> monitor peer score and path quality -> re-route on relay loss or direct-path recovery`
  4. Flow-P2P-PRA-004: `bootstrap poisoning / eclipse suspicion -> peer manager detects discovery-source集中、ASN集中或 relay abuse -> downgrade suspicious peers -> force rebootstrap from independent anchors -> emit block signal for release gate`
- Functional Specification Matrix:

| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Peer identity 与 record | `peer_id/node_identity_key/chain_id/role_mask/reachability_class/public_addrs/relay_addrs/ttl/signature` | 节点发布带有效期的签名 peer record | `draft -> published -> refreshed -> expired/revoked` | record 必须链内域分离签名，过期或域不匹配即拒绝 | 只有节点身份持有者可发布；consensus signer 不直接暴露 |
| Reachability service | `observed_addr/autonat_status/hole_punch_status/relay_reservation/path_quality` | 探测 direct、尝试打洞、预留 relay、维护路径排序 | `unknown -> direct/private/relay_only -> degraded/recovered` | direct 优先于 punched，punched 优先于 relay；不能打洞时自动降级 relay | 节点本地决策；relay 只提供转发，不授予签名权限 |
| Discovery fabric | `bootnodes/dht_namespace/rendezvous_topic/peer_record_cache/source_diversity` | 从 bootnode、DHT、rendezvous 与静态 allowlist 聚合候选 peer | `seeded -> converging -> healthy/degraded` | 至少保留两类独立 discovery source；单源集中不得视为 healthy | bootnode/relay 可公开；validator core 可只做 consumer |
| Transport abstraction | `transport_id/directness/security/mux/qos_class/max_streams` | 在 direct、hole-punched、relay 路径上复用统一流接口 | `dialing -> established -> draining -> closed` | QUIC 为主、TCP/Noise 为回退；UDP 只可作为加速，不得成为唯一真值链路 | transport key 可轮换；长期 signer 不进入 transport session |
| Role policy | `deployment_mode/node_role/sentry_set/relay_budget/exposed_surface` | 根据角色限制订阅、入站、转发与公开面 | `declared -> admitted -> enforced` | validator_hidden 至少配 2 条独立 ingress path；observer 不得请求 validator-private RPC | 角色由 operator 配置并被 peer manager 强制执行 |
| Traffic lanes | `lane_id/topic_or_stream/qos/peer_subset/replay_policy` | 分离 gossip、sync、blob/state、control 流量 | `registered -> active -> throttled/quarantined` | consensus lane 优先低抖动；blob/state lane 独立限速，不得拖垮 finality | 不同角色只开放最小必要 lane |
| Peer manager / anti-eclipse | `score/source_asn/source_operator/subnet_bucket/relay_dependence/misbehavior` | 评分、淘汰、重连、路径切换与 quarantine | `candidate -> active -> suspect -> blocked` | 同一 operator、同一 `/24`、同一 relay-domain 占比超过阈值即降权 | 安全策略由本地 peer manager 执行，不能由远端覆盖 |
- Acceptance Criteria:
  - AC-1: 本专题必须明确声明“公网 IP 不是 validator/full/storage/observer 参与网络的通用前置条件”，只是一种 reachability 优势。
  - AC-2: 本专题必须冻结五种部署模式：`public`、`hybrid`、`private`、`relay_only`、`validator_hidden`，并说明各自公开面与限制。
  - AC-3: 本专题必须冻结 `validator core / sentry / relay / full-storage / observer-light` 五类角色，并明确 validator core 可完全不暴露公网入站。
  - AC-4: 本专题必须冻结一套统一 P2P core：`peer record + discovery + reachability + transport + peer manager`，业务层不得再直接依赖静态 UDP peer 列表作为唯一主路径。
  - AC-5: 本专题必须明确写出 reachability 顺序：`direct -> hole-punched -> relay`，以及 `AutoNAT / hole punching / relay reservation` 三者的关系。
  - AC-6: 本专题必须明确写出 `gossip plane / sync plane / blob-state plane / control plane` 分离，且不同链型只替换上层适配器。
  - AC-7: 本专题必须明确写出主链适配器边界，至少覆盖 `mesh gossip`、`committee direct`、`tree broadcast` 与 `blob availability` 四类数据面模式。
  - AC-8: 本专题必须明确写出 anti-eclipse / anti-spam 基线，包括 discovery-source 多样性、ASN/subnet/operator 多样性、relay budget 与 path quarantine。
  - AC-9: 本专题必须明确写出 key boundary：transport/session key、node identity key、consensus signer、governance signer 彼此分离；relay、browser 和 public control plane 不得持有长期 signer 真值。
  - AC-10: 对应 `project.md` 必须把实现拆成 identity、transport、reachability、role policy、traffic lanes、shared-network validation 等 workstreams，而不是只停留在概念描述。
  - AC-11: `doc/p2p/prd.md`、`doc/p2p/project.md`、`doc/p2p/prd.index.md` 与 `doc/p2p/README.md` 必须接入本专题，形成模块级追踪链。
- Non-Goals:
  - 不在本专题内绑定单一网络库实现，不把 “libp2p / 自研 / 混合” 之一提前冻结成唯一方案。
  - 不把当前目标降级成“能连起来就行”的家宽补丁；本专题讨论的是公共主链级目标态。
  - 不在本专题内定义单一链型的共识算法细节；这里只冻结链无关的 P2P substrate 和数据面适配边界。
  - 不把 browser wallet、production signer custody 或 shared-network 实跑证据在此专题内冒充已实现。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 目标态采用四层框架。第一层是 `identity/discovery`，用带 TTL 的签名 peer record 把节点身份从瞬时地址中解耦；第二层是 `reachability/transport`，统一 direct、hole-punched 与 relay 路径，并对 QUIC/TCP/Noise/mux 做抽象；第三层是 `traffic lanes`，把 consensus gossip、header/block sync、state/blob transfer、control/heartbeat 分离；第四层是 `chain adapters`，让不同公共主链风格的数据面挂到同一底层 P2P core 上，而不是各自重造 discoverability 和 NAT 穿透。
- Integration Points:
  - `doc/p2p/prd.md`
  - `doc/p2p/project.md`
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
  - `doc/p2p/network/readme-p1-network-production-hardening.prd.md`
  - `doc/p2p/node/node-net-stack-unification-readme.prd.md`
  - `doc/p2p/node/node-replication-libp2p-migration.prd.md`
  - `doc/p2p/node/node-distfs-replication-network-closure.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 对称 NAT / CGNAT 无法打洞：节点必须自动降级为 `relay_only` 或 `validator_hidden + sentry`，而不是直接失去网络身份。
  - relay reservation 过期或 relay 宕机：peer manager 必须主动切换备用 relay/sentry，并降低该 relay-domain 权重。
  - 公开地址漂移：peer record 需快速 refresh；过期地址不得继续作为优选路径。
  - bootnode / rendezvous 污染：若候选 peer 集中于单一 operator、ASN 或 relay-domain，必须触发 suspect 状态和重引导。
  - private validator 误暴露：若 validator core 被配置为公开暴露共识面，系统必须发出高严重度运维警报。
  - relay 滥用与流量挤占：blob/state lane 必须有独立带宽预算，不能拖垮 consensus/control lane。
  - DHT 或 rendezvous 部分失效：已建立连接不得因单一 discovery 子系统故障而全量重置。
  - 角色越权：observer/light 节点请求 validator-private lane、control plane 或 signer path 时必须被拒绝并记分。
- Non-Functional Requirements:
  - NFR-P2P-PRA-1: `validator_hidden` 模式必须允许“零公网入站 + 至少两条独立 outbound ingress path”加入网络；这两条路径可以是 `sentry + relay` 或 `relay + relay`，但不得只有单一路径。
  - NFR-P2P-PRA-2: 网络健康目标必须要求节点在启动后 `60s` 内收敛到足够的 discovery 视图；`public/hybrid` 节点至少看到 `8` 个候选 peer，`private/validator_hidden` 节点至少建立 `4` 条有效工作路径。
  - NFR-P2P-PRA-3: relay fallback 的控制面时延中位数不得超过同区域 direct path 的 `2x`；若超过，系统必须将该 relay-domain 标记为降级。
  - NFR-P2P-PRA-4: active peer set 中，来自同一 operator、同一 ASN 或同一 `/24` 的占比默认不得超过 `25%`；超限必须降权或阻断。
  - NFR-P2P-PRA-5: signed peer record 的默认 TTL 不得超过 `1h`，reachability 状态变化后必须支持快速 refresh/revoke。
  - NFR-P2P-PRA-6: transport/session key、node identity key 与 consensus/governance signer 必须逻辑隔离；任何 relay 或 browser surface 泄露 transport key 都不得等价为节点治理权或出块权泄露。
  - NFR-P2P-PRA-7: gossip、sync、blob/state、control 四条 lane 必须支持独立的 rate limit、peer subset 和 quarantine 规则。
- Security & Privacy:
  - 所有 peer record 必须做链域隔离签名，防止跨链、跨环境重放。
  - transport 默认要求加密和双向身份确认；未认证链路不得进入 consensus/control lane。
  - 私网节点默认不公开内网地址；只有 operator 显式允许时才向受信对等方暴露私有地址提示。
  - relay 只转发，不解释业务语义，也不持有长期 signer；任何 relay route 都不能提升调用者权限。
  - sentry 与 validator core 必须分离身份与密钥；sentry 被打爆不应直接等价为 validator signer 暴露。

## 5. Risks & Roadmap
- Phased Rollout:
  - Phase 0: 冻结目标态文档、角色边界、lane taxonomy 与 mixed-topology 验证口径。
  - Phase 1: 落 peer record / discovery / reachability service，把现有静态 peer 与双路径网络接线收敛为统一 substrate。
  - Phase 2: 落 relay/sentry/validator_hidden 与 traffic lanes，把 consensus、sync、blob/state、control 分离成独立 QoS。
  - Phase 3: 落 anti-eclipse、peer scoring、shared-network mixed-topology、chaos/release-train 验证，之后才允许更高 public-chain maturity claims。
- Technical Risks:
  - 风险-1: 若继续在业务层保留“直接发 UDP 给静态地址”的逃生路径，框架层的 role policy 与 reachability 会长期失真。
  - 风险-2: 若把 relay 设计成“万能兜底”却不做预算和多样性限制，网络会从“全公网依赖”退化成“单 relay 依赖”。
  - 风险-3: 若不把 validator core 与 sentry 身份/密钥彻底拆开，私网 validator 的安全收益会被公开面抵消。
  - 风险-4: 若不为不同链型定义数据面适配边界，后续高吞吐链型需求会再次迫使底层 substrate 分叉。

## 6. Validation & Decision Record
- Test Plan & Traceability:

| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-024-A | P2PARCH-0/1 | `test_tier_required` | 文档建档、模块入口映射、peer record / deployment mode 术语冻结 | `doc/p2p` 主边界与架构口径 |
| PRD-P2P-024-B | P2PARCH-1/2/4 | `test_tier_required + test_tier_full` | transport abstraction、lane split、adapter boundary 设计与 mixed-topology 集成回归 | runtime / network substrate |
| PRD-P2P-024-C | P2PARCH-2/3 | `test_tier_required + test_tier_full` | validator_hidden、sentry、relay、full-storage、observer 角色矩阵与权限回归 | operator 部署与安全边界 |
| PRD-P2P-024-D | P2PARCH-2/3/6 | `test_tier_required + test_tier_full` | AutoNAT / hole punch / relay fallback / relay exhaustion / path failover 套件 | 私网节点可达性与恢复 |
| PRD-P2P-024-E | P2PARCH-5/6/7 | `test_tier_required + test_tier_full` | anti-eclipse、diversity、shared-network mixed-topology、chaos/release-train 证据 | public-chain-grade claims gate |
- Decision Log:

| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-PRA-001 | 采用 `identity != address` 的 signed peer record 模型 | 继续把节点地址写死在静态 peer 列表 | 家宽/NAT/地址漂移条件下，静态地址不是稳定身份。 |
| DEC-P2P-PRA-002 | Reachability 采用 `direct -> hole-punched -> relay` 多路径排序 | 只做 direct，失败即离网 | 公共主链现实部署天然包含 NAT、CGNAT 与防火墙环境。 |
| DEC-P2P-PRA-003 | validator 采用 `validator core + sentry/relay` 模型 | 要求 validator 自身全公网暴露 | 公开入站面与长期 signer 不应强耦合。 |
| DEC-P2P-PRA-004 | 将链差异限制在数据面适配器 | 每种链型各自重做 discovery/reachability | 发现、可达性、加密和 peer scoring 是跨链共性能力。 |
| DEC-P2P-PRA-005 | 把 anti-eclipse / diversity / relay budget 写成框架层硬约束 | 先实现连通，后续再补安全策略 | 公共主链级 P2P 不能把“安全拓扑”推迟到上线后再补。 |
