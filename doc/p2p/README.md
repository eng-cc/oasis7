# p2p 文档索引

审计轮次: 11

## 从这里开始
- 想先理解 P2P / 主链 / DistFS / 节点奖励的总边界：`doc/p2p/prd.md`
- 想看当前活跃任务、阻断与最新完成项：`doc/p2p/project.md`
- 想按专题文件名精确查某个 blockchain / token / node / distfs 文档：`doc/p2p/prd.index.md`
- 想先看主链安全、mainnet-grade readiness 与 signer custody：`doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`、`doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- 想先看 hosted world 玩家接入与网页会话鉴权：`doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- 想先看“没有公网 IP 也要成为正式节点”的主链级覆盖网络目标态：`doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
- 想先看 Token 分配 / 治理签名 / 生产 signer 外部化：`doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`、`doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`、`doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`

## 入口
- PRD: `doc/p2p/prd.md`
- 设计总览: `doc/p2p/design.md`
- 标准执行入口: `doc/p2p/project.md`
- 文件级索引: `doc/p2p/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者先选“总边界 / 当前执行 / 精确索引 / 高频专题”中的正确入口。
- `prd.md` 是模块权威规格入口，适合先理解主链、共识、DistFS、节点、token 与 hosted world 的统一边界。
- `project.md` 是执行台账，适合确认当前安全硬化、signer 外部化、token 与 hosted world 相关任务的推进状态。
- `prd.index.md` 是精确检索索引，适合已知专题名后按文件名直达，不适合作为第一次进入 p2p 模块时的首读入口。
- 高频专题承担主题真值：`p2p-mainnet-*` 负责主链安全与 readiness；`p2p-mainnet-private-reachability-architecture-2026-04-01` 负责 mixed-topology 覆盖网络目标态；`p2p-hosted-world-player-access-and-session-auth` 负责玩家接入与会话鉴权；token / signer 系列专题负责分配、签名交易与治理签名外部化。

## 模块职责
- 维护 P2P、共识、DistFS、节点奖励与网络桥接等核心链路口径。
- 汇总 blockchain / distfs / node / observer / token / viewer-live / consensus / distributed / network 九类专题。
- 承接跨 runtime、launcher、viewer-live 的分布式运行与发布约束收口。
- 承接非全公网 mixed-topology、sentry/relay、overlay reachability 与多链型数据面适配的框架层口径。
- 承接 hosted world 玩家接入、网页会话鉴权、public/control/signer 平面边界等跨模块 Web/P2P 口径。

## 主题目录
- `distfs/`：DistFS 设计与稳定性加固。
- `node/`：节点能力、奖励、身份与复制链路。
- `observer/`：观察者同步模式与可观测性。
- `blockchain/`：区块链与 P2PFS 硬化阶段。
- `token/`：主链 token 分配、创世分桶、低流通与治理分发。
- `viewer-live/`：viewer live 发行与开关策略。
- `consensus/`：共识相关专题。
- `distributed/`：分布式运行时专题。
- `network/`：网络桥接专题。

## 近期专题
- `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md`
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`
- `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
- `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
- `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.prd.md`

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-structure-standard.design.md` 为准。
- 模块行为、默认参数或跨模块分布式口径变化时，优先更新 `doc/p2p/prd.md` / `doc/p2p/project.md`；高频入口变化时，再同步回写本目录“从这里开始”与 `doc/p2p/prd.index.md`。
