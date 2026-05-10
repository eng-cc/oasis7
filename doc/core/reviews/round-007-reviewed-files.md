# ROUND-007 逐文档内容职责边界复核清单

审计轮次: 7

## 清单状态
- 当前目标范围文档数: 874
- 当前已完成复核文档数: 874
- 当前状态: `completed`

## 字段说明
| 字段 | 说明 |
| --- | --- |
| 文档路径 | 复核文档的仓库相对路径 |
| 当前类型 | 当前文档类型：`prd` / `design` / `project` / `manual` / `runbook` / `readme` / `index` / `legacy_misc` |
| 边界判定 | `pass` / `mixed_prd_design` / `authority_drift` / `manual_overreach` / `待判定` |
| 主要问题编号 | 主要对应 `I7-*`；无问题写 `none` |
| 整改动作 | `keep` / `trim` / `retarget` / `split` / `backfill_links` |
| 索引回写 | `pending` / `done` / `n/a` |
| 引用回写 | `pending` / `done` / `n/a` |
| owner role | 默认牵头角色 |
| 状态 | `pending` / `in_progress` / `done` / `blocked` |
| 备注 | 补充说明 |

## 汇总
| 范围 | 文档数 | 状态 |
| --- | --- | --- |
| ROUND-007 总范围（`doc/**/*.md` - `doc/devlog/**`） | 874 | completed |
| 合计 | 874 | completed |

## 明细
| 文档路径 | 当前类型 | 边界判定 | 主要问题编号 | 整改动作 | 索引回写 | 引用回写 | owner role | 状态 | 备注 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `doc/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/core/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/core/checklists/cross-module-impact-checklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/core/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/core/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/core/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/core/reviews/consistency-review-round-001.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-002.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-003.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-004.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-005.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-006.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/consistency-review-round-007.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-001-archive-migration-plan.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-001-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-002-dedup-merge-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-002-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-003-filename-semantic-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-003-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-004-audit-progress-log.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-004-doc-design-quality-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-004-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-005-audit-progress-log.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-005-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-005-timeliness-index-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-006-audit-progress-log.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-006-kickoff-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-006-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-007-audit-progress-log.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-007-kickoff-worklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/reviews/round-007-reviewed-files.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/core/templates/prd-id-test-evidence-mapping.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/engineering/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/engineering/doc-governance/doc-structure-standard.design.md` | `design` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Design 结构/契约边界复核通过 |
| `doc/engineering/doc-governance/doc-structure-standard.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/doc-governance/doc-structure-standard.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/doc-governance/documentation-governance-engineering-closure-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/engineering/prd-review/checklists/active-core.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-engineering.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-game.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-headless-runtime.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-p2p.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-playability_test_result.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-readme.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-root-legacy.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-scripts.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-site.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-testing.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-world-runtime.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/checklists/active-world-simulator.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/engineering/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/engineering/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/engineering/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/game/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-base-runtime-wasm-layer-split.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-beta-balance-hardening-2026-02-22.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-release-gate-2026-03-06.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/game/gameplay/gameplay-engineering-architecture.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-layer-lifecycle-rules-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-layer-war-governance-crisis-meta-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-longrun-p0-replay-rollback-runbook-2026-03-06.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-module-driven-production-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-module-driven-production-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-release-gap-closure-2026-02-21.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-release-production-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-release-production-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-runtime-governance-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-runtime-governance-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-top-level-design.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/gameplay/gameplay-top-level-design.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game/gameplay/gameplay-war-politics-mvp-baseline.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/game/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/game/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/game-test.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/game-test.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/headless-runtime/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/headless-runtime/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-design-alignment-closure-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-design-alignment-review-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-longrun-traceable-memory-archive-hardening-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/headless-runtime/nonviewer/nonviewer-onchain-auth-protocol-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/headless-runtime/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/headless-runtime/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/headless-runtime/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase2.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase4.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase5.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase6.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/p2p-blockchain-security-hardening-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/consensus/builtin-wasm-identity-consensus.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/consensus/builtin-wasm-identity-consensus.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/p2p/distfs/distfs-builtin-wasm-api-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-builtin-wasm-api-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-builtin-wasm-storage.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-builtin-wasm-storage.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-node-runtime-integration-2026-03-01.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-open-ledger-2026-03-01.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-feedback-p2p-bridge-2026-03-01.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-path-index-observer-bootstrap.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-path-index-observer-bootstrap.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase1.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase1.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase2.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase2.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase4.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase4.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase5.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase5.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase6.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase6.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase7.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase7.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase8.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase8.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase9.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-production-hardening-phase9.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-runtime-path-index.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-runtime-path-index.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distfs/distfs-standard-file-io.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distfs/distfs-standard-file-io.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distributed/distributed-hard-split-phase7.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distributed/distributed-hard-split-phase7.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distributed/distributed-pos-consensus.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distributed/distributed-pos-consensus.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distributed/distributed-production-runtime-gap1234568-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/distributed/distributed-runtime.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/distributed/distributed-runtime.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/network/net-runtime-bridge-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/network/net-runtime-bridge-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/network/p2p-mobile-light-client-authoritative-state-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/network/readme-p1-network-production-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/network/readme-p1-network-production-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-builtin-wasm-fetch-fallback-compile.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-consensus-signer-binding-replication-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-consensus-signer-binding-replication-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-contribution-points-multi-node-closure-test.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-contribution-points-multi-node-closure-test.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-contribution-points-runtime-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-contribution-points-runtime-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-contribution-points.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-contribution-points.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-distfs-replication-network-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-distfs-replication-network-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-execution-reward-consensus-bridge.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-execution-reward-consensus-bridge.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-keypair-config-bootstrap.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-keypair-config-bootstrap.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-net-stack-unification-readme.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-net-stack-unification-readme.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-pos-time-anchor-control-plane-alignment-2026-03-07.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset-audit-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset-audit-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset-audit-hardening.release.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset-signature-governance-phase3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-redeemable-power-asset.release.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/p2p/node/node-replication-libp2p-migration.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-replication-libp2p-migration.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-reward-runtime-production-hardening-phase1.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-reward-runtime-production-hardening-phase1.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-reward-settlement-native-transaction.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-reward-settlement-native-transaction.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-storage-system-reward-pool.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-storage-system-reward-pool.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-uptime-base-reward.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-uptime-base-reward.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/node/node-wasm32-libp2p-compile-guard.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-metrics-runtime-bridge.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-observability.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-observability.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/observer/observer-sync-mode-runtime-metrics.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/observer/observer-sync-source-dht-mode.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/observer/observer-sync-source-dht-mode.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/observer/observer-sync-source-mode.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/observer/observer-sync-source-mode.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/p2p/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/token/mainchain-token-allocation-mechanism-phase2-governance-bridge-distribution-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/token/mainchain-token-allocation-mechanism.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/token/mainchain-token-allocation-mechanism.release.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/viewer-live/oasis7-viewer-live-llm-default-on-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/viewer-live/oasis7-viewer-live-no-llm-flag-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/p2p/viewer-live/oasis7-viewer-live-release-locked-launch-2026-02-23.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/playability_test_card.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_manual.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/playability_test_result/card_2026_02_28_19_22_20.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_02_28_21_22_51.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_02_28_22_47_14.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_02_28_23_27_06.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_03_01_00_20_13.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_03_06_12_43_31.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/card_2026_03_06_18_40_48.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/playability_test_result/game-test.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/playability_test_result/game-test.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/playability_test_result/playability_test_card.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/playability_test_manual.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/playability_test_result/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/playability_test_result/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/playability_test_result/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/readme/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap-infra-exec-compiler-sandbox.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap-wasm-live-persistence-instance-upgrade.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap12-consensus-market-lifecycle-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap12-market-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap12-market-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap123-runtime-consensus-metering.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap123-runtime-consensus-metering.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap2-llm-wasm-lifecycle.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap3-install-target-infrastructure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap3-install-target-infrastructure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/gap/readme-gap34-lifecycle-orderbook-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/governance/readme-resource-model-layering.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/governance/readme-resource-model-layering.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/governance/readme-world-rules-consolidation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/governance/readme-world-rules-consolidation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/readme/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/production/readme-llm-p1p2-production-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/production/readme-llm-p1p2-production-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/production/readme-p0-p1-closure.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/production/readme-p0-p1-closure.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/production/readme-prod-closure-llm-distfs-consensus.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/production/readme-prod-closure-llm-distfs-consensus.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/readme/production/readme-prod-gap1245-wasm-repl-topology-player.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/readme/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/scripts/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/scripts/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/scripts/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/scripts/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/scripts/precommit/pre-commit.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/scripts/precommit/pre-commit.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/scripts/precommit/precommit-remediation-playbook.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/scripts/precommit/precommit-remediation-playbook.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/scripts/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: capture-viewer-frame.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: capture-viewer-frame.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-art-capture-2026-02-28.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-art-capture-2026-02-28.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-02-28.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-02-28.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-03-01.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-framework-rationalization-2026-03-01.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-material-recognizability-2026-02-28.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-material-recognizability-2026-02-28.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-visual-detail-system-optimization-2026-02-28.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed viewer-tools doc set: viewer-texture-inspector-visual-detail-system-optimization-2026-02-28.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/scripts/wasm/builtin-wasm-nightly-build-std.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/scripts/wasm/builtin-wasm-nightly-build-std.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/site/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/site/github-pages/github-pages-architecture-svg-refresh.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-architecture-svg-refresh.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-benchmark-polish-v3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-benchmark-polish-v3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-content-sync-2026-02-12.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-content-sync-2026-02-12.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-content-sync-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-content-sync-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-game-first-home-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-game-first-home-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-hero-cta-simplify-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-hero-motion-layer.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-hero-motion-layer.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-hero-pointer-interaction.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-hero-pointer-interaction.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-home-conversion-i18n-screenshot-refresh-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-home-radical-redesign-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-lean-tech-refresh.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-lean-tech-refresh.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-quality-gates-sync-seo-hardening-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-showcase.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-showcase.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-user-perspective-adjustments-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/github-pages/github-pages-visual-polish-v2-2026-02-12.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/manual/site-manual-static-docs.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/manual/site-manual-static-docs.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/manual/viewer-manual-content-migration-2026-02-15.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/manual/viewer-manual-content-migration-2026-02-15.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/site/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/site/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/site/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-m1.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-builtin-wasm-docker-canonical-gate.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-builtin-wasm-determinism-gate-required-check-protection.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-remove-builtin-wasm-hash-checks-from-base-gate.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-test-coverage.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-test-coverage.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-testcase-tiering.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-testcase-tiering.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-tiered-execution.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-tiered-execution.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/ci/ci-wasm32-target-install.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/ci/ci-wasm32-target-install.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/testing/governance/llm-skip-tick-ratio-metric.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/governance/llm-skip-tick-ratio-metric.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/governance/release-gate-metric-policy-alignment-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/governance/wasm-build-determinism-guard.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/governance/wasm-build-determinism-guard.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/launcher/launcher-chain-script-migration-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/launcher/launcher-full-usability-closure-audit-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/launcher/launcher-lifecycle-hardening-2026-03-01.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/launcher/launcher-viewer-auth-node-config-autowire-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/chain-runtime-feedback-replication-network-autowire-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/chain-runtime-soak-script-reactivation-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/p2p-longrun-continuous-chaos-injection-2026-02-24.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/p2p-longrun-endurance-chaos-template-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/p2p-longrun-feedback-event-injection-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/p2p-storage-consensus-longrun-online-stability-2026-02-24.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/s10-distfs-probe-bootstrap-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/longrun/s10-five-node-real-game-soak.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/longrun/s10-five-node-real-game-soak.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/manual/systematic-application-testing-manual.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/manual/systematic-application-testing-manual.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/manual/web-ui-agent-browser-closure-manual.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/performance/runtime-performance-observability-foundation-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/performance/runtime-performance-observability-llm-api-decoupling-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/performance/viewer-perf-bottleneck-observability-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/performance/viewer-performance-methodology-closure-2026-02-25.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/testing/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/testing/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/testing/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/ui_review_result/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/ui_review_result/card_2026_03_06_11_50_29.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/ui_review_result/ui_review_list.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/viewer-manual.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/world-runtime/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/world-runtime/governance/audit-export.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/governance/governance-events.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/governance/zero-trust-governance-receipt-hardening-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/integration/node-contribution-points-runtime-closure.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/module/agent-default-modules.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/module/agent-default-modules.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/module/module-lifecycle.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/module/module-storage.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/module/module-storage.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/module/module-subscription-filters.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/module/module-subscription-filters.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/module/online-module-release-legality-closure-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/module/player-published-entities-2026-03-05.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/module/player-published-entities-2026-03-05.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/world-runtime/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/bootstrap-power-modules.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/bootstrap-power-modules.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-infinite-sequence-rollover.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-integration.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase1.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase10.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase11.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase12.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase13.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase14.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase15.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase2.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase4.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase5.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase6.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase7.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase8.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-numeric-correctness-phase9.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.design.md` | `design` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Design 结构/契约边界复核通过 |
| `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/runtime/runtime-storage-footprint-governance-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/testing/testing.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/wasm/wasm-agent-os-alignment-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/wasm/wasm-executor.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/wasm/wasm-executor.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/wasm/wasm-interface.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-runtime/wasm/wasm-sandbox-security-hardening.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/wasm/wasm-sandbox-security-hardening.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/wasm/wasm-sdk-no-std.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/wasm/wasm-sdk-no-std.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime/wasm/wasm-sdk-wire-types-dedup.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-runtime.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-runtime.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/README.md` | `readme` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/world-simulator/design.md` | `design` | `mixed_prd_design` | `I7-002` | `trim` | `n/a` | `n/a` | `producer_system_designer` | `done` | 已将旧模板段落标题收敛为设计型段落名，并完成复核 |
| `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/intent-distributed-runtime-closure-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-hook-foundation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-hook-foundation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-executor-foundation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-module-governance.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-readiness.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-readiness.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/kernel-rule-wasm-sandbox-bridge.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/resource-kind-compound-hardware-hard-migration.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/runtime-required-failing-tests-offline-2026-03-09.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/rust-wasm-build-suite.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/rust-wasm-build-suite.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-availability-ux-hardening-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-panel-2026-03-07.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p0-2026-03-07.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-public-chain-p1-address-contract-assets-mempool-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-blockchain-explorer-ui-ux-optimization-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-decouple-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-chain-runtime-execution-world-dir-output-hardening-2026-03-09.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-egui-web-unification-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-distributed-submit-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-entry-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-feedback-window-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-full-usability-remediation-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-graceful-stop-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-i18n-required-config-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-llm-settings-panel-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-native-legacy-cleanup-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-native-web-control-plane-unification-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-self-guided-experience-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-transfer-product-grade-parity-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-ui-schema-share-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-console-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-console-gui-agent-interface-2026-03-08.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-required-config-gating-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-settings-feedback-parity-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-transfer-closure-2026-03-06.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/launcher/game-client-launcher-web-wasm-time-compat-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/indirect-control-tick-lifecycle-long-term-memory.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-agent-behavior.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-agent-behavior.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-async-openai-responses.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-async-openai-responses.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-chat-user-message-tool-visualization.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-config-toml-style-unification-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-dialogue-chat-loop.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-dialogue-chat-loop.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-factory-strategy-optimization.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-factory-strategy-optimization.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-industrial-mining-debug-tools.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-industrial-mining-debug-tools.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-lmso29-stability.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-lmso29-stability.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-multi-scenario-evaluation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-multi-scenario-evaluation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-effect-receipt.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-effect-receipt.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-multi-step-orchestration.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-system.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/llm/llm-prompt-system.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-builtin-wasm-maintainability-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-industrial-benchmark-current-state-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-industrial-economy-wasm.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-industrial-economy-wasm.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-market-hardware-data-governance-closure-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-power-system.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-power-system.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/m4/material-multi-ledger-logistics.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/m4/material-multi-ledger-logistics.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/prd/acceptance/unified-checklist.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `historical removed standard_3d viewer doc set: visual-review-score-card` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/prd/acceptance/web-llm-evidence-template.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/prd/launcher/blockchain-transfer.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/prd/quality/experience-trend-tracking.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/prd.index.md` | `index` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 导航/索引文档，未发现越权承载需求/设计/项目权威内容 |
| `doc/world-simulator/prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/agent-frag-initial-spawn-position.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/agent-frag-initial-spawn-position.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/asteroid-fragment-renaming.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/asteroid-fragment-renaming.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/chunked-fragment-generation.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/chunked-fragment-generation.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/frag-resource-balance-onboarding.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/frag-resource-balance-onboarding.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/fragment-spacing.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/fragment-spacing.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/scenario-asteroid-fragment-overrides.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/scenario-files.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/scenario-files.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/scenario-power-facility-baseline.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/scenario-power-facility-baseline.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/scenario-seed-locations.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/scenario-seed-locations.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/scenario/world-initialization.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/scenario/world-initialization.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-2d-3d-clarity-improvement.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-2d-3d-clarity-improvement.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-2d-visual-polish.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-2d-visual-polish.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-3d-commercial-polish.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-3d-commercial-polish.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-3d-polish-performance.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-3d-polish-performance.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-agent-module-rendering.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-agent-module-rendering.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-agent-quick-locate.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-agent-quick-locate.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-agent-size-inspection.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-agent-size-inspection.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-asset-pipeline-ui-system-hardening-2026-03-05.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-asset-pipeline-ui-system-hardening-2026-03-05.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-auto-focus-capture.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-auto-focus-capture.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-auto-select-capture.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-auto-select-capture.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-bevy-web-runtime.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-bevy-web-runtime.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-agent-prompt-default-values-prefill.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-dedicated-right-panel.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-enter-send.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-enter-send.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-ime-cn-input.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-ime-cn-input.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-ime-egui-bridge.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-profile-editing.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets-scroll.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-prompt-presets.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-right-panel-polish.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-right-panel-polish.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-chat-web-deadlock-resolution.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase1-asset-pipeline.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase1-asset-pipeline.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase2-visual-quality-gate.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase2-visual-quality-gate.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase3-material-style-layer.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase3-material-style-layer.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase4-texture-style-layer.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase4-texture-style-layer.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase5-advanced-texture-maps.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase5-advanced-texture-maps.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase6-material-variant-preview.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase6-material-variant-preview.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase7-theme-pack-batch-preview.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase7-theme-pack-batch-preview.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-commercial-release-phase8-runtime-theme-hot-reload-and-asset-v2.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-advanced-debug-folding.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-plane-split-live-playback-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-copyable-text.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-copyable-text.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-dual-view-2d-3d.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-dual-view-2d-3d.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-egui-right-panel.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-egui-right-panel.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-first-session-goal-clarity-hardening-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-frag-default-rendering.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-frag-default-rendering.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-frag-scale-selection-stability.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-fragment-element-rendering.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-fragment-element-rendering.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-experience-overhaul.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase2.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase3.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase4.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase5.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase6.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-gameplay-release-immersion-phase7.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-generic-focus-targets.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-generic-focus-targets.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-i18n.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-i18n.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-industrial-visual-closure.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-industrial-visual-closure.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-industry-graph-layered-symbolic-zoom-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-disable-seek-p2p-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-full-event-driven-phase10-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-llm-event-driven-trigger-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-logical-time-interface-phase11-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-llm-full-bridge-2026-03-05.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase1-2026-03-04.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase2-2026-03-05.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-runtime-world-migration-phase3-2026-03-05.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-step-control-progress-stability-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-live-tick-driven-doc-archive-2026-02-27.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-location-depletion-visualization.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-location-depletion-visualization.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-location-fine-grained-rendering.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-location-fine-grained-rendering.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-manual.md` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/viewer/viewer-minimal-system.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-minimal-system.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-module-visual-entities.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-module-visual-entities.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-node-hard-decouple-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-observability-visual-optimization.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-observability-visual-optimization.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-open-world-sandbox-readiness.stress-report.template` | `legacy_misc` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | 补充文档/台账类，未发现本轮关注的职责边界高信号问题 |
| `doc/world-simulator/viewer/viewer-overview-map-zoom.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-overview-map-zoom.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-player-ui-declutter-2026-02-24.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-release-full-coverage-gate.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-release-full-coverage-gate.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-release-qa-iteration-loop.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-release-qa-iteration-loop.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-rendering-physical-accuracy.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-rendering-physical-accuracy.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-right-panel-module-visibility.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-right-panel-module-visibility.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-selection-details.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-selection-details.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-step-completion-ack-2026-02-28.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-texture-inspector.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-texture-inspector.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visual-release-readiness-hardening-2026-03-01.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visual-release-readiness-hardening-2026-03-01.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visual-upgrade.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visual-upgrade.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visualization-3d.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visualization-3d.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visualization.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-visualization.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-wasd-camera-navigation.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-wasd-camera-navigation.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-build-pruning-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-build-pruning-phase2-2026-03-02.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-web-closure-testing-policy.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-web-closure-testing-policy.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-fullscreen-panel-toggle.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-playability-unblock-2026-02-26.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-semantic-test-api.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-semantic-test-api.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-test-api-step-control-2026-02-24.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-web-usability-hardening-2026-02-22.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-webgl-deferred-compat-2026-02-24.prd` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `historical removed standard_3d viewer doc set: viewer-webgl-deferred-compat-2026-02-24.project` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator/viewer/viewer-websocket-http-bridge.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator/viewer/viewer-websocket-http-bridge.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
| `doc/world-simulator.prd.md` | `prd` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | PRD 目标态边界复核通过 |
| `doc/world-simulator.project.md` | `project` | `pass` | `none` | `keep` | `n/a` | `n/a` | `producer_system_designer` | `done` | Project 执行闭环边界复核通过 |
