# game PRD 文件级索引
审计轮次: 12

更新时间：2026-05-07

## 入口
- 模块 PRD：`doc/game/prd.md`
- 模块设计总览：`doc/game/design.md`
- 模块标准执行入口：`doc/game/project.md`
- 当前高频 gameplay 入口：`doc/game/gameplay/gameplay-top-level-design.prd.md`

## 首读分流
- 想先回答 game 模块当前目标态、阶段判断与完成定义：先读 `doc/game/prd.md`
- 想先回答当前还在推进什么、阻断在哪里、下一步做什么：先读 `doc/game/project.md`
- 想先理解核心玩法骨架，而不是逐篇翻 gameplay 长表：先读 `doc/game/gameplay/gameplay-top-level-design.prd.md`
- 想先看当前冲刺窗口与留存修复：先读 `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- 想先回答“1cm 物理世界”和“当前为什么不是 Minecraft 式逐块玩法”之间的边界：先读 `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
- 想先看试玩放行与 beta 边界：先读 `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md` 与 `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- 想继续按文件名、专题或补充材料下钻：使用下方密度快照、热点子域导航与补充入口

## 密度快照（2026-05-07）
- `doc/game/`：80 份文件
- `doc/game/gameplay/`：75 份文件
- `doc/game/gameplay/` 正式专题三件套：57 份文件
- `doc/game/gameplay/` 补充材料：18 份文件
- 模块根入口：5 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `gameplay/` 正式专题三件套 | 54 | 核心玩法骨架、留存修复、preview/beta gate、claim economy、治理、长稳与发布闭环 |
| `gameplay/` 补充材料 | 18 | runbook、evidence、checklist、handoff 与跨角色执行留痕 |
| 模块根入口 | 5 | 模块目标态、执行台账、设计总览与文件级精确检索 |

## 活跃补充文档
- `doc/game/gameplay/gameplay-top-level-design.prd.md`：核心玩法骨架主入口。
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`：当前冲刺窗口、跨角色优先级与 10 分钟留存修复主入口。
- `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`：物理尺度真值、间接控制动作粒度与表现层夸张边界主入口。
- `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`：试玩执行边界与继续/暂停决策主入口。
- `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`：closed beta 放行条件与候选级门禁主入口。
- `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`：近期高频经济规则与 token 成本边界主入口。
- `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`：restricted grant 发放、撤销、过期与 incident 处理 runbook。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫所有 gameplay 专题三件套与补充材料。
- runbook、evidence、checklist 与 handoff 继续保留可检索性，但默认不与主专题三件套同屏平铺成长名单。
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
| `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.prd.md` | `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.design.md` | `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md` |
| `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md` | `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.design.md` | `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md` |
| `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md` | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.design.md` | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md` |
| `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md` | `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.design.md` | `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md` |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md` | `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.design.md` | `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.project.md` |
| `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md` | `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.design.md` | `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.project.md` |
| `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md` | `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.design.md` | `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.project.md` |
| `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md` | `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.design.md` | `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.project.md` |
| `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md` | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.design.md` | `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md` |
| `doc/game/gameplay/gameplay-module-driven-production-closure.prd.md` | `doc/game/gameplay/gameplay-module-driven-production-closure.design.md` | `doc/game/gameplay/gameplay-module-driven-production-closure.project.md` |
| `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md` | `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.design.md` | `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.project.md` |
| `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md` | `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.design.md` | `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.project.md` |
| `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.prd.md` | `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.design.md` | `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.project.md` |
| `doc/game/gameplay/gameplay-release-production-closure.prd.md` | `doc/game/gameplay/gameplay-release-production-closure.design.md` | `doc/game/gameplay/gameplay-release-production-closure.project.md` |
| `doc/game/gameplay/gameplay-runtime-governance-closure.prd.md` | `doc/game/gameplay/gameplay-runtime-governance-closure.design.md` | `doc/game/gameplay/gameplay-runtime-governance-closure.project.md` |
| `doc/game/gameplay/gameplay-top-level-design.prd.md` | `doc/game/gameplay/gameplay-top-level-design.design.md` | `doc/game/gameplay/gameplay-top-level-design.project.md` |
| `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md` | `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.design.md` | `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md` |

## 运行 / 证据 / 交接补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md` | `runbook` | restricted grant 发放、撤销、过期与 incident runbook |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-release-gate-2026-03-06.md` | `evidence` | distributed consensus longrun release gate 采证 |
| `doc/game/gameplay/gameplay-longrun-p0-replay-rollback-runbook-2026-03-06.md` | `runbook` | longrun P0 replay rollback 处理 runbook |
| `doc/game/gameplay/gameplay-micro-loop-readable-world-checklist-2026-03-10.md` | `checklist` | micro-loop readable world 验收清单 |
| `doc/game/gameplay/gameplay-micro-loop-visual-closure-evidence-2026-03-10-round009.md` | `evidence` | micro-loop visual closure 采证 |
| `doc/game/gameplay/producer-to-liveops-task-game-036-limited-preview-execution-2026-03-22.md` | `handoff` | limited preview 执行交接 |
| `doc/game/gameplay/producer-to-qa-task-game-037-limited-preview-gate-watch-2026-03-22.md` | `handoff` | limited preview gate watch 交接 |
| `doc/game/gameplay/producer-to-runtime-task-game-029-closed-beta-runtime-evidence-2026-03-21.md` | `handoff` | closed beta runtime evidence 交接 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- ROUND-002 口径：`doc/game/gameplay/gameplay-top-level-design.prd.md` 为 gameplay 主文档，其余 gameplay 专题文档仅维护增量。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法完成分流时，才进入下一轮路径级治理。
