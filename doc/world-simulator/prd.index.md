# world-simulator PRD 文件级索引

审计轮次: 7

更新时间：2026-04-18

## 入口
- 模块 PRD：`doc/world-simulator/prd.md`
- 模块设计总览：`doc/world-simulator/design.md`
- 模块标准执行入口：`doc/world-simulator/project.md`

## 首读分流
- 想先回答模块在做什么、能力边界是什么：先读 `doc/world-simulator/prd.md`
- 想先回答当前在推进什么、谁在负责、哪里被阻断：先读 `doc/world-simulator/project.md`
- 想先进入 Viewer 热点子域，而不是直接面对近 300 份 Viewer 文档：先读 `doc/world-simulator/viewer/README.md`
- 想直接执行 Viewer / Web 闭环 / 操作步骤：先读 `doc/world-simulator/viewer/viewer-manual.manual.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-18）
- `doc/world-simulator/`：553 份文件
- `doc/world-simulator/viewer/`：297 份文件
- `doc/world-simulator/launcher/`：84 份文件
- `doc/world-simulator/llm/`：54 份文件
- `doc/world-simulator/kernel/`：36 份文件
- `doc/world-simulator/m4/`：36 份文件
- `doc/world-simulator/scenario/`：30 份文件
- `doc/world-simulator/prd/`：9 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `viewer/` | 296 | Viewer UI、Web 闭环、`software_safe`、2D/3D、操作手册与 QA/发布闭环 |
| `launcher/` | 84 | 启动器、控制面、链上转账、explorer、自引导体验 |
| `llm/` | 54 | provider、本地桥接、direct-connect、体验等价与双模式策略 |
| `kernel/` | 36 | 规则桥接、WASM 执行、runtime 约束、资源与制度规则 |
| `m4/` | 36 | M4 路线、阶段拆分与配套设计 |
| `scenario/` | 30 | 场景定义、初始化、配置模板与环境准备 |
| `prd/` | 9 | 验收模板、评分卡、质量趋势与补充附件 |

## 活跃补充文档
- `doc/world-simulator/viewer/README.md`：`viewer/` 热点子域 landing page，适合先做簇级分流，再决定进入 `manual`、`software_safe` 或 runtime live 专题。
- `doc/world-simulator/viewer/viewer-manual.manual.md`：Viewer / Web 闭环 / operator 手册，不在下方 PRD 三件套长表中展开。
- `doc/world-simulator/llm/llm-provider-agent-direct-connect-review-2026-04-06.md`：`provider agent direct connect` 的正式 review，适合在判断双模式产品完整性、实施差距和后续 remediation 时定向进入。
- `doc/world-simulator/llm/provider-agent-dual-mode-contract-2026-03-16.md`：`Local Provider` 双轨模式的 observation / action contract supporting spec。
- `doc/world-simulator/llm/provider-agent-profile-oasis7_p0_low_freq_npc-2026-03-13.md`：`Local Provider` `P0` 默认 profile supporting spec，用于解释 provider-side 行为约束与 parity 口径。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- 审计记录、历史背景与低频专题继续保留可检索性，但不在模块 README 中平铺成“近期专题”列表。
- 下方完整清单仍保留，用于精确文件名检索和互链可达性，不再承担新读者首读入口职责。

## 覆盖规则（ROUND-005 统一）
- 纳入规则：纳入 `doc/world-simulator/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：`*.manual.md` 与仍被当前模块 PRD / 项目态直接引用的 supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**` 与非 PRD 配对文档（如临时草稿/日志快照）。
- 按需进入：复签结论、状态收口、evidence、report、template 等审计留痕保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。
- 历史入口：根目录 `doc/world-simulator.prd.md` 与 `doc/world-simulator.project.md` 仅保留兼容跳转语义，不作为主索引分母。
- 兼容跳转：历史路径命中时统一跳转到本目录 `prd.md` / `project.md` 主入口。

## 完整活跃专题清单（按文件名精确检索）

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.prd.md` | `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.design.md` | `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.project.md` |
| `doc/world-simulator/kernel/kernel-rule-hook-foundation.prd.md` | `doc/world-simulator/kernel/kernel-rule-hook-foundation.design.md` | `doc/world-simulator/kernel/kernel-rule-hook-foundation.project.md` |
| `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.prd.md` | `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.design.md` | `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.project.md` |
| `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.prd.md` | `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.design.md` | `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.project.md` |
| `doc/world-simulator/kernel/kernel-rule-wasm-readiness.prd.md` | `doc/world-simulator/kernel/kernel-rule-wasm-readiness.design.md` | `doc/world-simulator/kernel/kernel-rule-wasm-readiness.project.md` |
| `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.prd.md` | `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.design.md` | `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.project.md` |
| `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md` | `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.design.md` | `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.project.md` |
| `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md` | `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.design.md` | `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.project.md` |
| `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.prd.md` | `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.design.md` | `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.project.md` |
| `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.prd.md` | `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.design.md` | `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.project.md` |
| `doc/world-simulator/kernel/rust-wasm-build-suite.prd.md` | `doc/world-simulator/kernel/rust-wasm-build-suite.design.md` | `doc/world-simulator/kernel/rust-wasm-build-suite.project.md` |
| `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md` | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.design.md` | `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.prd.md` | `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.design.md` | `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.prd.md` | `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.design.md` | `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md` | `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.design.md` | `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.prd.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.design.md` | `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-mainnet-grade-rebuild-2026-04-18.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.design.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.prd.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.design.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-stale-execution-world-recovery-2026-03-12.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md` | `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.design.md` | `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.project.md` |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.design.md` | `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.project.md` |
| `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.prd.md` | `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.design.md` | `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.project.md` |
| `doc/world-simulator/llm/llm-agent-behavior.prd.md` | `doc/world-simulator/llm/llm-agent-behavior.design.md` | `doc/world-simulator/llm/llm-agent-behavior.project.md` |
| `doc/world-simulator/llm/llm-async-openai-responses.prd.md` | `doc/world-simulator/llm/llm-async-openai-responses.design.md` | `doc/world-simulator/llm/llm-async-openai-responses.project.md` |
| `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.prd.md` | `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.design.md` | `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.project.md` |
| `doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.prd.md` | `doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.design.md` | `doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.project.md` |
| `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.prd.md` | `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.design.md` | `doc/world-simulator/llm/llm-decision-provider-standard-loopback-provider-feasibility-2026-03-12.project.md` |
| `doc/world-simulator/llm/llm-dialogue-chat-loop.prd.md` | `doc/world-simulator/llm/llm-dialogue-chat-loop.design.md` | `doc/world-simulator/llm/llm-dialogue-chat-loop.project.md` |
| `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.prd.md` | `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.design.md` | `doc/world-simulator/llm/llm-provider-loopback-http-integration-2026-03-12.project.md` |
| `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.prd.md` | `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.design.md` | `doc/world-simulator/llm/llm-provider-agent-experience-parity-2026-03-12.project.md` |
| `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.prd.md` | `—` | `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.project.md` |
| `doc/world-simulator/llm/llm-factory-strategy-optimization.prd.md` | `doc/world-simulator/llm/llm-factory-strategy-optimization.design.md` | `doc/world-simulator/llm/llm-factory-strategy-optimization.project.md` |
| `doc/world-simulator/llm/llm-industrial-mining-debug-tools.prd.md` | `doc/world-simulator/llm/llm-industrial-mining-debug-tools.design.md` | `doc/world-simulator/llm/llm-industrial-mining-debug-tools.project.md` |
| `doc/world-simulator/llm/llm-lmso29-stability.prd.md` | `doc/world-simulator/llm/llm-lmso29-stability.design.md` | `doc/world-simulator/llm/llm-lmso29-stability.project.md` |
| `doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md` | `doc/world-simulator/llm/llm-multi-scenario-evaluation.design.md` | `doc/world-simulator/llm/llm-multi-scenario-evaluation.project.md` |
| `doc/world-simulator/llm/llm-prompt-effect-receipt.prd.md` | `doc/world-simulator/llm/llm-prompt-effect-receipt.design.md` | `doc/world-simulator/llm/llm-prompt-effect-receipt.project.md` |
| `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.prd.md` | `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.design.md` | `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.project.md` |
| `doc/world-simulator/llm/llm-prompt-system.prd.md` | `doc/world-simulator/llm/llm-prompt-system.design.md` | `doc/world-simulator/llm/llm-prompt-system.project.md` |
| `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.prd.md` | `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.design.md` | `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.project.md` |
| `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.design.md` | `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-industrial-economy-wasm.prd.md` | `doc/world-simulator/m4/m4-industrial-economy-wasm.design.md` | `doc/world-simulator/m4/m4-industrial-economy-wasm.project.md` |
| `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.prd.md` | `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.design.md` | `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.project.md` |
| `doc/world-simulator/m4/m4-power-system.prd.md` | `doc/world-simulator/m4/m4-power-system.design.md` | `doc/world-simulator/m4/m4-power-system.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.design.md` | `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.design.md` | `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.design.md` | `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.design.md` | `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.design.md` | `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.project.md` |
| `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md` | `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.design.md` | `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.project.md` |
| `doc/world-simulator/m4/material-multi-ledger-logistics.prd.md` | `doc/world-simulator/m4/material-multi-ledger-logistics.design.md` | `doc/world-simulator/m4/material-multi-ledger-logistics.project.md` |
| `doc/world-simulator/scenario/agent-frag-initial-spawn-position.prd.md` | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.design.md` | `doc/world-simulator/scenario/agent-frag-initial-spawn-position.project.md` |
| `doc/world-simulator/scenario/asteroid-fragment-renaming.prd.md` | `doc/world-simulator/scenario/asteroid-fragment-renaming.design.md` | `doc/world-simulator/scenario/asteroid-fragment-renaming.project.md` |
| `doc/world-simulator/scenario/chunked-fragment-generation.prd.md` | `doc/world-simulator/scenario/chunked-fragment-generation.design.md` | `doc/world-simulator/scenario/chunked-fragment-generation.project.md` |
| `doc/world-simulator/scenario/frag-resource-balance-onboarding.prd.md` | `doc/world-simulator/scenario/frag-resource-balance-onboarding.design.md` | `doc/world-simulator/scenario/frag-resource-balance-onboarding.project.md` |
| `doc/world-simulator/scenario/fragment-spacing.prd.md` | `doc/world-simulator/scenario/fragment-spacing.design.md` | `doc/world-simulator/scenario/fragment-spacing.project.md` |
| `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.prd.md` | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.design.md` | `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.project.md` |
| `doc/world-simulator/scenario/scenario-files.prd.md` | `doc/world-simulator/scenario/scenario-files.design.md` | `doc/world-simulator/scenario/scenario-files.project.md` |
| `doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md` | `doc/world-simulator/scenario/scenario-power-facility-baseline.design.md` | `doc/world-simulator/scenario/scenario-power-facility-baseline.project.md` |
| `doc/world-simulator/scenario/scenario-seed-locations.prd.md` | `doc/world-simulator/scenario/scenario-seed-locations.design.md` | `doc/world-simulator/scenario/scenario-seed-locations.project.md` |
| `doc/world-simulator/scenario/world-initialization.prd.md` | `doc/world-simulator/scenario/world-initialization.design.md` | `doc/world-simulator/scenario/world-initialization.project.md` |
| `doc/world-simulator/viewer/viewer-minimal-system.prd.md` | `doc/world-simulator/viewer/viewer-minimal-system.design.md` | `doc/world-simulator/viewer/viewer-minimal-system.project.md` |
| `doc/world-simulator/viewer/viewer-module-visual-entities.prd.md` | `doc/world-simulator/viewer/viewer-module-visual-entities.design.md` | `doc/world-simulator/viewer/viewer-module-visual-entities.project.md` |
| `doc/world-simulator/viewer/viewer-rendering-physical-accuracy.prd.md` | `doc/world-simulator/viewer/viewer-rendering-physical-accuracy.design.md` | `doc/world-simulator/viewer/viewer-rendering-physical-accuracy.project.md` |
| `doc/world-simulator/viewer/viewer-2d-3d-clarity-improvement.prd.md` | `doc/world-simulator/viewer/viewer-2d-3d-clarity-improvement.design.md` | `doc/world-simulator/viewer/viewer-2d-3d-clarity-improvement.project.md` |
| `doc/world-simulator/viewer/viewer-2d-visual-polish.prd.md` | `doc/world-simulator/viewer/viewer-2d-visual-polish.design.md` | `doc/world-simulator/viewer/viewer-2d-visual-polish.project.md` |
| `doc/world-simulator/viewer/viewer-3d-commercial-polish.prd.md` | `doc/world-simulator/viewer/viewer-3d-commercial-polish.design.md` | `doc/world-simulator/viewer/viewer-3d-commercial-polish.project.md` |
| `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.prd.md` | `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.design.md` | `doc/world-simulator/viewer/viewer-3d-pause-user-interaction-hold-2026-04-01.project.md` |
| `doc/world-simulator/viewer/viewer-3d-polish-performance.prd.md` | `doc/world-simulator/viewer/viewer-3d-polish-performance.design.md` | `doc/world-simulator/viewer/viewer-3d-polish-performance.project.md` |
| `doc/world-simulator/viewer/viewer-agent-module-rendering.prd.md` | `doc/world-simulator/viewer/viewer-agent-module-rendering.design.md` | `doc/world-simulator/viewer/viewer-agent-module-rendering.project.md` |
| `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md` | `doc/world-simulator/viewer/viewer-agent-quick-locate.design.md` | `doc/world-simulator/viewer/viewer-agent-quick-locate.project.md` |
| `doc/world-simulator/viewer/viewer-agent-size-inspection.prd.md` | `doc/world-simulator/viewer/viewer-agent-size-inspection.design.md` | `doc/world-simulator/viewer/viewer-agent-size-inspection.project.md` |
| `doc/world-simulator/viewer/viewer-auto-focus-capture.prd.md` | `doc/world-simulator/viewer/viewer-auto-focus-capture.design.md` | `doc/world-simulator/viewer/viewer-auto-focus-capture.project.md` |
| `doc/world-simulator/viewer/viewer-auto-select-capture.prd.md` | `doc/world-simulator/viewer/viewer-auto-select-capture.design.md` | `doc/world-simulator/viewer/viewer-auto-select-capture.project.md` |
| `doc/world-simulator/viewer/viewer-bevy-web-runtime.prd.md` | `doc/world-simulator/viewer/viewer-bevy-web-runtime.design.md` | `doc/world-simulator/viewer/viewer-bevy-web-runtime.project.md` |
| `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.prd.md` | `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.design.md` | `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.project.md` |
| `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.prd.md` | `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.design.md` | `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.project.md` |
| `doc/world-simulator/viewer/viewer-chat-enter-send.prd.md` | `doc/world-simulator/viewer/viewer-chat-enter-send.design.md` | `doc/world-simulator/viewer/viewer-chat-enter-send.project.md` |
| `doc/world-simulator/viewer/viewer-chat-ime-cn-input.prd.md` | `doc/world-simulator/viewer/viewer-chat-ime-cn-input.design.md` | `doc/world-simulator/viewer/viewer-chat-ime-cn-input.project.md` |
| `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.prd.md` | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.design.md` | `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.project.md` |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.design.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.project.md` |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.prd.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.design.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.project.md` |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets.prd.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets.design.md` | `doc/world-simulator/viewer/viewer-chat-prompt-presets.project.md` |
| `doc/world-simulator/viewer/viewer-chat-right-panel-polish.prd.md` | `doc/world-simulator/viewer/viewer-chat-right-panel-polish.design.md` | `doc/world-simulator/viewer/viewer-chat-right-panel-polish.project.md` |
| `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.prd.md` | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.design.md` | `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase1-asset-pipeline.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase1-asset-pipeline.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase1-asset-pipeline.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase2-visual-quality-gate.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase2-visual-quality-gate.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase2-visual-quality-gate.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase3-material-style-layer.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase3-material-style-layer.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase3-material-style-layer.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase4-texture-style-layer.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase4-texture-style-layer.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase4-texture-style-layer.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase5-advanced-texture-maps.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase5-advanced-texture-maps.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase5-advanced-texture-maps.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase6-material-variant-preview.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase6-material-variant-preview.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase6-material-variant-preview.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase7-theme-pack-batch-preview.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase7-theme-pack-batch-preview.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase7-theme-pack-batch-preview.project.md` |
| `doc/world-simulator/viewer/viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.prd.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.design.md` | `doc/world-simulator/viewer/viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.project.md` |
| `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.prd.md` | `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.design.md` | `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.project.md` |
| `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-copyable-text.prd.md` | `doc/world-simulator/viewer/viewer-copyable-text.design.md` | `doc/world-simulator/viewer/viewer-copyable-text.project.md` |
| `doc/world-simulator/viewer/viewer-dual-view-2d-3d.prd.md` | `doc/world-simulator/viewer/viewer-dual-view-2d-3d.design.md` | `doc/world-simulator/viewer/viewer-dual-view-2d-3d.project.md` |
| `doc/world-simulator/viewer/viewer-egui-right-panel.prd.md` | `doc/world-simulator/viewer/viewer-egui-right-panel.design.md` | `doc/world-simulator/viewer/viewer-egui-right-panel.project.md` |
| `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-frag-default-rendering.prd.md` | `doc/world-simulator/viewer/viewer-frag-default-rendering.design.md` | `doc/world-simulator/viewer/viewer-frag-default-rendering.project.md` |
| `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.prd.md` | `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.design.md` | `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.project.md` |
| `doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md` | `doc/world-simulator/viewer/viewer-fragment-element-rendering.design.md` | `doc/world-simulator/viewer/viewer-fragment-element-rendering.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.project.md` |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.prd.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.design.md` | `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.project.md` |
| `doc/world-simulator/viewer/viewer-generic-focus-targets.prd.md` | `doc/world-simulator/viewer/viewer-generic-focus-targets.design.md` | `doc/world-simulator/viewer/viewer-generic-focus-targets.project.md` |
| `doc/world-simulator/viewer/viewer-i18n.prd.md` | `doc/world-simulator/viewer/viewer-i18n.design.md` | `doc/world-simulator/viewer/viewer-i18n.project.md` |
| `doc/world-simulator/viewer/viewer-industrial-visual-closure.prd.md` | `doc/world-simulator/viewer/viewer-industrial-visual-closure.design.md` | `doc/world-simulator/viewer/viewer-industrial-visual-closure.project.md` |
| `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.prd.md` | `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.design.md` | `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.project.md` |
| `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.design.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.project.md` |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.design.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.project.md` |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.design.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.project.md` |
| `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.design.md` | `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.project.md` |
| `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.prd.md` | `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.design.md` | `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.project.md` |
| `doc/world-simulator/viewer/viewer-location-depletion-visualization.prd.md` | `doc/world-simulator/viewer/viewer-location-depletion-visualization.design.md` | `doc/world-simulator/viewer/viewer-location-depletion-visualization.project.md` |
| `doc/world-simulator/viewer/viewer-location-fine-grained-rendering.prd.md` | `doc/world-simulator/viewer/viewer-location-fine-grained-rendering.design.md` | `doc/world-simulator/viewer/viewer-location-fine-grained-rendering.project.md` |
| `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-observability-visual-optimization.prd.md` | `doc/world-simulator/viewer/viewer-observability-visual-optimization.design.md` | `doc/world-simulator/viewer/viewer-observability-visual-optimization.project.md` |
| `doc/world-simulator/viewer/viewer-open-world-sandbox-readiness.prd.md` | `doc/world-simulator/viewer/viewer-open-world-sandbox-readiness.design.md` | `doc/world-simulator/viewer/viewer-open-world-sandbox-readiness.project.md` |
| `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md` | `doc/world-simulator/viewer/viewer-overview-map-zoom.design.md` | `doc/world-simulator/viewer/viewer-overview-map-zoom.project.md` |
| `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.prd.md` | `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.design.md` | `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.project.md` |
| `doc/world-simulator/viewer/viewer-release-full-coverage-gate.prd.md` | `doc/world-simulator/viewer/viewer-release-full-coverage-gate.design.md` | `doc/world-simulator/viewer/viewer-release-full-coverage-gate.project.md` |
| `doc/world-simulator/viewer/viewer-release-qa-iteration-loop.prd.md` | `doc/world-simulator/viewer/viewer-release-qa-iteration-loop.design.md` | `doc/world-simulator/viewer/viewer-release-qa-iteration-loop.project.md` |
| `doc/world-simulator/viewer/viewer-right-panel-module-visibility.prd.md` | `doc/world-simulator/viewer/viewer-right-panel-module-visibility.design.md` | `doc/world-simulator/viewer/viewer-right-panel-module-visibility.project.md` |
| `doc/world-simulator/viewer/viewer-selection-details.prd.md` | `doc/world-simulator/viewer/viewer-selection-details.design.md` | `doc/world-simulator/viewer/viewer-selection-details.project.md` |
| `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.prd.md` | `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.design.md` | `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.project.md` |
| `doc/world-simulator/viewer/viewer-texture-inspector.prd.md` | `doc/world-simulator/viewer/viewer-texture-inspector.design.md` | `doc/world-simulator/viewer/viewer-texture-inspector.project.md` |
| `doc/world-simulator/viewer/viewer-visual-release-readiness-hardening-2026-03-01.prd.md` | `doc/world-simulator/viewer/viewer-visual-release-readiness-hardening-2026-03-01.design.md` | `doc/world-simulator/viewer/viewer-visual-release-readiness-hardening-2026-03-01.project.md` |
| `doc/world-simulator/viewer/viewer-visual-upgrade.prd.md` | `doc/world-simulator/viewer/viewer-visual-upgrade.design.md` | `doc/world-simulator/viewer/viewer-visual-upgrade.project.md` |
| `doc/world-simulator/viewer/viewer-wasd-camera-navigation.prd.md` | `doc/world-simulator/viewer/viewer-wasd-camera-navigation.design.md` | `doc/world-simulator/viewer/viewer-wasd-camera-navigation.project.md` |
| `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.prd.md` | `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.design.md` | `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.project.md` |
| `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.prd.md` | `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.design.md` | `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.project.md` |
| `doc/world-simulator/viewer/viewer-web-closure-testing-policy.prd.md` | `doc/world-simulator/viewer/viewer-web-closure-testing-policy.design.md` | `doc/world-simulator/viewer/viewer-web-closure-testing-policy.project.md` |
| `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md` | `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.design.md` | `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.project.md` |
| `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md` | `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.design.md` | `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.project.md` |
| `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md` | `doc/world-simulator/viewer/viewer-web-semantic-test-api.design.md` | `doc/world-simulator/viewer/viewer-web-semantic-test-api.project.md` |
| `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.prd.md` | `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.design.md` | `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.project.md` |
| `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.prd.md` | `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.design.md` | `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.project.md` |
| `doc/world-simulator/viewer/viewer-webgl-deferred-compat-2026-02-24.prd.md` | `doc/world-simulator/viewer/viewer-webgl-deferred-compat-2026-02-24.design.md` | `doc/world-simulator/viewer/viewer-webgl-deferred-compat-2026-02-24.project.md` |
| `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.prd.md` | `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.design.md` | `doc/world-simulator/viewer/viewer-web-runtime-fatal-surfacing-2026-03-12.project.md` |
| `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md` | `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.design.md` | `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.project.md` |
| `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md` | `doc/world-simulator/viewer/viewer-websocket-http-bridge.design.md` | `doc/world-simulator/viewer/viewer-websocket-http-bridge.project.md` |
| `doc/world-simulator/viewer/viewer-visualization-3d.prd.md` | `doc/world-simulator/viewer/viewer-visualization-3d.design.md` | `doc/world-simulator/viewer/viewer-visualization-3d.project.md` |
| `doc/world-simulator/viewer/viewer-visualization.prd.md` | `doc/world-simulator/viewer/viewer-visualization.design.md` | `doc/world-simulator/viewer/viewer-visualization.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；若热点子域进入后仍无首读入口，则继续追加路径级治理。当前 `viewer/README.md` 即为 `viewer/` 的首个已执行子域入口。
- ROUND-002 物理合并（gameplay release）：`viewer-gameplay-release-experience-overhaul` 为主文档，`immersion-phase8~10` 已并入并从仓库移除旧阶段文档（不再保留 archive）。
- ROUND-002 物理合并（live event-driven）：`viewer-live-full-event-driven-phase10-2026-02-27` 为主文档，`phase8/9` 已并入并从仓库移除旧阶段文档（不再保留 archive）。

## 补充验收模板
- `doc/world-simulator/prd/acceptance/provider-agent-parity-scenario-matrix-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-score-card-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-benchmark-protocol-2026-03-12.md`
- `doc/world-simulator/prd/acceptance/provider-agent-parity-aggregation-template-2026-03-12.md`
