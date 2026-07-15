# testing PRD 文件级索引

审计轮次: 10

更新时间：2026-06-26

## 入口
- 模块 PRD：`doc/testing/prd.md`
- 模块设计总览：`doc/testing/design.md`
- 模块标准执行入口：`doc/testing/project.md`
- 当前 QA 阻断与活跃覆盖缺口：`doc/testing/project.md`

## 首读分流
- 想先回答 testing 模块覆盖哪些测试层级、证据与门禁边界：先读 `doc/testing/prd.md`
- 想判断好玩性证据、`L4A/L4B/L5` 边界、角色 subagent review 或 simulated player persona 的 canonical topic：先读 `doc/testing/governance/README.md`
- 想直接在一个 worktree 里准备完整 `L4A + L4B` 执行包：先经 `doc/testing/governance/README.md` 确认所需证据层，再读 `testing-manual.md` 的 `L4A/L4B/L5` 段落并执行 `./scripts/prepare-playability-l4-review.sh`；正式 `L4B` embodied-agent run 再由 `./scripts/run-playability-l4b-agent.sh --l4-manifest <artifact>/manifest.json` 收口。
- 想执行 Web UI、Playwright、public-testnet attach 或模型视觉评审手册：先读 `doc/testing/manual/README.md`，再按问题进入对应 manual。
- 想快速判断“现有性能测试覆盖到哪、哪些功能面最值得补性能测试、哪些更适合进 scoped gate”：先读 `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`
- 想先回答 world state sync、commit closure、state-sync bundle、API/viewer projection，或 `module_required/module_full/integration_required/release_full` 如何定档：先读 `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md`，再下钻 S9/S10 执行套件。
- 想先回答当前在推进什么、哪些测试治理任务或 QA 阻断仍在影响收口：先读 `doc/testing/project.md`
- 想直接决定要跑哪套测试或按步骤执行：先读 `testing-manual.md`；涉及专项操作再进入 `doc/testing/manual/README.md`。
- 想先进入 `evidence` 热点子域，并按 release gate / hosted access / legacy p2p network rehearsal / governance drill / claim-audit 问题分流：先读 `doc/testing/evidence/README.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度说明
- 本索引不维护静态文件数量；数量快照容易漂移，只能作为历史审计材料，不作为默认阅读顺序或覆盖完整性的依据。
- 需要重新统计时，以 `./scripts/doc-inventory-report.sh` 的 module density / hotspot 状态为准，并把正式报告结果写入具体审计任务；`find doc/testing -type f` / `rg --files doc/testing` 只可作为本地探索辅助，不能替代正式 inventory 状态。

## 热点子域导航
| 子域 | 适合回答的问题 |
| --- | --- |
| `evidence/` | 发布证据、趋势基线与审计留痕；当前已补 `evidence/README.md` 作为热点子域入口 |
| `ci/` | CI、wasm determinism、tiering、required check 保护 |
| `longrun/` | 长稳、chaos、soak、world state sync / commit closure 与在线稳定性 |
| `launcher/` | 启动器链路测试、playtest 与配置自动接线 |
| `governance/` | 质量趋势、release-gate 指标、审计检查与 playability 证据治理；先读 `doc/testing/governance/README.md` 再按问题下钻 |
| `templates/` | 证据包、报告、模型视觉评审卡与检查清单模板；默认按需进入 |
| `performance/` | runtime / viewer 性能观测与方法学 |
| `manual/` | 系统测试手册分册、Web UI / Playwright / public-testnet attach 与模型视觉评审；先读 `doc/testing/manual/README.md` |
| `chaos-plans/` | 专项 chaos plan 入口 |

## 活跃补充文档
- `doc/testing/governance/README.md`：`governance/` 热点子域 landing page，按质量门禁、playability 证据、L4 分层、role review 与 persona 问题分流读者。
- `doc/testing/evidence/README.md`：`evidence/` 热点子域 landing page，按 release gate、hosted access、legacy p2p network rehearsal、governance drill 与 claim/audit 分流读者。
- `testing-manual.md`：仓库级系统测试手册，不并入下方模块 PRD 三件套长表。
- `doc/testing/manual/README.md`：`manual/` 子树 landing page，按 Web UI 页面闭环、真实 Playwright 流程、formal public-testnet attach 与模型视觉评审分流；具体 manual 保留 operator 真值，不并入下方模块 PRD 三件套长表。
- `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`：按 surface 汇总现有性能覆盖、当前缺口、建议补测和建议 tier 的速查表。
- Provider dual-mode 历史 blocker 已收敛到 `doc/world-simulator/llm/llm-provider-agent-dual-mode-2026-03-16.project.md` 与复签 evidence；当前 testing 阻断只从 `doc/testing/project.md` 进入。

Manual convergence note: Web UI manual 当前按 driver / evidence mode 分流，不再作为单一 PRD/design/project 三件套维护。`web-ui-agent-browser-closure-manual.*` 保留 Viewer 页面通用闭环的 PRD / project / manual 真值；`web-ui-playwright-closure-manual.manual.md` 保留真实本地栈 + 玩家 UI 操作流程的 `PWT-###` 系列入口；`web-ui-playwright-closure-manual.design.md` 仅作为历史/共享设计 companion 追溯，不再把两条 manual surface 合并成一个 normal triplet。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者从第一行开始顺扫完整长表。
- README 不再平铺“近期专题”；专题清单继续保留在下方，用于精确文件名检索和互链可达性。
- `evidence/README.md` 负责最高密度热点子域的首读分流；完整长表继续由本页与目标 evidence 文件承担。
- 手册、blocker、evidence 与 template 等 supporting / 审计材料继续保留可检索性，但不并入模块 PRD 三件套长表。

## 覆盖规则
- 纳入规则：纳入 `doc/testing/**` 下所有 `*.prd.md` 与同名 `*.project.md`。
- 活跃补充：`testing-manual.md`、`*.manual.md` 与仍被当前模块 PRD / 项目态直接引用的 blocker/supporting spec，可在“活跃补充文档”区定向列出，但不并入下方三件套长表。
- 排除规则：不纳入 `doc/devlog/**`、`doc/testing/evidence/**`、`doc/testing/templates/**` 与其他非 PRD 配对文档。
- 按需进入：evidence、template、blocker、closure 说明与历史归档继续保留可检索性；除非它们重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。

## 专题清单（含历史完成项，按文件名精确检索）
本表用于互链可达性和 traceability，不代表每一行都是默认首读或当前活跃执行面；已完成的规划三件套应先读其 redirect / historical status，再进入当前 canonical 手册或 topic 文档。

2026-07-15 governance note: 早期 launcher 专题三件套与已完成的 design/project 拆解已退役删除；当前 launcher / playtest / Web UI 闭环入口收敛到 `testing-manual.md`、`doc/testing/launcher/README.md`、两份 launcher 现行 PRD、Web UI manual、GitHub task issue evidence comments 与 Git 历史。

Viewer performance historical/provenance note: `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.*` 是已完成的 legacy methodology closure，依赖已移除的 `viewer-owr4-stress` / old `perf_probe` 路径；当前 Viewer performance 入口为 `doc/testing/performance/performance-coverage-gap-matrix-2026-06-09.md`、`doc/testing/performance/viewer-current-web-performance-harness-2026-06-02.prd.md` 与 `testing-manual.md` 的 Viewer performance probe 指引。Delete-candidate boundary: focused `rg` 确认只剩 self/provenance 引用前，保留该 legacy triplet 作为历史证据；不得把它作为当前 operator guidance。

Builtin wasm m1 determinism historical/provenance note: `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.*` 是已完成的 historical triplet，旧 independent multi-runner / host-native 对账口径已并入当前 Docker canonical `wasm-determinism-gate`。当前 builtin wasm 独立 gate 入口为 `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md` 与 `.github/workflows/wasm-determinism-gate.yml`；删除该 legacy triplet 前，需 focused `rg` 确认只剩 self/provenance 引用。

Wasm build determinism guard historical/provenance note: `doc/testing/governance/wasm-build-determinism-guard.*` 是已完成的 historical QA gate substrate，记录早期脚本护栏、污染环境变量拦截和 workspace 编译期风险前置。当前 WASM 发布级 canonical build / release evidence 入口为 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`；required / full gate 归属继续以 `doc/testing/prd.md`、`doc/testing/project.md`、`testing-manual.md` 与当前 task truth 为准。删除该 legacy triplet 前，需 focused `rg` 确认只剩 self/provenance 引用。

| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md` | `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.design.md` | `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.project.md` |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.prd.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.design.md` | `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.project.md` |
| `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.prd.md` | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.design.md` | `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.project.md` |
| `doc/testing/ci/ci-test-coverage.prd.md` | `doc/testing/ci/ci-test-coverage.design.md` | `doc/testing/ci/ci-test-coverage.project.md` |
| `doc/testing/ci/ci-testcase-tiering.prd.md` | `doc/testing/ci/ci-testcase-tiering.design.md` | `doc/testing/ci/ci-testcase-tiering.project.md` |
| `doc/testing/ci/ci-tiered-execution.prd.md` | `doc/testing/ci/ci-tiered-execution.design.md` | `doc/testing/ci/ci-tiered-execution.project.md` |
| `doc/testing/ci/ci-wasm32-target-install.prd.md` | `doc/testing/ci/ci-wasm32-target-install.design.md` | `doc/testing/ci/ci-wasm32-target-install.project.md` |
| `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md` | `doc/testing/governance/llm-skip-tick-ratio-metric.design.md` | `doc/testing/governance/llm-skip-tick-ratio-metric.project.md` |
| `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md` | `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md` | `doc/testing/governance/playability-evidence-stack-2026-05-06.project.md` |
| `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md` | `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.design.md` | `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md` |
| `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md` | `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.design.md` | `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.project.md` |
| `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md` | `doc/testing/governance/playability-subagent-review-system-2026-05-06.design.md` | `doc/testing/governance/playability-subagent-review-system-2026-05-06.project.md` |
| `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.prd.md` | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.design.md` | `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.project.md` |
| `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md` | `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.design.md` | `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.project.md` |
| `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.prd.md` | `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.design.md` | `doc/testing/governance/testing-quality-trend-tracking-2026-03-11.project.md` |
| `doc/testing/launcher/launcher-bundle-first-playtest-entry-2026-03-12.prd.md` | 当前设计决策与验收记录已并入 PRD | 已完成项目拆解见 Git 历史与 GitHub task evidence |
| `doc/testing/launcher/launcher-manual-test-checklist-2026-03-10.prd.md` | 当前执行设计与分层清单已并入 PRD | 已完成项目拆解见 Git 历史与 GitHub task evidence |
| `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md` | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.design.md` | `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md` |
| `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md` | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.design.md` | `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.project.md` |
| `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md` | `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md` | `doc/testing/longrun/game-world-state-sync-commit-closure-2026-06-26.project.md` |
| `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.prd.md` | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.design.md` | `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.project.md` |
| `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.prd.md` | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.design.md` | `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md` |
| `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.prd.md` | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.design.md` | `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md` |
| `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md` | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.design.md` | `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.project.md` |
| `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.prd.md` | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.design.md` | `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.project.md` |
| `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | `doc/testing/longrun/s10-five-node-real-game-soak.design.md` | `doc/testing/longrun/s10-five-node-real-game-soak.project.md` |
| `doc/testing/manual/systematic-application-testing-manual.prd.md` | `doc/testing/manual/systematic-application-testing-manual.design.md` | `doc/testing/manual/systematic-application-testing-manual.project.md` |
| `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.prd.md` | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.design.md` | `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.project.md` |
| `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.prd.md` | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.design.md` | `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.project.md` |
| `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md` | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.design.md` | `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.project.md` |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
