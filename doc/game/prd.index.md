# game PRD 文件级索引

区域设施阅读顺序：先从 `doc/product/README.md` 选择“大世界基础设施”，再由其产品 PRD 下钻到本索引中的 `micro_depot` 专题。`PRD-GAME-016` 继续是该设施玩法与经济边界的专业域权威。
审计轮次: 12

更新时间：2026-07-06

## 入口
- 模块 PRD：`doc/game/prd.md`
- 模块设计总览：`doc/game/design.md`
- 模块标准执行入口：`doc/game/project.md`
- gameplay 子域入口：`doc/game/gameplay/README.md`
- 核心玩法骨架入口：`doc/game/gameplay/gameplay-top-level-design.prd.md`

## 首读分流
- 想先回答 game 模块当前目标态、阶段判断与完成定义：先读 `doc/game/prd.md`
- 想先回答当前还在推进什么、阻断在哪里、下一步做什么：先读 `doc/game/project.md`
- 想先进入 gameplay 热点子域，而不是直接面对完整 gameplay 文档长表：先读 `doc/game/gameplay/README.md`
- 想先理解核心玩法骨架，而不是逐篇翻 gameplay 长表：先读 `doc/game/gameplay/gameplay-top-level-design.prd.md`
- 想先看首局与持续游玩的产品承诺：先读 `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`；exact gameplay 合同读 `doc/game/gameplay/gameplay-top-level-design.prd.md`，当前 verdict 读 `doc/game/project.md`
- 想先看“间接控制为什么仍然要让玩家感觉自己在控制”：先读 `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
- 想先看“成熟世界里小玩家为什么不必立刻依附 major power，仍能继续形成 leverage”：先读 `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`，再读 gameplay 顶层合同
- 想先回答“1cm 物理世界”和“当前为什么不是 Minecraft 式逐块玩法”之间的边界：先读 `doc/product/world-rules-core-gameplay/prd.md` 的产品承诺，再读 `doc/game/gameplay/gameplay-top-level-design.prd.md` 的玩法合同
- 想先回答“可编程区域设施如何作为中后期区域专业化能力落地，而不变成自由建造或任意 WASM 上传”：先读 `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`
- 想先看访问模式、受控试玩与 release readiness：读 `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md`；当前执行状态看 `doc/game/project.md` 与对应 round execution record
- 想继续按文件名、专题或补充材料下钻：使用下方热点子域导航与补充入口；当前文件库存统一以 `./scripts/doc-inventory-report.sh` 为准，本页不维护容易漂移的数量快照

## 热点子域导航
| 子域 | 适合回答的问题 |
| --- | --- |
| `gameplay/` 正式专题 | 核心玩法骨架、留存修复、preview/beta gate、claim economy、治理、agency 合同、mature-world 小玩家承接与可编程区域设施 |
| `gameplay/` 补充材料 | runbook、evidence、checklist 与跨角色执行留痕 |
| 模块根入口 | 模块目标态、执行台账、设计总览与文件级精确检索 |

## 活跃补充文档
- `doc/game/gameplay/README.md`：`gameplay/` 热点子域 landing page，适合先做簇级分流，再决定进入玩法骨架、留存、agency、preview/beta gate 或 economy/claim 专题。
- `doc/game/gameplay/gameplay-top-level-design.prd.md`：核心玩法骨架主入口。
- `doc/product/world-rules-core-gameplay/first-session-and-continuation.prd.md`：首局、后引导、首次持续能力与失败恢复的产品承诺；专业玩法合同见 `doc/game/gameplay/gameplay-top-level-design.prd.md`，执行状态见 `doc/game/project.md`。
- `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`：间接控制下的 accepted intent、主因果、打断重排与续玩恢复合同主入口。
- `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`：mature-world 小玩家承接、受保护 first win、专业化与局部影响力的产品主入口。
- `doc/product/world-rules-core-gameplay/prd.md` 与 `doc/game/gameplay/gameplay-top-level-design.prd.md`：分别承载物理尺度/间接控制的产品承诺，以及玩法侧动作粒度与表现层夸张边界。
- `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md`：访问模式、统一候选门禁与公开 claim 升阶入口。
- `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`：近期高频经济规则与 token 成本边界主入口。
- `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md`：WASM-backed 可编程区域设施、micro_depot quote/receipt、upkeep 与区域专业化边界主入口。
- `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`：restricted grant 发放、撤销、过期与 incident 处理 runbook。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫所有 gameplay 专题三件套与补充材料。
- runbook、evidence、checklist 与仍承担当前追溯职责的补充材料继续保留可检索性，但默认不与主专题三件套同屏平铺成长名单；一次性 handoff brief 若已被 topic project / evidence / GitHub task issue evidence comments 覆盖，应退役删除。
- 完整活跃专题清单与补充入口继续保留在下方，用于精确文件名检索和互链可达性。

## 覆盖规则
- 纳入规则：纳入 `doc/game/gameplay/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的活跃专题三件套。
- 活跃补充：仍被模块 PRD / 项目态直接引用、且承担当前阶段判断或执行入口职责的 runbook / handoff / evidence，可在“活跃补充文档”或补充入口表中定向列出。
- 排除规则：补充材料继续保留检索能力，但除非重新成为默认首读入口，否则不进入首屏长表。
- 按需进入：当 README 与 `project.md` 已经能完成首读分流时，本页只承担精确检索与补充路由职责。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md` | `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.design.md` | `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.project.md` |
| `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md` | `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.design.md` | `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md` |
| `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md` | `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.design.md` | `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md` |
| `doc/game/gameplay/gameplay-top-level-design.prd.md` | `doc/game/gameplay/gameplay-top-level-design.design.md` | `doc/game/gameplay/gameplay-top-level-design.project.md` |
| `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.prd.md` | `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.design.md` | `doc/game/gameplay/gameplay-wasm-backed-regional-infrastructure-micro-depot-2026-06-22.project.md` |

## 历史 closure / provenance 入口
| 历史 closure 专题 | 当前追溯入口 |
| --- | --- |
| lifecycle rules、war/governance/crisis/meta、module-driven production 历史 closure（正文已退役） | 玩家合同与生产落地证据已收敛到 `doc/game/gameplay/gameplay-top-level-design.prd.md#82-评审输入包`、`doc/game/gameplay/gameplay-top-level-design.project.md#t3-工程落地拆解下阶段`、战争/政治数值基线及 `doc/world-runtime/prd.md#gameplay-生命周期协议边界`；历史审读见 `doc/core/reviews/round-004-reviewed-files.md` 与 `doc/core/reviews/round-008-reviewed-files.md`。 |
| `gameplay-release-gap-closure-2026-02-21` | 正文已退役；历史内容从 Git history、core review logs 与 GitHub task issue evidence comments 追溯。 |
| `gameplay-release-production-closure`、`gameplay-runtime-governance-closure` | 当前仍作为非首读 provenance 保留；当前判断不得以这两份旧 closure 替代 gameplay 主入口、现行专业域验证或 GitHub task evidence。 |

上述 closure 均不作为 active gameplay truth；已完成语义收敛的正文直接退役，不保留 redirect 或占位文件，尚未完成逐文件迁移审计的旧 closure 仅保留为非首读 provenance。

## 运行 / 证据 / 交接补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md` | `runbook` | restricted grant 发放、撤销、过期与 incident runbook |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-release-gate-2026-03-06.md` | `evidence` | distributed consensus longrun release gate 采证 |
| `doc/game/gameplay/gameplay-longrun-p0-replay-rollback-runbook-2026-03-06.md` | `runbook` | longrun P0 replay rollback 处理 runbook |
| `doc/game/gameplay/gameplay-micro-loop-readable-world-checklist-2026-03-10.md` | `checklist` | micro-loop readable world 验收清单 |
| `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md` | `evidence` | micro-loop visual closure 采证 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- ROUND-002 口径：`doc/game/gameplay/gameplay-top-level-design.prd.md` 为 gameplay 主文档，其余 gameplay 专题文档仅维护增量。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法完成分流时，才进入下一轮路径级治理。
