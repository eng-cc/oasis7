# world-runtime PRD 文件级索引

审计轮次: 8

## 入口
- 模块 PRD：`doc/world-runtime/prd.md`
- 模块设计总览：`doc/world-runtime/design.md`
- 模块标准执行入口：`doc/world-runtime/project.md`
- 当前高频 runtime 入口：`doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`

## 首读分流
- 想先回答 world-runtime 模块在管什么、哪些边界是当前真值：先读 `doc/world-runtime/prd.md`
- 想先回答当前还在推进什么、阻断在哪里、下一步是什么：先读 `doc/world-runtime/project.md`
- 想直接进入 Docker canonical build / release evidence 主入口：先读 `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`
- 想直接进入 WASM 全局 timing/status/window 或单模块 contract/perf 观测：先读 `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`
- 想直接进入 SDK no_std、共享 wire 与 codec 兼容契约：先读 `doc/world-runtime/wasm/wasm-sdk.prd.md`
- 想直接进入 retention / GC / replay contract：先读 `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`
- 想直接进入链 PoS 时间锚、slot/tick 相位、控制面参数与 restart/replay 边界：先读 `doc/world-runtime/runtime/chain-pos-control-plane.prd.md`
- 想直接进入线上模块发布合法性与 binary-only 边界：先读 `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`
- 想确认 module install target、location 校验、legacy `SelfAgent` 默认值与 snapshot/replay 持久化：先读 `doc/world-runtime/prd.md#7-模块执行市场与历史-gap-合并边界`，详细生命周期读 `doc/world-runtime/module/module-lifecycle.md#稳定实例交易与升级合同`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 热点子域导航
| 子域 | 适合回答的问题 |
| --- | --- |
| `runtime/` | 确定性运行时主链路、数值正确性、retention / GC、replay contract 与存储预算 |
| `wasm/` | Docker canonical build、执行器、模块级 observe runner、SDK、sandbox、ABI 与发布工件治理 |
| `module/` | 模块生命周期、线上发布合法性、模块存储与订阅过滤边界 |
| 根目录入口 | 模块主入口与当前高频导航 |
| `evidence/` | 候选级指标、storage gate、profile consistency 与 soak 采证 |
| `governance/` | 治理事件、收据安全与运行时审计边界；先从 `doc/world-runtime/governance/README.md` 按问题分流 |

当前模块库存与热点二级目录概览以 `./scripts/doc-inventory-report.sh` 输出为准；该报告不保证列出每个子域的精确数量，本索引也不复制数量快照。

## 活跃补充文档
- `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md`：Docker canonical build、receipt、identity 与 release evidence 主入口。
- `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md`：build/executor/router timing、`/v1/chain/status.wasm`、窗口汇总与 module-local observe spec/runner/template 主入口。
- `doc/world-runtime/wasm/wasm-sdk.prd.md`：默认 no_std、共享 Canonical-CBOR wire、codec 错误与 builtin 兼容主入口。
- `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md`：retention / GC / replay contract 与 storage budget 主入口。
- `doc/world-runtime/runtime/chain-pos-control-plane.prd.md`：链 PoS 时间、tick 相位、控制面 status 与恢复合同主入口。
- `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md`：线上模块发布合法性与默认 binary-only 边界主入口。
- `doc/world-runtime/module/player-published-entities.prd.md`：玩家发布实体与模块发布链路衔接入口。
- `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md`：治理收据安全 hardening 主入口。

## governance 子域阅读边界
- `doc/world-runtime/governance/README.md` 是治理子域的唯一首读入口：它把当前模块规格、执行台账、按需设计分册与已完成专题追溯分开，避免根 README / 文件索引重复平铺。
- `audit-export.md` 和 `governance-events.md` 仍分别有 runtime API/test 与模块入口引用；本轮 caller scan 未发现可证明替代它们的现行文档，故保留并降为按需阅读面，而不是删除审计资料。

## WASM 相邻历史入口
| 文档路径 | 当前状态 | 当前阅读入口 |
| --- | --- | --- |
| deterministic pipeline project 的 absorbed historical nightly build-std 记录 | 历史实现证据；记录 nightly build-std 方案，不再作为发布级 canonical build 入口 | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md#已吸收的-historical-nightly-build-std-记录` |
| 早期 WASM build determinism QA guard（已退役） | 历史脚本护栏和编译期拦截只从 Git/GitHub task evidence 追溯，不替代当前 release evidence / Docker canonical 口径 | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md` 与 `doc/testing/prd.md` |
| `doc/testing/evidence/viewer-wasm-only-runtime-proof-2026-05-13.md` | dated evidence；只证明当轮 viewer wasm-only runtime proof，不替代 required/full gate | `doc/world-simulator/viewer/README.md` 与当前 viewer/testing task truth |

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫全部 runtime / wasm / module 专题表。
- `evidence/`、模板与 checklist 文档继续保留可检索性，但默认不和活跃专题三件套同屏平铺；旧 2026-03 runtime handoff root 文档已退役删除，追溯从正式 evidence、runtime storage topic project 与 GitHub task issue evidence comments 进入。
- 完整活跃专题清单继续保留在下方，用于精确文件名检索和互链可达性。
- WASM 相关的跨目录历史文档只保留在“相邻历史入口”或对应模块索引中；除非 owner 明确恢复为当前任务入口，不再作为 world-runtime 默认首读面。

## 覆盖规则
- 纳入规则：纳入 `doc/world-runtime/{runtime,wasm,module,governance}/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的活跃专题三件套。
- 活跃补充：仍被模块 PRD / 项目态直接引用的高频专题，可在“活跃补充文档”区定向列出，但不并入 evidence / template / checklist / handoff 清单。
- 排除规则：不纳入 `doc/world-runtime/evidence/**`、`doc/world-runtime/templates/**`、`doc/world-runtime/checklists/**` 与 legacy redirect 的非三件套材料。
- 按需进入：evidence、候选级采证、模板与 checklist 继续保留可检索性；除非重新成为当前 owner 的直接入口，否则不进入默认首屏。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md` | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.design.md` | `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md` |
| `doc/world-runtime/module/agent-default-modules.prd.md` | `doc/world-runtime/module/agent-default-modules.design.md` | `doc/world-runtime/module/agent-default-modules.project.md` |
| `doc/world-runtime/module/player-published-entities.prd.md` | `doc/world-runtime/module/player-published-entities.design.md` | `doc/world-runtime/module/player-published-entities.project.md` |
| `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md` | `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.design.md` | `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md` |
| `doc/world-runtime/module/module-subscription-filters.prd.md` | `doc/world-runtime/module/module-subscription-filters.design.md` | `doc/world-runtime/module/module-subscription-filters.project.md` |
| `doc/world-runtime/runtime/runtime-numeric-safety.prd.md` | `doc/world-runtime/runtime/runtime-numeric-safety.design.md` | `doc/world-runtime/runtime/runtime-numeric-safety.project.md` |
| `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md` | `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md` | `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md` |
| `doc/world-runtime/runtime/chain-pos-control-plane.prd.md` | `doc/world-runtime/runtime/chain-pos-control-plane.design.md` | `doc/world-runtime/runtime/chain-pos-control-plane.project.md` |
| `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.prd.md` | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.design.md` | `doc/world-runtime/wasm/wasm-deterministic-build-pipeline.project.md` |
| `doc/world-runtime/wasm/wasm-executor.prd.md` | `doc/world-runtime/wasm/wasm-executor.design.md` | `doc/world-runtime/wasm/wasm-executor.project.md` |
| `doc/world-runtime/wasm/wasm-observability-timing-metrics.prd.md` | `doc/world-runtime/wasm/wasm-observability-timing-metrics.design.md` | `doc/world-runtime/wasm/wasm-observability-timing-metrics.project.md` |
| `doc/world-runtime/wasm/wasm-sdk.prd.md` | `doc/world-runtime/wasm/wasm-sdk.design.md` | `doc/world-runtime/wasm/wasm-sdk.project.md` |

## 证据 / 模板 / 清单 / 交接补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/world-runtime/evidence/runtime-version-candidate-evidence-2026-03-11.md` | `evidence` | 版本候选 runtime evidence 汇总 |
| `doc/world-runtime/evidence/runtime-version-candidate-soak-evidence-2026-03-11.md` | `evidence` | 版本候选 soak evidence 汇总 |
| `doc/world-runtime/evidence/runtime-launcher-profile-consistency-2026-03-11.md` | `evidence` | launcher profile consistency 采证 |
| `doc/world-runtime/templates/runtime-release-gate-metrics-template.md` | `template` | release gate 指标模板 |
| `doc/world-runtime/templates/runtime-security-numeric-regression-template.md` | `template` | 安全与数值语义回归模板 |
| `doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md` | `checklist` | runtime 核心边界验收清单 |

已退役删除的 2026-03 runtime P0 candidate / T7.2 / T7.3 / T7.4 role handoff root 文档不再作为补充入口；对应结论从上方 evidence、`doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md` 与 GitHub task issue evidence comments 追溯。

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法分流时，才进入后续路径级治理。
