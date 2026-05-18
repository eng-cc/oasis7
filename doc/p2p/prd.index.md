# p2p PRD 文件级索引

审计轮次: 13

更新时间：2026-05-18

## 入口
- 模块 PRD：`doc/p2p/prd.md`
- 模块设计总览：`doc/p2p/design.md`
- 模块标准执行入口：`doc/p2p/project.md`

## 首读分流
- 想先回答模块在做什么、主链和 DistFS 的统一边界是什么：先读 `doc/p2p/prd.md`
- 想先回答当前在推进什么、哪些安全/签名/覆盖网络任务仍在推进：先读 `doc/p2p/project.md`
- 想先进入 `node` 热点子域，并按奖励 / 复制 / PoS 时间 / 身份引导 / WASM 编译问题分流：先读 `doc/p2p/node/README.md`
- 想先看主链安全、hosted world 接入或 mixed-topology reachability：优先从 `blockchain/` 与 `network/` 子域进入
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-10）
- `doc/p2p/`：269 份文件
- `doc/p2p/node/`：68 份文件
- `doc/p2p/distfs/`：66 份文件
- `doc/p2p/blockchain/`：62 份文件
- `doc/p2p/observer/`：15 份文件
- `doc/p2p/token/`：14 份文件
- `doc/p2p/network/`：12 份文件
- `doc/p2p/distributed/`：12 份文件
- `doc/p2p/viewer-live/`：9 份文件
- `doc/p2p/consensus/`：6 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `node/` | 68 | 节点奖励、身份、复制、PoS 时间基线与执行验证 |
| `distfs/` | 66 | DistFS 生产加固、路径索引、自愈与 runtime/bridge 集成 |
| `blockchain/` | 62 | 主链安全、mainnet readiness、signer custody、hosted world 接入 |
| `observer/` | 15 | 观察者同步模式、指标与可观测性 |
| `token/` | 14 | 创世分配、签名授权、治理分发与流通边界 |
| `network/` | 12 | reachability、mobile light client、runtime bridge 与 mixed-topology |
| `distributed/` | 12 | 分布式 runtime / consensus / hard split 路线 |
| `viewer-live/` | 9 | viewer-live 发行、开关与观察服务边界 |
| `consensus/` | 6 | 共识实现与内建 wasm 身份口径 |

## 活跃补充文档
- `doc/p2p/node/README.md`：`node/` 热点子域 landing page，按奖励、复制、PoS 时间、身份引导与 WASM 编译分流读者。
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`：formal `public_testnet` 从 `specified_skeleton_only` 进入 `ready_for_live_candidate` 前的 companion checklist/runbook。
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`：共享网络最小发布列车的执行 companion runbook，不并入下方 PRD 三件套长表。
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.runbook.md`：LetAI Run OpenAPI bridge 的 operator companion runbook，覆盖独立部署、首次演练、manual review 与回滚边界。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“近期专题”；完整清单继续保留在下方，用于精确文件名检索和互链可达性。
- `node/README.md` 负责最高密度热点子域的首读分流；完整长表继续由本页保留。
- runbook、release 补充材料与历史说明继续保留可检索性，但不进入模块默认首屏。

## 覆盖规则（ROUND-005 统一）
- 纳入规则：纳入 `doc/p2p/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：`*.runbook.md` 与仍被当前模块 PRD / 项目态直接引用的 supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**` 与非 PRD 配对文档（如 `*.release.md` 补充材料）。
- 历史入口：根目录历史入口文件（`p2p.prd.md` / `p2p.project.md`）仅保留兼容跳转语义，不作为主索引分母。
- 兼容跳转：历史路径命中时统一跳转到本目录 `prd.md` / `project.md` 主入口。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.project.md` |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.prd.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.design.md` | `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.project.md` |
| `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.prd.md` | `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md` | `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md` |
| `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md` | `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.design.md` | `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.project.md` |
| `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md` | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md` | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md` |
| `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md` | `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.design.md` | `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md` |
| `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.prd.md` | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.design.md` | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md` |
| `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md` | `doc/p2p/consensus/builtin-wasm-identity-consensus.design.md` | `doc/p2p/consensus/builtin-wasm-identity-consensus.project.md` |
| `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.prd.md` | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.design.md` | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.project.md` |
| `doc/p2p/distfs/distfs-builtin-wasm-api-closure.prd.md` | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.design.md` | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.project.md` |
| `doc/p2p/distfs/distfs-builtin-wasm-storage.prd.md` | `doc/p2p/distfs/distfs-builtin-wasm-storage.design.md` | `doc/p2p/distfs/distfs-builtin-wasm-storage.project.md` |
| `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.prd.md` | `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.design.md` | `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.project.md` |
| `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.prd.md` | `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.design.md` | `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.project.md` |
| `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.prd.md` | `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.design.md` | `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.project.md` |
| `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.prd.md` | `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.design.md` | `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.project.md` |
| `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.prd.md` | `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.design.md` | `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.project.md` |
| `doc/p2p/distfs/distfs-path-index-observer-bootstrap.prd.md` | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.design.md` | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase1.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase1.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase1.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase2.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase2.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase2.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase3.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase3.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase3.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase4.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase4.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase4.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase5.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase5.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase5.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase6.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase6.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase6.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase7.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase7.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase7.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase8.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase8.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase8.project.md` |
| `doc/p2p/distfs/distfs-production-hardening-phase9.prd.md` | `doc/p2p/distfs/distfs-production-hardening-phase9.design.md` | `doc/p2p/distfs/distfs-production-hardening-phase9.project.md` |
| `doc/p2p/distfs/distfs-runtime-path-index.prd.md` | `doc/p2p/distfs/distfs-runtime-path-index.design.md` | `doc/p2p/distfs/distfs-runtime-path-index.project.md` |
| `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md` | `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.design.md` | `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md` |
| `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.prd.md` | `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.design.md` | `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.project.md` |
| `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md` | `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.design.md` | `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md` |
| `doc/p2p/distfs/distfs-standard-file-io.prd.md` | `doc/p2p/distfs/distfs-standard-file-io.design.md` | `doc/p2p/distfs/distfs-standard-file-io.project.md` |
| `doc/p2p/distributed/distributed-hard-split-phase7.prd.md` | `doc/p2p/distributed/distributed-hard-split-phase7.design.md` | `doc/p2p/distributed/distributed-hard-split-phase7.project.md` |
| `doc/p2p/distributed/distributed-pos-consensus.prd.md` | `doc/p2p/distributed/distributed-pos-consensus.design.md` | `doc/p2p/distributed/distributed-pos-consensus.project.md` |
| `doc/p2p/distributed/distributed-runtime.prd.md` | `doc/p2p/distributed/distributed-runtime.design.md` | `doc/p2p/distributed/distributed-runtime.project.md` |
| `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.prd.md` | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.design.md` | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.project.md` |
| `doc/p2p/network/net-runtime-bridge-closure.prd.md` | `doc/p2p/network/net-runtime-bridge-closure.design.md` | `doc/p2p/network/net-runtime-bridge-closure.project.md` |
| `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md` | `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.design.md` | `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.project.md` |
| `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md` | `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.design.md` | `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.project.md` |
| `doc/p2p/network/readme-p1-network-production-hardening.prd.md` | `doc/p2p/network/readme-p1-network-production-hardening.design.md` | `doc/p2p/network/readme-p1-network-production-hardening.project.md` |
| `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.prd.md` | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.design.md` | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.project.md` |
| `doc/p2p/node/node-consensus-signer-binding-replication-hardening.prd.md` | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.design.md` | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.project.md` |
| `doc/p2p/node/node-contribution-points-multi-node-closure-test.prd.md` | `doc/p2p/node/node-contribution-points-multi-node-closure-test.design.md` | `doc/p2p/node/node-contribution-points-multi-node-closure-test.project.md` |
| `doc/p2p/node/node-contribution-points-runtime-closure.prd.md` | `doc/p2p/node/node-contribution-points-runtime-closure.design.md` | `doc/p2p/node/node-contribution-points-runtime-closure.project.md` |
| `doc/p2p/node/node-contribution-points.prd.md` | `doc/p2p/node/node-contribution-points.design.md` | `doc/p2p/node/node-contribution-points.project.md` |
| `doc/p2p/node/node-distfs-replication-network-closure.prd.md` | `doc/p2p/node/node-distfs-replication-network-closure.design.md` | `doc/p2p/node/node-distfs-replication-network-closure.project.md` |
| `doc/p2p/node/node-execution-reward-consensus-bridge.prd.md` | `doc/p2p/node/node-execution-reward-consensus-bridge.design.md` | `doc/p2p/node/node-execution-reward-consensus-bridge.project.md` |
| `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.prd.md` | `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.design.md` | `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.project.md` |
| `doc/p2p/node/node-keypair-config-bootstrap.prd.md` | `doc/p2p/node/node-keypair-config-bootstrap.design.md` | `doc/p2p/node/node-keypair-config-bootstrap.project.md` |
| `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md` | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.design.md` | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.project.md` |
| `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.prd.md` | `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.design.md` | `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.project.md` |
| `doc/p2p/node/node-redeemable-power-asset.prd.md` | `doc/p2p/node/node-redeemable-power-asset.design.md` | `doc/p2p/node/node-redeemable-power-asset.project.md` |
| `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md` | `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.design.md` | `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.project.md` |
| `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md` | `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.design.md` | `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.project.md` |
| `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md` | `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.design.md` | `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.project.md` |
| `doc/p2p/node/node-replication-libp2p-migration.prd.md` | `doc/p2p/node/node-replication-libp2p-migration.design.md` | `doc/p2p/node/node-replication-libp2p-migration.project.md` |
| `doc/p2p/node/node-reward-runtime-production-hardening-phase1.prd.md` | `doc/p2p/node/node-reward-runtime-production-hardening-phase1.design.md` | `doc/p2p/node/node-reward-runtime-production-hardening-phase1.project.md` |
| `doc/p2p/node/node-reward-settlement-native-transaction.prd.md` | `doc/p2p/node/node-reward-settlement-native-transaction.design.md` | `doc/p2p/node/node-reward-settlement-native-transaction.project.md` |
| `doc/p2p/node/node-storage-system-reward-pool.prd.md` | `doc/p2p/node/node-storage-system-reward-pool.design.md` | `doc/p2p/node/node-storage-system-reward-pool.project.md` |
| `doc/p2p/node/node-triad-observability-stack-2026-04-23.prd.md` | `doc/p2p/node/node-triad-observability-stack-2026-04-23.design.md` | `doc/p2p/node/node-triad-observability-stack-2026-04-23.project.md` |
| `doc/p2p/node/node-uptime-base-reward.prd.md` | `doc/p2p/node/node-uptime-base-reward.design.md` | `doc/p2p/node/node-uptime-base-reward.project.md` |
| `doc/p2p/node/node-wasm32-libp2p-compile-guard.prd.md` | `doc/p2p/node/node-wasm32-libp2p-compile-guard.design.md` | `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md` |
| `doc/p2p/node/node-net-stack-unification-readme.prd.md` | `doc/p2p/node/node-net-stack-unification-readme.design.md` | `doc/p2p/node/node-net-stack-unification-readme.project.md` |
| `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.prd.md` | `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.design.md` | `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.project.md` |
| `doc/p2p/observer/observer-sync-mode-observability.prd.md` | `doc/p2p/observer/observer-sync-mode-observability.design.md` | `doc/p2p/observer/observer-sync-mode-observability.project.md` |
| `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md` | `doc/p2p/observer/observer-sync-mode-runtime-metrics.design.md` | `doc/p2p/observer/observer-sync-mode-runtime-metrics.project.md` |
| `doc/p2p/observer/observer-sync-source-dht-mode.prd.md` | `doc/p2p/observer/observer-sync-source-dht-mode.design.md` | `doc/p2p/observer/observer-sync-source-dht-mode.project.md` |
| `doc/p2p/observer/observer-sync-source-mode.prd.md` | `doc/p2p/observer/observer-sync-source-mode.design.md` | `doc/p2p/observer/observer-sync-source-mode.project.md` |
| `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md` | `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.design.md` | `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.project.md` |
| `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md` | `doc/p2p/token/mainchain-token-allocation-mechanism.design.md` | `doc/p2p/token/mainchain-token-allocation-mechanism.project.md` |
| `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md` | `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.design.md` | `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md` |
| `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md` | `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md` | `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md` |
| `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md` | `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.design.md` | `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.project.md` |

## 发布说明文档（release，补充材料）
| 发布说明 | 对应专题 |
| --- | --- |
| `doc/p2p/node/node-redeemable-power-asset.release.md` | `doc/p2p/node/node-redeemable-power-asset.prd.md` |
| `doc/p2p/node/node-redeemable-power-asset-audit-hardening.release.md` | `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md` |
| `doc/p2p/token/mainchain-token-allocation-mechanism.release.md` | `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- `*.release.md` 为发布补充材料，不参与 PRD 任务配对规则。
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24` 另有执行 companion：`doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`。
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14` 另有执行 companion：`doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`。
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06` 另有执行 companion：`doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.runbook.md`。
- ROUND-002 主从口径（observer）：`observer-sync-source-mode` 为主文档；`observer-sync-source-dht-mode` 为 DHT 增量子文档。
- ROUND-002 主从口径（observer）：`observer-sync-mode-runtime-metrics` 为主文档；`metrics-runtime-bridge` 与 `observability` 为增量子文档。
- ROUND-002 主从口径（node）：`node-contribution-points` 为主文档；`runtime-closure` 与 `multi-node-closure-test` 为增量子文档。
- ROUND-002 主从口径（node）：`node-redeemable-power-asset` 为主文档；`audit-hardening` 与 `signature-governance-phase3` 为增量子文档。
- ROUND-002 主从口径（distfs）：`distfs-self-healing-control-plane-2026-02-23` 为主文档；`polling-loop` 与 `runtime-polling-wiring` 为增量子文档。
- ROUND-002 主从口径（distfs）：`distfs-production-hardening-phase1` 为主文档；`phase2~phase9` 为增量子文档。
