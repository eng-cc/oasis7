# ROUND-008 Design 补齐优先级清单

审计轮次: 8

## 缺口总数
- 总缺口专题数: `365`
- 已补齐专题数: `365`
- 剩余待补齐专题数: `0`

## 模块分布
| 模块 | 缺口数 |
| --- | --- |

## 分级规则（首版）
- `must_backfill`: 存在接口/协议、状态机/时序、跨组件协作、错误恢复或已明显在 `PRD/Project` 中承载设计内容。
- `should_backfill`: 已形成长期维护主题、跨多阶段推进、多人协作频繁，但暂未见明确协议/状态机。
- `defer_allowed`: 范围小、生命周期短、没有独立结构设计。

## 分级统计
- `must_backfill`: 347
- `should_backfill`: 16
- `defer_allowed`: 2

## 首批已补齐专题
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03`
- `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27`
- `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23`
- `doc/engineering/prd-review/prd-full-system-audit-2026-03-03`

- `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26`

- `doc/world-runtime/module/agent-default-modules`

- `doc/world-runtime/module/module-storage`

- `doc/world-runtime/runtime/bootstrap-power-modules`

- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution`

- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network`

- `doc/p2p/consensus/builtin-wasm-identity-consensus`

- `doc/p2p/distfs/distfs-builtin-wasm-api-closure`

- `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27`
- `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant`
- `doc/world-simulator/viewer/viewer-web-closure-testing-policy`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07`
- `doc/world-simulator/kernel/kernel-rule-wasm-readiness`
- `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06`
- `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09`
- `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration`
- `doc/world-simulator/kernel/rust-wasm-build-suite`
- `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07`
- `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08`
- `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04`
- `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04`
- `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06`
- `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06`
- `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02`
- `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02`
- `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02`
- `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02`
- `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02`
- `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06`
- `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06`
- `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08`
- `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08`
- `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09`
- `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08`
- `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04`

## 新增已补齐（2026-03-10 / p2p 收口）
- `doc/p2p/network/net-runtime-bridge-closure`
- `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06`
- `doc/p2p/network/readme-p1-network-production-hardening`
- `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile`
- `doc/p2p/node/node-consensus-signer-binding-replication-hardening`
- `doc/p2p/node/node-contribution-points`
- `doc/p2p/node/node-contribution-points-runtime-closure`
- `doc/p2p/node/node-contribution-points-multi-node-closure-test`
- `doc/p2p/node/node-distfs-replication-network-closure`
- `doc/p2p/node/node-execution-reward-consensus-bridge`
- `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening`
- `doc/p2p/node/node-keypair-config-bootstrap`
- `doc/p2p/node/node-net-stack-unification-readme`
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07`
- `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07`
- `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07`
- `doc/p2p/node/node-redeemable-power-asset`
- `doc/p2p/node/node-redeemable-power-asset-audit-hardening`
- `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3`
- `doc/p2p/node/node-replication-libp2p-migration`
- `doc/p2p/node/node-reward-runtime-production-hardening-phase1`
- `doc/p2p/node/node-reward-settlement-native-transaction`
- `doc/p2p/node/node-storage-system-reward-pool`
- `doc/p2p/node/node-uptime-base-reward`
- `doc/p2p/node/node-wasm32-libp2p-compile-guard`
- `doc/p2p/distfs/distfs-path-index-observer-bootstrap`
- `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge`
- `doc/p2p/observer/observer-sync-mode-observability`
- `doc/p2p/observer/observer-sync-mode-runtime-metrics`
- `doc/p2p/observer/observer-sync-source-dht-mode`
- `doc/p2p/observer/observer-sync-source-mode`
- `doc/p2p/token/mainchain-token-allocation-mechanism`
- `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26`
- `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23`
- `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23`
- `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23`

## 新增已补齐（2026-03-10 / world-runtime + world-simulator 收口）
- `doc/world-runtime/module/module-subscription-filters`
- `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08`
- `doc/world-runtime/module/player-published-entities-2026-03-05`
- `doc/world-runtime/runtime/runtime-infinite-sequence-rollover`
- `doc/world-runtime/runtime/runtime-numeric-correctness-phase1` ~ `phase15`
- `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening`
- `doc/world-runtime/wasm/wasm-executor`
- `doc/world-runtime/wasm/wasm-sandbox-security-hardening`
- `doc/world-runtime/wasm/wasm-sdk-no-std`
- `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup`
- `doc/world-simulator/llm/*` 13 个专题
- `doc/world-simulator/m4/*` 12 个专题
- `doc/world-simulator/scenario/*` 10 个专题

## 新增已补齐（2026-03-10 / world-simulator 收口）
- `doc/world-simulator/llm/*`（13 个专题）
- `doc/world-simulator/m4/*`（12 个专题）
- `doc/world-simulator/scenario/*`（11 个专题）

## 首批待补齐（must_backfill 样本）
- `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split` — path:runtime,wasm; content:design-signals; lines:101
- `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22` — path:hardening; content:design-signals; lines:108
- `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06` — path:consensus,governance; content:design-signals; lines:332
- `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure` — context:closure; content:design-signals; lines:115
- `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure` — path:governance; context:closure; content:design-signals; lines:123
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06` — path:hardening; content:design-signals; lines:243
- `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05` — context:feedback,visibility; content:design-signals; lines:316
- `doc/game/gameplay/gameplay-module-driven-production-closure` — path:module; context:closure; content:design-signals; lines:130
- `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21` — context:closure,gap,release; content:design-signals; lines:308
- `doc/game/gameplay/gameplay-release-production-closure` — context:closure,release; content:design-signals; lines:133
- `doc/game/gameplay/gameplay-runtime-governance-closure` — path:governance,runtime; context:closure; content:design-signals; lines:125
- `doc/game/gameplay/gameplay-top-level-design` — content:design-signals; lines:561
- `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25` — path:runtime,viewer; context:closure; content:design-signals; lines:116
- `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25` — path:runtime,viewer; context:review; content:design-signals; lines:128
- `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23` — path:archive,hardening,runtime,trace; content:design-signals; lines:146
- `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening` — path:auth,hardening,protocol,runtime; content:design-signals; lines:143
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2` — path:blockchain,hardening,p2pfs; context:phase; content:design-signals; lines:108; module:priority
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3` — path:blockchain,hardening,p2pfs; context:phase; content:design-signals; lines:125; module:priority
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4` — path:blockchain,hardening,p2pfs; context:phase; content:design-signals; lines:113; module:priority
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5` — path:blockchain,hardening,p2pfs; context:phase; content:design-signals; lines:109; module:priority

- `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08`
- `doc/world-simulator/viewer/viewer-2d-3d-clarity-improvement`
- `doc/world-simulator/viewer/viewer-2d-visual-polish`
- `doc/world-simulator/viewer/viewer-3d-commercial-polish`
- `doc/world-simulator/viewer/viewer-3d-polish-performance`
- `doc/world-simulator/viewer/viewer-agent-module-rendering`
- `doc/world-simulator/viewer/viewer-agent-quick-locate`
- `doc/world-simulator/viewer/viewer-agent-size-inspection`
- `doc/world-simulator/viewer/viewer-asset-pipeline-ui-system-hardening-2026-03-05`
- `doc/world-simulator/viewer/viewer-auto-focus-capture`
- `doc/world-simulator/viewer/viewer-auto-select-capture`
- `doc/world-simulator/viewer/viewer-bevy-web-runtime`
- `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill`
- `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel`
- `doc/world-simulator/viewer/viewer-chat-enter-send`
- `doc/world-simulator/viewer/viewer-chat-ime-cn-input`
- `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge`
- `doc/world-simulator/viewer/viewer-chat-prompt-presets`
- `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing`
- `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll`
- `doc/world-simulator/viewer/viewer-chat-right-panel-polish`
- `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution`
- `doc/world-simulator/viewer/viewer-commercial-release-phase1-asset-pipeline`
- `doc/world-simulator/viewer/viewer-commercial-release-phase2-visual-quality-gate`
- `doc/world-simulator/viewer/viewer-commercial-release-phase3-material-style-layer`
- `doc/world-simulator/viewer/viewer-commercial-release-phase4-texture-style-layer`
- `doc/world-simulator/viewer/viewer-commercial-release-phase5-advanced-texture-maps`
- `doc/world-simulator/viewer/viewer-commercial-release-phase6-material-variant-preview`
- `doc/world-simulator/viewer/viewer-commercial-release-phase7-theme-pack-batch-preview`
- `doc/world-simulator/viewer/viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2`
- `doc/world-simulator/viewer/viewer-control-advanced-debug-folding`
- `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27`
- `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27`
- `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27`
- `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28`
- `doc/world-simulator/viewer/viewer-copyable-text`
- `doc/world-simulator/viewer/viewer-dual-view-2d-3d`
- `doc/world-simulator/viewer/viewer-egui-right-panel`
- `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27`
- `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27`
- `doc/world-simulator/viewer/viewer-frag-default-rendering`
- `doc/world-simulator/viewer/viewer-frag-scale-selection-stability`
- `doc/world-simulator/viewer/viewer-fragment-element-rendering`
- `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6`
- `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7`
- `doc/world-simulator/viewer/viewer-generic-focus-targets`
- `doc/world-simulator/viewer/viewer-i18n`
- `doc/world-simulator/viewer/viewer-industrial-visual-closure`
- `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28`
- `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27`
- `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27`
- `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26`
- `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27`
- `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04`
- `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05`
- `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05`
- `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28`
- `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27`
- `doc/world-simulator/viewer/viewer-location-depletion-visualization`
- `doc/world-simulator/viewer/viewer-location-fine-grained-rendering`
- `doc/world-simulator/viewer/viewer-minimal-system`
- `doc/world-simulator/viewer/viewer-module-visual-entities`
- `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28`
- `doc/world-simulator/viewer/viewer-observability-visual-optimization`
- `doc/world-simulator/viewer/viewer-overview-map-zoom`
- `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24`
- `doc/world-simulator/viewer/viewer-release-full-coverage-gate`
- `doc/world-simulator/viewer/viewer-release-qa-iteration-loop`
- `doc/world-simulator/viewer/viewer-rendering-physical-accuracy`
- `doc/world-simulator/viewer/viewer-right-panel-module-visibility`
- `doc/world-simulator/viewer/viewer-selection-details`
- `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28`
- `doc/world-simulator/viewer/viewer-texture-inspector`
- `doc/world-simulator/viewer/viewer-visual-release-readiness-hardening-2026-03-01`
- `doc/world-simulator/viewer/viewer-visual-upgrade`
- `doc/world-simulator/viewer/viewer-visualization`
- `doc/world-simulator/viewer/viewer-visualization-3d`
- `doc/world-simulator/viewer/viewer-wasd-camera-navigation`
- `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle`
- `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26`
- `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24`
- `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22`
- `doc/world-simulator/viewer/viewer-webgl-deferred-compat-2026-02-24`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7`
- `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8`
- `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23`
- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap`
- `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus`
- `doc/p2p/distfs/distfs-builtin-wasm-storage`
- `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01`
- `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01`
- `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01`
- `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23`
- `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23`
- `doc/p2p/distfs/distfs-production-hardening-phase1`
- `doc/p2p/distfs/distfs-production-hardening-phase2`
- `doc/p2p/distfs/distfs-production-hardening-phase3`
- `doc/p2p/distfs/distfs-production-hardening-phase4`
- `doc/p2p/distfs/distfs-production-hardening-phase5`
- `doc/p2p/distfs/distfs-production-hardening-phase6`
- `doc/p2p/distfs/distfs-production-hardening-phase7`
- `doc/p2p/distfs/distfs-production-hardening-phase8`
- `doc/p2p/distfs/distfs-production-hardening-phase9`
- `doc/p2p/distfs/distfs-runtime-path-index`
- `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23`
- `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23`
- `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23`
- `doc/p2p/distfs/distfs-standard-file-io`
