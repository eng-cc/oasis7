# testing PRD 文件级索引

审计轮次: 7

更新时间：2026-04-10

## 入口
- 模块 PRD：`doc/testing/prd.md`
- 模块设计总览：`doc/testing/design.md`
- 模块标准执行入口：`doc/testing/project.md`
- 当前 QA 阻断摘要：`doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`

## 首读分流
- 想先回答 testing 模块覆盖哪些测试层级、证据与门禁边界：先读 `doc/testing/prd.md`
- 想先回答当前在推进什么、哪些测试治理任务或 QA 阻断仍在影响收口：先读 `doc/testing/project.md`
- 想直接决定要跑哪套测试或按步骤执行：先读 `testing-manual.md` 与 `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-10）
- `doc/testing/`：182 份文件
- `doc/testing/evidence/`：49 份文件
- `doc/testing/ci/`：33 份文件
- `doc/testing/longrun/`：24 份文件
- `doc/testing/launcher/`：18 份文件
- `doc/testing/governance/`：16 份文件
- `doc/testing/templates/`：15 份文件
- `doc/testing/performance/`：12 份文件
- `doc/testing/manual/`：7 份文件
- `doc/testing/chaos-plans/`：1 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `evidence/` | 49 | 发布证据、趋势基线与审计留痕；默认按需进入 |
| `ci/` | 33 | CI、wasm determinism、tiering、required check 保护 |
| `longrun/` | 24 | 长稳、chaos、soak 与在线稳定性 |
| `launcher/` | 18 | 启动器链路测试、playtest 与配置自动接线 |
| `governance/` | 16 | 质量趋势、release-gate 指标与审计检查 |
| `templates/` | 15 | 证据包、报告与检查清单模板；默认按需进入 |
| `performance/` | 12 | runtime / viewer 性能观测与方法学 |
| `manual/` | 7 | 系统测试手册分册与 Web UI 闭环 manual |
| `chaos-plans/` | 1 | 专项 chaos plan 入口 |

## 活跃补充文档
- `testing-manual.md`：仓库级系统测试手册，不并入下方模块 PRD 三件套长表。
- `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`：Web UI 闭环 canonical 操作手册，不并入下方模块 PRD 三件套长表。
- `doc/testing/provider-dual-mode-t4-blocker-2026-03-16.md`：当前 QA 阻断摘要，适合在判断 provider 双模式收口风险时定向进入。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“近期专题”；完整清单继续保留在下方，用于精确文件名检索和互链可达性。
- 手册、blocker、evidence 与 template 等 supporting / 审计材料继续保留可检索性，但不并入模块 PRD 三件套长表。

## 覆盖规则
- 纳入规则：纳入 `doc/testing/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：`testing-manual.md`、`*.manual.md` 与仍被当前模块 PRD / 项目态直接引用的 blocker/supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**`、`doc/testing/evidence/**`、`doc/testing/templates/**` 与其他非 PRD 配对文档。
- 按需进入：evidence、template、blocker、closure 说明与历史归档继续保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。

## 完整活跃专题清单（按文件名精确检索）
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
