# core PRD 文件级索引

审计轮次: 7

更新时间：2026-04-10

## 入口
- 模块 PRD：`doc/core/prd.md`
- 模块设计总览：`doc/core/design.md`
- 模块标准执行入口：`doc/core/project.md`
- 当前高频 cross-module 契约入口：`doc/core/player-access-mode-contract-2026-03-19.prd.md`

## 首读分流
- 想先回答 core 模块在管什么、哪些口径是全局唯一真值：先读 `doc/core/prd.md`
- 想先回答当前在推进什么、最近完成了什么、下一步是什么：先读 `doc/core/project.md`
- 想直接进入下一轮跨模块优先级主入口：先读 `doc/core/next-round-priority-slate-2026-03-11.prd.md`
- 想直接进入玩家访问模式 / execution lane 的正式契约：先读 `doc/core/player-access-mode-contract-2026-03-19.prd.md`
- 想直接进入版本候选 readiness / go-no-go：先读 `doc/core/release-candidate-readiness-entry-2026-03-11.prd.md` 与 `doc/core/release-candidate-go-no-go-entry-2026-03-11.prd.md`
- 想继续按子域或文件名下钻：使用下方热点子域导航，再跳到对应清单区域

## 密度快照（2026-04-10）
- `doc/core/`：81 份文件
- `doc/core/reviews/`：45 份文件
- 根目录活跃专题与 handoff：28 份文件
- `doc/core/templates/`：2 份文件
- `doc/core/checklists/`：1 份文件

## 热点子域导航
| 子域 | 文件数 | 适合回答的问题 |
| --- | --- | --- |
| `reviews/` | 45 | ROUND 审查、候选级 readiness/go-no-go、audit progress 与历史评审留痕；默认按需进入 |
| 根目录活跃专题 | 28 | 项目级总览、下一轮优先级、release candidate readiness/version/go-no-go、docs hub 同步、player access mode contract |
| `templates/` | 2 | 阶段收口与 PRD-ID 追踪模板 |
| `checklists/` | 1 | 跨模块影响检查清单 |

## 活跃补充文档
- `doc/core/next-round-priority-slate-2026-03-11.prd.md`：下一轮跨模块优先级主入口，适合快速判断“接下来只做什么”。
- `doc/core/player-access-mode-contract-2026-03-19.prd.md`：`software_safe / pure_api` 与 execution lane 的正式 cross-module 契约。
- `doc/core/release-candidate-readiness-entry-2026-03-11.prd.md`：版本候选 readiness 正式入口。
- `doc/core/release-candidate-go-no-go-entry-2026-03-11.prd.md`：版本候选 go/no-go 正式入口。
- `doc/core/doc-readme-public-entry-sync-2026-03-11.prd.md`：仓库 docs hub 与公共阅读路径同步入口。

## 默认阅读面边界
- 本页首屏只负责分流，不再要求读者先顺扫全部活跃专题和 review 文件。
- `reviews/`、模板、checklist 与 handoff 文档继续保留可检索性，但默认不和模块 PRD 三件套同屏平铺。
- 完整专题清单继续保留在下方，用于精确文件名检索和互链可达性。

## 覆盖规则
- 纳入规则：纳入 `doc/core/*.prd.md` 与同名 `*.design.md` / `*.project.md` 的活跃专题三件套。
- 活跃补充：仍被当前模块 PRD / 项目态直接引用的 cross-module 入口，可在“活跃补充文档”区定向列出，但不并入 review / template / checklist 清单。
- 排除规则：不纳入 `doc/core/reviews/**`、`doc/core/templates/**`、`doc/core/checklists/**` 与 `doc/devlog/**` 的非三件套材料。
- 按需进入：ROUND 审查、go/no-go 留痕、采证板、模板与 checklist 继续保留可检索性；除非重新成为当前 operator 或 owner 的直接入口，否则不进入默认首屏。

## 完整活跃专题清单（按文件名精确检索）
| 专题 PRD | 专题设计文档 | 专题项目文档 |
| --- | --- | --- |
| `doc/core/next-round-priority-slate-2026-03-11.prd.md` | `doc/core/next-round-priority-slate-2026-03-11.design.md` | `doc/core/next-round-priority-slate-2026-03-11.project.md` |
| `doc/core/release-candidate-readiness-entry-2026-03-11.prd.md` | `doc/core/release-candidate-readiness-entry-2026-03-11.design.md` | `doc/core/release-candidate-readiness-entry-2026-03-11.project.md` |
| `doc/core/release-candidate-version-escalation-2026-03-11.prd.md` | `doc/core/release-candidate-version-escalation-2026-03-11.design.md` | `doc/core/release-candidate-version-escalation-2026-03-11.project.md` |
| `doc/core/release-candidate-go-no-go-entry-2026-03-11.prd.md` | `doc/core/release-candidate-go-no-go-entry-2026-03-11.design.md` | `doc/core/release-candidate-go-no-go-entry-2026-03-11.project.md` |
| `doc/core/doc-readme-public-entry-sync-2026-03-11.prd.md` | `doc/core/doc-readme-public-entry-sync-2026-03-11.design.md` | `doc/core/doc-readme-public-entry-sync-2026-03-11.project.md` |
| `doc/core/player-access-mode-contract-2026-03-19.prd.md` | `doc/core/player-access-mode-contract-2026-03-19.design.md` | `doc/core/player-access-mode-contract-2026-03-19.project.md` |

## 审计 / 模板 / 清单补充入口
| 文档路径 | 类型 | 用途 |
| --- | --- | --- |
| `doc/core/reviews/consistency-review-round-009.md` | `audit` | ROUND-009 文档消费入口与手册语义收口轮记录 |
| `doc/core/reviews/consistency-review-round-010.md` | `audit` | ROUND-010 继续复审记录 |
| `doc/core/reviews/release-candidate-go-no-go-version-2026-03-11.md` | `audit` | 版本候选 go/no-go 留痕 |
| `doc/core/reviews/release-candidate-readiness-board-version-2026-03-11.md` | `audit` | 版本候选 readiness board |
| `doc/core/templates/stage-closure-go-no-go-template.md` | `template` | 阶段收口 go/no-go 模板 |
| `doc/core/templates/prd-id-test-evidence-mapping.md` | `template` | PRD-ID 到测试证据映射模板 |
| `doc/core/checklists/cross-module-impact-checklist.md` | `checklist` | 跨模块影响检查清单 |

## 说明
- 本索引用于保证模块专题文档在根入口文档树中可达。
- 文档配对规则：`*.prd.md`、`*.design.md` 与同名 `*.project.md`。
- readiness / version / go-no-go / docs-hub / player-access-mode-contract 同步专题均属于本轮 `core` 活跃执行链。
- 默认入口面先在 `README.md` / `prd.index.md` 收紧；只有当入口仍无法分流时，才进入后续路径级治理。
