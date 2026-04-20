# world-runtime PRD 文件级索引

审计轮次: 7

更新时间：2026-04-20

## 入口
- 模块 PRD：`doc/world-runtime/prd.md`
- 模块设计总览：`doc/world-runtime/design.md`
- 模块标准执行入口：`doc/world-runtime/project.md`
- 当前高频 runtime 入口：`doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`

## 首读分流
- 想先回答 world-runtime 模块在管什么、哪些边界是当前真值：先读 `doc/world-runtime/prd.md`
- 想先回答当前还在推进什么、阻断在哪里、下一步是什么：先读 `doc/world-runtime/project.md`
- 想直接进入 Docker canonical build / release evidence 主入口：先读 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 想直接进入 WASM build / executor / router 的观测与耗时指标：先读 `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- 想直接进入单模块标准化功能/性能观测入口：先读 `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md`
- 想直接进入 retention / GC / replay contract：先读 `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- 想直接进入线上模块发布合法性与 binary-only 边界：先读 `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-20）
- `doc/world-runtime/`：121 份文件
- `doc/world-runtime/runtime/`：55 份文件
- `doc/world-runtime/wasm/`：25 份文件
- `doc/world-runtime/module/`：16 份文件
- 根目录入口与 handoff：9 份文件
- `doc/world-runtime/evidence/`：6 份文件
- `doc/world-runtime/governance/`：5 份文件
- `doc/world-runtime/templates/`：2 份文件
- `doc/world-runtime/checklists/`：1 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `runtime/` | 55 | 确定性运行时主链路、数值正确性、retention / GC、replay contract 与存储预算 |
| `wasm/` | 25 | Docker canonical build、执行器、模块级 observe runner、SDK、sandbox、ABI 与发布工件治理 |
| `module/` | 16 | 模块生命周期、线上发布合法性、模块存储与订阅过滤边界 |
| 根目录入口与 handoff | 9 | 模块主入口、候选收口交接与当前高频导航 |
| `evidence/` | 6 | 候选级指标、storage gate、profile consistency 与 soak 采证 |
| `governance/` | 5 | 治理事件、收据安全与运行时审计边界 |

## 活跃补充文档
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`：Docker canonical build、receipt、identity 与 release evidence 主入口。
- `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`：build/executor/router timing、`/v1/chain/status.wasm` 与外部窗口汇总主入口。
- `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md`：module-local observe spec、共享 runner、wrapper script 与模板化接入主入口。
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`：retention / GC / replay contract 与 storage budget 主入口。
- `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`：线上模块发布合法性与默认 binary-only 边界主入口。
- `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md`：玩家发布实体与模块发布链路衔接入口。
- `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md`：治理收据安全 hardening 主入口。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫全部 runtime / wasm / module 专题表。
- `evidence/`、模板、checklist 与 handoff 文档继续保留可检索性，但默认不和活跃专题三件套同屏平铺。
- 完整活跃专题清单继续保留在下方，用于精确文件名检索和互链可达性。

## 覆盖规则
- 纳入规则：纳入 `doc/world-runtime/{runtime,wasm,module,governance}/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的活跃专题三件套。
- 活跃补充：仍被模块 PRD / 项目态直接引用的高频专题，可在“活跃补充文档”区定向列出，但不并入 evidence / template / checklist / handoff 清单。
- 排除规则：不纳入 `doc/world-runtime/evidence/**`、`doc/world-runtime/templates/**`、`doc/world-runtime/checklists/**` 与 handoff / legacy redirect 的非三件套材料。
- 按需进入：evidence、候选级采证、模板、checklist 与 handoff 继续保留可检索性；除非重新成为当前 owner 的直接入口，否则不进入默认首屏。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md` | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.design.md` | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md` |
| `doc/world-runtime/module/agent-default-modules.prd.md` | `doc/world-runtime/module/agent-default-modules.design.md` | `doc/world-runtime/module/agent-default-modules.project.md` |
| `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md` | `doc/world-runtime/module/player-published-entities-2026-03-05.design.md` | `doc/world-runtime/module/player-published-entities-2026-03-05.project.md` |
| `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md` | `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.design.md` | `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md` |
| `doc/world-runtime/module/module-storage.prd.md` | `doc/world-runtime/module/module-storage.design.md` | `doc/world-runtime/module/module-storage.project.md` |
| `doc/world-runtime/module/module-subscription-filters.prd.md` | `doc/world-runtime/module/module-subscription-filters.design.md` | `doc/world-runtime/module/module-subscription-filters.project.md` |
| `doc/world-runtime/runtime/bootstrap-power-modules.prd.md` | `doc/world-runtime/runtime/bootstrap-power-modules.design.md` | `doc/world-runtime/runtime/bootstrap-power-modules.project.md` |
| `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md` | `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.design.md` | `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md` |
| `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md` | `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md` | `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.design.md` | `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md` |
| `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.prd.md` | `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.design.md` | `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.project.md` |
| `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md` | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md` | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md` |
| `doc/world-runtime/wasm/wasm-executor.prd.md` | `doc/world-runtime/wasm/wasm-executor.design.md` | `doc/world-runtime/wasm/wasm-executor.project.md` |
| `doc/world-runtime/wasm/wasm-module-observability-standardization.prd.md` | `doc/world-runtime/wasm/wasm-module-observability-standardization.design.md` | `doc/world-runtime/wasm/wasm-module-observability-standardization.project.md` |
| `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md` | `doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md` | `doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md` |
| `doc/world-runtime/wasm/wasm-sandbox-security-hardening.prd.md` | `doc/world-runtime/wasm/wasm-sandbox-security-hardening.design.md` | `doc/world-runtime/wasm/wasm-sandbox-security-hardening.project.md` |
| `doc/world-runtime/wasm/wasm-sdk-no-std.prd.md` | `doc/world-runtime/wasm/wasm-sdk-no-std.design.md` | `doc/world-runtime/wasm/wasm-sdk-no-std.project.md` |
| `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.prd.md` | `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.design.md` | `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.project.md` |

## 证据 / 模板 / 清单 / 交接补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/world-runtime/evidence/runtime-version-candidate-evidence-2026-03-11.md` | `evidence` | 版本候选 runtime evidence 汇总 |
| `doc/world-runtime/evidence/runtime-version-candidate-soak-evidence-2026-03-11.md` | `evidence` | 版本候选 soak evidence 汇总 |
| `doc/world-runtime/evidence/runtime-launcher-profile-consistency-2026-03-11.md` | `evidence` | launcher profile consistency 采证 |
| `doc/world-runtime/templates/runtime-release-gate-metrics-template.md` | `template` | release gate 指标模板 |
| `doc/world-runtime/templates/runtime-security-numeric-regression-template.md` | `template` | 安全与数值语义回归模板 |
| `doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md` | `checklist` | runtime 核心边界验收清单 |
| `doc/world-runtime/runtime-p0-candidate-evidence-handoff-2026-03-10.md` | `handoff` | runtime 候选证据交接入口 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法分流时，才进入后续路径级治理。
