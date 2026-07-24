# p2p PRD 文件级索引

审计轮次: 14

更新时间：2026-06-13

## 入口
- 模块 PRD：`doc/p2p/prd.md`
- 模块设计总览：`doc/p2p/design.md`
- 模块标准执行入口：`doc/p2p/project.md`

## 首读分流
- 想先回答模块在做什么、主链和 DistFS 的统一边界是什么：先读 `doc/p2p/prd.md`
- 想先回答当前在推进什么、哪些安全/签名/覆盖网络任务仍在推进：先读 `doc/p2p/project.md`
- 想先回答 P2P、DistFS、共识、执行与 observer 如何作为“链上大世界状态底座”单模块自闭环测试：先读 `testing-manual.md#s9a链上大世界状态底座自闭环`
- 想先进入 `node` 热点子域，并按奖励 / 复制 / PoS 时间 / 身份引导 / WASM 编译问题分流：先读 `doc/p2p/node/README.md`
- 想区分 builtin Wasm identity 与共识代码 crate 收敛两个已完成专题：先读 `doc/p2p/consensus/README.md`
- 想先看主链安全、network tier、hosted player access / `hosted_public_join` 接入或托管身份/托管密钥：先读 `doc/p2p/blockchain/README.md`；mixed-topology reachability 再进入 `network/` 子域
- 想先看 Token 创世分配、交易授权、治理分发、理想化交易或服务额度 bridge：先读 `doc/p2p/token/README.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 热点子域导航
| 子域 | 适合回答的问题 |
| --- | --- |
| `node/` | 节点奖励、身份、复制、PoS 时间基线与执行验证 |
| `distfs/` | DistFS 生产加固、路径索引、自愈与 runtime/bridge 集成；默认先读 `doc/p2p/distfs/README.md` |
| `blockchain/` | 主链安全、network tier、mainnet readiness、signer custody、hosted player access；默认先读 `doc/p2p/blockchain/README.md` |
| `observer/` | 观察者同步模式、指标与可观测性；默认先读 `doc/p2p/observer/README.md` |
| `token/` | 创世分配、签名授权、治理分发与流通边界；默认先读 `doc/p2p/token/README.md` |
| `network/` | reachability、mobile light client、runtime bridge 与 mixed-topology |
| `distributed/` | 分布式 runtime / consensus / hard split 路线；首读 `doc/p2p/distributed/README.md` |
| `viewer-live/` | Viewer LLM 默认与 observer/debug-only `--no-llm` 边界；默认先读 `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.prd.md` |
| `consensus/` | 共识实现与内建 wasm 身份口径；默认先读 `doc/p2p/consensus/README.md` |

## 当前补充阅读面
- `doc/engineering/governance/environment-lanes-and-inventory-2026-05-29.md`：项目级 test/prod 环境分层、hosted-login 云上清单与 `testnet/mainnet` claim boundary。
- `doc/p2p/node/README.md`：`node/` 热点子域 landing page，按奖励、复制、PoS 时间、身份引导与 WASM 编译分流读者。
- `doc/p2p/blockchain/README.md`：`blockchain/` 高密度子域 landing page，按现行 network tier、mainnet-grade security、hosted player identity 与历史 P2PFS hardening 分流读者。
- `doc/p2p/blockchain/hosted-player-access-operator-runbook.md`：hosted player join 分享、信任面隔离、session 处置、claim freeze 与最小披露的稳定 operator 入口；不定义产品 claim 或 custody 实现。
- `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`：formal `public_testnet` 从规格骨架进入候选状态前的 companion checklist/runbook。
- `doc/p2p/blockchain/p2p-public-testnet-governed-bootstrap-2026-06-06.runbook.md`：formal `public_testnet` governed bootstrap operator path，定义四节点重建输入、deployment truth、hard rules 与 evidence 闭包；不表示 `public_testnet` 已 live 或 ready。
- `testing-manual.md#s9a链上大世界状态底座自闭环`：P2P transport、DistFS/blob closure、replication/gap sync/state sync、consensus/finality、execution record/receipt、observer/ops 与 API/viewer projection 的模块自闭环测试入口；用于区分 `module_required/module_full/integration_required/release_full` claim boundary。
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.runbook.md`：共享网络最小发布列车的 legacy 执行 companion runbook；`shared_devnet` pass 仅作 rehearsal evidence，不等于 formal `public_testnet` / `mainnet` readiness。
- `doc/testing/evidence/README.md`：QA evidence landing page，负责 public-testnet readiness evidence / claims-boundary / mixed-topology / legacy shared-network 证据的当前入口与归档边界。
- `doc/p2p/token/README.md`：Token 专题簇 landing page，按创世冻结、分配机制、授权、目标态与 quota bridge 分流；再按需进入理想交易设计或 quota bridge runbook。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“近期专题”；完整清单继续保留在下方，用于精确文件名检索和互链可达性。
- `node/README.md`、`blockchain/README.md`、`distfs/README.md`、`observer/README.md`、`token/README.md`、`viewer-live/README.md` 与 `consensus/README.md` 负责专题簇的首读分流；完整长表继续由本页保留。
- runbook、release 补充材料与历史说明继续保留可检索性，但不进入模块默认首屏。

## 状态语义
- `active`：当前执行真值仍以 `doc/p2p/project.md` 和具体专题 `*.project.md` 的状态段为准。
- `canonical`：模块规格、设计总览、热点子域入口、当前补充阅读面和专题主文档；这些文档可长期保留在默认可达路径中。
- `completed`：已完成但仍需按文件名可达的专题三件套；保留在下方检索清单，不自动进入默认首读面。
- `historical` / `evidence-snapshot`：历史审计清单、旧轮次证据、release 补充材料与 legacy runbook；只作为追溯证据或兼容入口，不作为当前执行状态或可删除依据。

## 覆盖规则（ROUND-005 统一）
- 纳入规则：纳入 `doc/p2p/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 当前补充阅读面：`*.runbook.md` 与仍被当前模块 PRD / 项目态直接引用的 supporting spec，可在“当前补充阅读面”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**` 与非 PRD 配对文档（如 `*.release.md` 补充材料）。
- 历史入口：根目录历史入口文件（`p2p.prd.md` / `p2p.project.md`）仅保留兼容跳转语义，不作为主索引分母。
- 兼容跳转：历史路径命中时统一跳转到本目录 `prd.md` / `project.md` 主入口。

## 折叠阅读层（主从/增量组）
| 子域 | 默认先读 | 折叠的增量/追溯入口 |
| --- | --- | --- |
| `distfs/` production hardening | `doc/p2p/distfs/README.md` -> `doc/p2p/distfs/distfs-production-hardening.prd.md` | 历史 phase1~phase9 已语义收敛；原 phase 身份仅在历史 audit/review 文字和 Git history 中保留，不作为当前入口或 readiness 依据 |
| `distfs/` distributed resilience | `doc/p2p/distfs/README.md` -> `doc/p2p/distfs/distfs-distributed-resilience.prd.md` | 异构 provider 选择、无单机完整依赖、自愈控制/轮询与受限 NodeRuntime 接线统一收敛；NodeRuntime 仍仅是依赖齐备时运行、失败不阻断 tick 的增量边界 |
| `observer/` sync source | `doc/p2p/observer/README.md` -> `doc/p2p/observer/observer-sync-source-mode.prd.md` | `observer-sync-source-dht-mode` 仅作为 DHT 组合链路 delta |
| `observer/` sync mode metrics | `doc/p2p/observer/README.md` -> `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md` | `observer-sync-mode-metrics-runtime-bridge` 与 `observer-sync-mode-observability` 仅作为增量入口 |
| `viewer-live/` decision mode | `doc/p2p/viewer-live/README.md` -> `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.prd.md` | 两组 2026-02 源三件套已语义回填；当前 batch 暂留，待引用审计与删除切片后移除 |

## 完整专题检索清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md` | `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.design.md` | `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.project.md` |
| `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.project.md` |
| `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.project.md` |
| `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.prd.md` | `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.design.md` | `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism.project.md` |
| `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.prd.md` | `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.design.md` | `doc/p2p/blockchain/p2p-genesis-freeze-ceremony-qa-gate-2026-03-23.project.md` |
| `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.prd.md` | `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.design.md` | `doc/testing/benchmarks/mainstream-public-chain-testing-benchmark.project.md` |
| `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md` | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.design.md` | `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.project.md` |
| `doc/p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md` | `doc/p2p/blockchain/hosted-public-join-managed-identity-custody.design.md` | `doc/p2p/blockchain/hosted-public-join-managed-identity-custody.project.md` |
| `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.prd.md` | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.design.md` | `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md` |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.design.md` | `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md` |
| `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md` | `doc/p2p/consensus/builtin-wasm-identity-consensus.design.md` | `doc/p2p/consensus/builtin-wasm-identity-consensus.project.md` |
| `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.prd.md` | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.design.md` | `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.project.md` |
| `doc/p2p/distfs/distfs-builtin-wasm-api-closure.prd.md` | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.design.md` | `doc/p2p/distfs/distfs-builtin-wasm-api-closure.project.md` |
| `doc/p2p/distfs/distfs-builtin-wasm-storage.prd.md` | `doc/p2p/distfs/distfs-builtin-wasm-storage.design.md` | `doc/p2p/distfs/distfs-builtin-wasm-storage.project.md` |
| `doc/p2p/distfs/distfs-feedback-ledger-and-replication.prd.md` | `doc/p2p/distfs/distfs-feedback-ledger-and-replication.design.md` | `doc/p2p/distfs/distfs-feedback-ledger-and-replication.project.md` |
| `doc/p2p/distfs/distfs-distributed-resilience.prd.md` | `doc/p2p/distfs/distfs-distributed-resilience.design.md` | `doc/p2p/distfs/distfs-distributed-resilience.project.md` |
| `doc/p2p/distfs/distfs-path-index-observer-bootstrap.prd.md` | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.design.md` | `doc/p2p/distfs/distfs-path-index-observer-bootstrap.project.md` |
| `doc/p2p/distfs/distfs-production-hardening.prd.md` | `doc/p2p/distfs/distfs-production-hardening.design.md` | `doc/p2p/distfs/distfs-production-hardening.project.md` |
| `doc/p2p/distfs/distfs-runtime-path-index.prd.md` | `doc/p2p/distfs/distfs-runtime-path-index.design.md` | `doc/p2p/distfs/distfs-runtime-path-index.project.md` |
| `doc/p2p/distfs/distfs-standard-file-io.prd.md` | `doc/p2p/distfs/distfs-standard-file-io.design.md` | `doc/p2p/distfs/distfs-standard-file-io.project.md` |
| `doc/p2p/distributed/distributed-hard-split-phase7.prd.md` | `doc/p2p/distributed/distributed-hard-split-phase7.design.md` | `doc/p2p/distributed/distributed-hard-split-phase7.project.md` |
| `doc/p2p/distributed/distributed-pos-consensus.prd.md` | `doc/p2p/distributed/distributed-pos-consensus.design.md` | `doc/p2p/distributed/distributed-pos-consensus.project.md` |
| `doc/p2p/distributed/distributed-runtime.prd.md` | `doc/p2p/distributed/distributed-runtime.design.md` | `doc/p2p/distributed/distributed-runtime.project.md` |
| `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.prd.md` | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.design.md` | `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.project.md` |
| `doc/p2p/network/net-runtime-bridge-closure.prd.md` | `doc/p2p/network/net-runtime-bridge-closure.design.md` | `doc/p2p/network/net-runtime-bridge-closure.project.md` |
| `doc/p2p/network/p2p-mobile-light-client-authoritative-state.prd.md` | `doc/p2p/network/p2p-mobile-light-client-authoritative-state.design.md` | `doc/p2p/network/p2p-mobile-light-client-authoritative-state.project.md` |
| `doc/p2p/network/mainnet-private-reachability-architecture.prd.md` | `doc/p2p/network/mainnet-private-reachability-architecture.design.md` | `doc/p2p/network/mainnet-private-reachability-architecture.project.md` |
| `doc/p2p/network/readme-p1-network-production-hardening.prd.md` | `doc/p2p/network/readme-p1-network-production-hardening.design.md` | `doc/p2p/network/readme-p1-network-production-hardening.project.md` |
| `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.prd.md` | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.design.md` | `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.project.md` |
| `doc/p2p/node/node-consensus-signer-binding-replication-hardening.prd.md` | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.design.md` | `doc/p2p/node/node-consensus-signer-binding-replication-hardening.project.md` |
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
| `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.prd.md` | `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.design.md` | `doc/p2p/token/mainchain-token-ideal-transaction-upgrade-2026-06-08.project.md` |
| `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md` | `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.design.md` | `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md` |
| `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md` | `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md` | `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md` |
| `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md` | `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.design.md` | `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-decision-mode.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.project.md` |
| `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md` | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.design.md` | `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.project.md` |

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
- `doc/p2p/blockchain/formal-network-tiers-testnet-mechanism` 另有 governed bootstrap operator path：`doc/p2p/blockchain/p2p-public-testnet-governed-bootstrap-2026-06-06.runbook.md`。
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06` 另有执行 companion：`doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.runbook.md`。
- ROUND-002 主从口径（observer）：`observer-sync-source-mode` 为主文档；`observer-sync-source-dht-mode` 为 DHT 增量子文档。
- ROUND-002 主从口径（observer）：`observer-sync-mode-runtime-metrics` 为主文档；`metrics-runtime-bridge` 与 `observability` 为增量子文档。
- ROUND-002 主从口径（node）：`node-contribution-points` 为主文档；`runtime-closure` 与 `multi-node-closure-test` 为增量子文档。
- NodePoints historical/provenance note：`doc/p2p/node/node-contribution-points-runtime-closure.*` 与 `doc/p2p/node/node-contribution-points-multi-node-closure-test.*` 保留为已完成增量闭环/验证来源线索，不再作为根索引普通 active 三件套入口；当前首读与主规格以 `doc/p2p/node/node-contribution-points.prd.md`、`doc/p2p/node/node-contribution-points.project.md` 和 `doc/p2p/node/README.md` 为准。后续若删除这些历史子文档，需先完成引用审计与 deletion-readiness slice。
- ROUND-002 主从口径（node）：`node-redeemable-power-asset` 为主文档；`audit-hardening` 与 `signature-governance-phase3` 为增量子文档。
- DistFS distributed resilience：`distfs-distributed-resilience` 是异构 provider 兼容/选择、无单机完整依赖、分布覆盖、自愈控制/轮询与 NodeRuntime 接线的唯一主入口。NodeRuntime 接线仍受“依赖齐备才运行、缺依赖跳过、单轮失败不阻断 tick”约束，不能被误述为全局自治恢复或 readiness 结论。
- DistFS production hardening：`distfs-production-hardening` 是 Phase 1-9 的唯一当前专业权威；历史 phase 文件名只保留为 audit/review provenance 和 Git history，后续删除必须保留该身份而不得改写为重复的当前路径。
- DistFS feedback ledger and replication：`distfs-feedback-ledger-and-replication` 是公开 feedback 的 append-only 账本、签名/nonce、announce/fetch 复制和 NodeRuntime 有界接线的唯一当前专业 authority。三个 2026-03 feedback 源三件套已删除；其追溯仅保留在 Git 与 `.pm` task evidence，且不构成 current entry、consensus/finality 或 readiness 依据。
