# testing PRD 文件级索引

审计轮次: 7

更新时间：2026-03-22

## 入口
- 模块 PRD：`doc/testing/prd.md`
- 模块设计总览：`doc/testing/design.md`
- 模块标准执行入口：`doc/testing/project.md`
- 模块兼容项目管理：`doc/testing/project.md`
- 当前 QA 阻断摘要：`doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`
- builtin wasm CI 现行口径统一以 `.github/workflows/wasm-determinism-gate.yml` 为准；旧 `multi-runner` / `hash-drift-hardening` 文件名已降级为原地归档提示，当前活跃入口以下表为准。

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.prd.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.design.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.project.md` |
| `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md` | `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.design.md` | `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.project.md` |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.prd.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.design.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.project.md` |
| `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.prd.md` | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.design.md` | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.project.md` |
| `doc/testing/ci/ci-test-coverage.prd.md` | `doc/testing/ci/ci-test-coverage.design.md` | `doc/testing/ci/ci-test-coverage.project.md` |
| `doc/testing/ci/ci-testcase-tiering.prd.md` | `doc/testing/ci/ci-testcase-tiering.design.md` | `doc/testing/ci/ci-testcase-tiering.project.md` |
| `doc/testing/ci/ci-tiered-execution.prd.md` | `doc/testing/ci/ci-tiered-execution.design.md` | `doc/testing/ci/ci-tiered-execution.project.md` |
| `doc/testing/ci/ci-wasm32-target-install.prd.md` | `doc/testing/ci/ci-wasm32-target-install.design.md` | `doc/testing/ci/ci-wasm32-target-install.project.md` |
| `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md` | `doc/testing/governance/llm-skip-tick-ratio-metric.design.md` | `doc/testing/governance/llm-skip-tick-ratio-metric.project.md` |
| `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.prd.md` | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.design.md` | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.project.md` |
| `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md` | `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.design.md` | `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.project.md` |
| `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md` | `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.design.md` | `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.project.md` |
| `doc/testing/governance/wasm-build-determinism-guard.prd.md` | `doc/testing/governance/wasm-build-determinism-guard.design.md` | `doc/testing/governance/wasm-build-determinism-guard.project.md` |
| `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.prd.md` | `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.design.md` | `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.project.md` |
| `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md` | `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.design.md` | `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.project.md` |
| `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md` | `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.design.md` | `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.project.md` |
| `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md` | `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.design.md` | `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.project.md` |
| `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md` | `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.design.md` | `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.project.md` |
| `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.prd.md` | `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.design.md` | `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md` |
| `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md` | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.design.md` | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md` |
| `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md` | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.design.md` | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.project.md` |
| `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.prd.md` | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.design.md` | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.project.md` |
| `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.prd.md` | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.design.md` | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md` |
| `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.prd.md` | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.design.md` | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md` |
| `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md` | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.design.md` | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.project.md` |
| `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.prd.md` | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.design.md` | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.project.md` |
| `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | `doc/testing/longrun/s10-five-node-real-game-soak.design.md` | `doc/testing/longrun/s10-five-node-real-game-soak.project.md` |
| `doc/testing/manual/systematic-application-testing-manual.prd.md` | `doc/testing/manual/systematic-application-testing-manual.design.md` | `doc/testing/manual/systematic-application-testing-manual.project.md` |
| `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` | `doc/testing/manual/web-ui-playwright-closure-manual.design.md` | `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md` |
| `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.prd.md` | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.design.md` | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.project.md` |
| `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.prd.md` | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.design.md` | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.project.md` |
| `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md` | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.design.md` | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.project.md` |
| `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.prd.md` | `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.design.md` | `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
