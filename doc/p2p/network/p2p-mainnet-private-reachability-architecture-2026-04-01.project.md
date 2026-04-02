# oasis7 主链级非全公网 P2P 覆盖网络架构（项目管理文档）

- 对应设计文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md`
- 对应需求文档: `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] P2PARCH-0 (PRD-P2P-024-A/B/C/D/E) [test_tier_required]: 新建“主链级非全公网 P2P 覆盖网络架构”专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。
- [ ] P2PARCH-1 (PRD-P2P-024-A/B) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 node identity、signed peer record、bootnode/DHT/rendezvous 发现链路，并让业务层不再直接依赖静态 UDP peer truth。
  当前已落地两个 substrate 切片: stable libp2p identity、signed peer record schema + DHT contract、默认 bootstrap/DHT discovery taxonomy，以及 query-driven peer acquisition（DHT discovery query + bootstrap cached peer list/record fallback）；剩余余量主要收敛到 rendezvous 自动化。
- [ ] P2PARCH-2 (PRD-P2P-024-B/D) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 transport abstraction，统一 direct / hole-punched / relay path，并把 QUIC/TCP/Noise/mux 语义冻结到 substrate。
- [ ] P2PARCH-3 (PRD-P2P-024-C/D) [test_tier_required + test_tier_full]: `runtime_engineer` 落 `public / hybrid / private / relay_only / validator_hidden` deployment mode 与 `validator core / sentry / relay / full-storage / observer-light` 角色策略。
- [ ] P2PARCH-4 (PRD-P2P-024-B/C) [test_tier_required + test_tier_full]: `runtime_engineer` 收敛 traffic lanes，把 consensus gossip、sync、blob/state、control 拆成独立 QoS 与 peer subset。
- [ ] P2PARCH-5 (PRD-P2P-024-B/E) [test_tier_required + test_tier_full]: `runtime_engineer` + `qa_engineer` 落 peer manager、anti-eclipse、diversity、relay budget 与 quarantine 信号。
- [ ] P2PARCH-6 (PRD-P2P-024-D/E) [test_tier_required + test_tier_full]: `qa_engineer` 建立 mixed-topology 套件，覆盖家宽/NAT、CGNAT、relay exhaustion、sentry loss、bootstrap poisoning、path failover。
- [ ] P2PARCH-7 (PRD-P2P-024-E) [test_tier_required + test_tier_full]: `producer_system_designer` + `liveops_community` + `qa_engineer` 把 shared-network / release-train / claim gate 升级为 mixed-topology 正式门禁。

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - 覆盖网络架构 verdict: `partial`
- 当前专题结论:
  - 已冻结目标态，不走 “先做一个 NAT patch 的 MVP” 路线。
  - `P2PARCH-1` 已落首个 identity/discovery substrate 切片：runtime 现在会从 node root key 派生稳定 libp2p identity，并发布/校验 signed peer record。
  - `P2PARCH-1` 已补齐 query-driven discovery acquisition：runtime 会周期性刷新 DHT peer discovery，并在 provider 查询只返回 self 或未命中 peer record 时，向已连接 bootstrap 拉取缓存 peer 列表/peer record，再按 world/network/signature 校验后并入候选集。
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
- 完成定义:
  - 业务层不再把静态 peer 地址当作唯一网络真值
  - discovery 至少支持两类独立 source

### P2PARCH-2 / runtime_engineer
- 输入:
  - P2PARCH-1 identity/discovery 结果
- 输出:
  - direct / punched / relay 统一 transport API
  - path ranking 与 failover 策略
- 完成定义:
  - direct -> punched -> relay 对业务透明
  - relay failure 可自动切换备用路径

### P2PARCH-3 / runtime_engineer
- 输入:
  - 本专题 deployment modes 与 role model
- 输出:
  - deployment config schema
  - role admission / exposed-surface policy
- 完成定义:
  - `validator_hidden`、`relay_only` 成为正式配置，不再靠脚本约定
  - validator core 不再要求自身全公网暴露

### P2PARCH-4 / runtime_engineer
- 输入:
  - 统一 transport substrate
- 输出:
  - lane registry
  - consensus/sync/blob/control QoS policy
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
- 完成定义:
  - operator/ASN/subnet/relay-domain 多样性具备正式阈值与 block 条件

### P2PARCH-6 / qa_engineer
- 输入:
  - P2PARCH-2~5 reachability / role / policy 结果
- 输出:
  - mixed-topology matrix
  - chaos / failover / relay exhaustion 证据模板
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
- 下一步: 继续补齐 `P2PARCH-1` 余量，把 rendezvous 自动化接进 runtime，再进入 `P2PARCH-2~3` 的 transport / role policy 收口；在此之前，不再把“本机无公网 IP 连不上”视为单点部署事故。
- 最近更新: 2026-04-02
