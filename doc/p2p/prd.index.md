# p2p PRD 文件级索引

审计轮次: 12

更新时间：2026-04-02

## 入口
- 模块 PRD：`doc/p2p/prd.md`
- 模块设计总览：`doc/p2p/design.md`
- 模块标准执行入口：`doc/p2p/project.md`

## 覆盖规则（ROUND-005 统一）
- 纳入规则：纳入 `doc/p2p/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 排除规则：不纳入 `doc/devlog/**` 与非 PRD 配对文档（如 `*.release.md` 补充材料）。
- 历史入口：根目录历史入口文件（`p2p.prd.md` / `p2p.project.md`）仅保留兼容跳转语义，不作为主索引分母。
- 兼容跳转：历史路径命中时统一跳转到本目录 `prd.md` / `project.md` 主入口。

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
- ROUND-002 主从口径（observer）：`observer-sync-source-mode` 为主文档；`observer-sync-source-dht-mode` 为 DHT 增量子文档。
- ROUND-002 主从口径（observer）：`observer-sync-mode-runtime-metrics` 为主文档；`metrics-runtime-bridge` 与 `observability` 为增量子文档。
- ROUND-002 主从口径（node）：`node-contribution-points` 为主文档；`runtime-closure` 与 `multi-node-closure-test` 为增量子文档。
- ROUND-002 主从口径（node）：`node-redeemable-power-asset` 为主文档；`audit-hardening` 与 `signature-governance-phase3` 为增量子文档。
- ROUND-002 主从口径（distfs）：`distfs-self-healing-control-plane-2026-02-23` 为主文档；`polling-loop` 与 `runtime-polling-wiring` 为增量子文档。
- ROUND-002 主从口径（distfs）：`distfs-production-hardening-phase1` 为主文档；`phase2~phase9` 为增量子文档。
