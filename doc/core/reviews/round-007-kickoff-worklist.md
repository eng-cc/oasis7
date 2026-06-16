# ROUND-007 内容职责边界复核执行清单

审计轮次: 7

## 目标
- 将 ROUND-007 定义为“内容职责边界复核轮”，重点检查文档内容是否真正落在正确职责载体中。
- 形成“冻结范围 -> 逐批复核 -> 回写权威源/互链 -> 复审验收”的闭环。

## 执行阶段
| 阶段 | 动作 | 产物 | 状态 |
| --- | --- | --- | --- |
| P0 | 建立 ROUND-007 执行台账骨架 | `consistency-review-round-007.md`、`round-007-reviewed-files.md`、`round-007-kickoff-worklist.md`、`round-007-audit-progress-log.md` | done |
| P1 | 冻结 ROUND-007 范围与问题域，明确 owner 与批次 | 总范围、`G7-*`、`I7-*` | done |
| P2 | 生成逐文档复核清单；当前保留 compact snapshot entrypoint，逐行证据由 pre-compaction git snapshot 恢复 | `round-007-reviewed-files.md` | done |
| P3 | 按批次执行内容职责边界复核与回写 | 文档回写提交 + 互链/索引同步 | done |
| P4 | 运行复审与验收门禁 | 复审结论 + 验收记录 | done |

## 并行批次（完成态）
| 批次 | 范围 | 目标问题 | owner role | 状态 |
| --- | --- | --- | --- | --- |
| B7-001 | 模块入口 `prd/design/project` | D7-001/D7-002/D7-003/D7-006 | `producer_system_designer` | done |
| B7-002 | 活跃专题三件套 | D7-001/D7-002/D7-003/D7-005 | 对应模块 owner | done |
| B7-003 | `manual` / `runbook` 与权威源边界 | D7-004/D7-005 | 对应模块 owner | done |
| B7-004 | 复审与阻断结论 | 全量复核 | `qa_engineer` | done |

## 执行原则
- 不重复 ROUND-006：结构、命名、入口、legacy 收口已视为上轮完成项；本轮不再以结构迁移为主要目标。
- 以内容职责为准：优先判断 Why/What/Done、How/Structure/Contract、How/When/Who 是否落在正确文档中。
- 一处调整，多处回写：若内容迁移影响 `README`、`prd.index.md`、模块 `project.md` 或专题互链，必须同步回写。
- 先定权威源：同一事项若多文档重复，必须先确定唯一权威源，再执行裁剪或引用替换。
- 未完成不升轮次：文档未完成本轮内容边界整改前，不得回写 `审计轮次: 7`。
- 逐文档证据恢复：当前入口为 compact snapshot，完整逐行清单使用 `git show 0d6fd50849cae07bac17883cca14f141ede93196:doc/core/reviews/round-007-reviewed-files.md` 恢复。

## 验收口径
- required：ROUND-007 台账、清单、批次、问题池与字段定义齐全。
- full：本轮范围内内容职责混写问题关闭、延期项已登记、复审结论已落档。

## 通用验收命令
- `test -f doc/core/reviews/consistency-review-round-007.md`
- `test -f doc/core/reviews/round-007-reviewed-files.md`
- `test -f doc/core/reviews/round-007-kickoff-worklist.md`
- `test -f doc/core/reviews/round-007-audit-progress-log.md`
- `rg -n "内容职责边界复核轮|D7-|I7-|B7-|compact snapshot" doc/core/reviews/consistency-review-round-007.md doc/core/reviews/round-007-kickoff-worklist.md doc/core/reviews/round-007-reviewed-files.md`
- `./scripts/doc-evidence-snapshot-check.sh`
