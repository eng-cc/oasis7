# Core 延期模块入口分流治理台账（第010轮）

审计轮次: 10

## 目的
- 为 `TASK-CORE-041` / `TASK-ENGINEERING-066` 提供 ROUND-010 的统一执行台账，将本轮目标定义为“ROUND-009 延期模块入口分流跟进轮”。
- 本轮只处理上一轮明确延期、但仍可能受益于更清晰读者入口的模块 `README.md`，不重新打开已在 ROUND-009 收口的 manual、例外目录与静态镜像问题。
- 所有 ROUND-010 结论仍以 `doc/engineering/doc-structure-standard.prd.md` 与 `doc/engineering/doc-structure-standard.design.md` 为裁定依据；若发现规则冲突，先补标准，再继续治理。

## 权威依据
- 结构规范需求：`doc/engineering/doc-structure-standard.prd.md`
- 结构规范设计：`doc/engineering/doc-structure-standard.design.md`
- 结构规范执行：`doc/engineering/doc-structure-standard.project.md`
- 上一轮结论：`doc/core/reviews/consistency-review-round-009.md`
- 上一轮清单：`doc/core/reviews/round-009-reviewed-files.md`
- 工程总入口：`doc/README.md`
- 项目级 ROUND 台账：`doc/core/project.md`
- 工程主台账：`doc/engineering/project.md`

## 轮次信息
- 轮次编号: `ROUND-010`
- 轮次状态: `in_progress` (`not_started` | `in_progress` | `completed`)
- 轮次类型: `deferred_module_entry_routing`
- 审查/治理时间窗: `2026-03-30`
- owner role: `producer_system_designer`
- 协作角色: `qa_engineer`（证据/测试模块入口复核）
- 当前阶段说明: 已从 ROUND-009 的 `deferred` 项中抽取 6 个模块 README，建立新的小分母 focused scope，按“高体量 runtime -> 工具/网络 -> 玩法/证据”顺序继续治理。

状态判定：
- `not_started`: 仅记录轮次意图，尚未冻结范围与问题域。
- `in_progress`: 已冻结 focused scope、问题池、批次与清单，正在逐对象回写。
- `completed`: focused scope 内对象均已形成 `aligned/deferred` 终态，并完成 QA 复核。

## 文档级审计/治理标记方法（缺省=0）
- ROUND-010 继续采用“小分母 focused scope”模式，不要求对 `doc/**/*.md` 全量回写 `审计轮次: 10`。
- 仅当文档被纳入 ROUND-010 focused scope 且已完成本轮结论回写时，才允许回写 `审计轮次: 10`。
- 若对象沿用 ROUND-009 的延期结论且本轮决定继续不动，应明确标记为 `deferred`，而不是继续停留在 `scoped`。

## 治理维度（以“入口可消费性”为主）
| 编号 | 维度 | 治理目标 | 严重度判定 |
| --- | --- | --- | --- |
| D10-001 | 模块入口起点 | 模块 README 能告诉读者先看 PRD、Project、索引还是操作手册 | 起点缺失=high |
| D10-002 | 角色/读者映射 | 不同模块能说明其主要读者是谁，避免把证据库、工具库伪装成通用新手入口 | 角色模糊=medium |
| D10-003 | 高体量索引分流 | 高体量模块不把长表索引误当成首读入口 | 索引失焦=high |
| D10-004 | 延期项可追溯性 | 本轮决定继续不动的模块必须明确后续触发条件 | 延期失控=medium |

## 总范围（ROUND-010 固定分母）
- focused scope 固定为 `6` 个来自 ROUND-009 的 deferred 模块 README。
- 范围文件记录于 `doc/core/reviews/round-010-reviewed-files.md`。
- 范围对象：
  - `doc/world-runtime/README.md`
  - `doc/game/README.md`
  - `doc/p2p/README.md`
  - `doc/scripts/README.md`
  - `doc/playability_test_result/README.md`
  - `doc/headless-runtime/README.md`

## 执行批次（已启动）
- G10-001: 高体量 runtime / infra 入口分流
- G10-002: 网络 / 工具模块入口抽样收口
- G10-003: 玩法 / 证据模块入口抽样收口
- G10-004: QA 复审与延期项冻结

## focused scope 清单（S_round010）
- 清单文件：`doc/core/reviews/round-010-reviewed-files.md`
- 当前固定分母：`6`
- 字段：`文档路径`、`当前角色`、`关注点`、`建议动作`、`优先级`、`owner role`、`当前状态`、`问题编号`、`备注`

## 进度日志
- 日志文件：`doc/core/reviews/round-010-audit-progress-log.md`
- 记录粒度：每完成一次范围冻结、对象回写、延期判定或复审结论即更新。

## 问题池
| 编号 | 来源 | 问题描述 | 影响范围 | 建议动作 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I10-001 | ROUND-009 deferred follow-up | `world-runtime`、`p2p` 一类高体量模块仍主要按主题目录列出，缺少“先读哪里”的任务导向入口 | `doc/world-runtime/README.md`、`doc/p2p/README.md` | 优先为高体量模块补“从这里开始”与索引/手册边界说明 | open |
| I10-002 | ROUND-009 deferred follow-up | `scripts`、`headless-runtime` 这类工具/基础设施模块可能仍偏工程内视角，未说明主要读者和使用前提 | `doc/scripts/README.md`、`doc/headless-runtime/README.md` | 明确读者角色、首读入口与非目标 | open |
| I10-003 | ROUND-009 deferred follow-up | `game`、`playability_test_result` 的入口分别偏产品/证据，但尚未确认是否需要进一步分流 | `doc/game/README.md`、`doc/playability_test_result/README.md` | 抽样后决定 `keep` 还是补小幅读者分流 | open |

## 执行项
| 编号 | 执行动作 | owner role | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A10-001 | 建立 ROUND-010 台账、focused scope 清单、kickoff worklist 与进度日志 | `producer_system_designer` | 2026-03-30 | `test -f doc/core/reviews/consistency-review-round-010.md && test -f doc/core/reviews/round-010-reviewed-files.md && test -f doc/core/reviews/round-010-kickoff-worklist.md && test -f doc/core/reviews/round-010-audit-progress-log.md` | done |
| A10-002 | 冻结 ROUND-010 focused scope、问题池与批次口径 | `producer_system_designer` | 2026-03-30 | `rg -n "ROUND-010|focused scope|I10-|G10-" doc/core/reviews/consistency-review-round-010.md` | done |
| A10-003 | 先完成 `world-runtime` 入口治理，验证 ROUND-010 的首片模式 | `producer_system_designer` | 待定 | `./scripts/doc-governance-check.sh` | pending |
| A10-004 | 完成剩余模块 README 的 `aligned/deferred` 决议 | 对应 owner | 待定 | `./scripts/doc-governance-check.sh` | pending |
| A10-005 | ROUND-010 复审与阻断结论 | `qa_engineer` | 待定 | `./scripts/doc-governance-check.sh` + focused scope 抽样复核 | pending |

## 复审结果
- 当前结论: `in_progress`
- 备注: ROUND-010 当前仅完成 deferred scope 冻结与问题池建档，接下来按 `world-runtime -> p2p/scripts -> game/playability/headless-runtime` 的顺序推进。
