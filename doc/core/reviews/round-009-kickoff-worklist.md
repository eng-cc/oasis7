# ROUND-009 文档消费入口与手册语义执行清单

审计轮次: 9

## 目标
- 将 ROUND-009 定义为“小分母消费层治理轮”，重点处理入口可消费性、手册语义载体和目录边界热点。
- 形成“冻结 focused scope -> 逐对象给出 keep/migrate/split/defer -> 先做高优先级回写 -> 复审收口”的闭环。

## 执行阶段
| 阶段 | 动作 | 产物 | 状态 |
| --- | --- | --- | --- |
| P0 | 建立 ROUND-009 执行台账骨架 | `consistency-review-round-009.md`、`round-009-reviewed-files.md`、`round-009-kickoff-worklist.md`、`round-009-audit-progress-log.md` | done |
| P1 | 冻结 focused scope、问题池与批次 | `ROUND-009` 台账内的总范围、`G9-*`、`I9-*` | done |
| P2 | 完成首轮 keep/migrate/split/defer 判定 | `round-009-reviewed-files.md` | done |
| P3 | 执行高优先级入口/手册对象的首批回写 | 文档回写、互链修复、静态镜像同步建议 | pending |
| P4 | 运行复审与验收门禁 | `doc-governance-check` + focused scope 抽样复核 | pending |

## 并行批次
| 批次 | 范围 | 目标问题 | owner role | 状态 |
| --- | --- | --- | --- | --- |
| B9-001 | 根入口 + docs hub 入口 | D9-001/D9-005 | `producer_system_designer` | in_progress |
| B9-002 | 模块 README / 高体量索引热点 | D9-001/D9-002/D9-004 | `producer_system_designer` | in_progress |
| B9-003 | 高频手册与测试分册 | D9-003/D9-005 | `qa_engineer` / `viewer_engineer` | in_progress |
| B9-004 | `readme` / `ui_review_result` 边界热点 | D9-004/D9-006 | `liveops_community` / `viewer_engineer` | in_progress |
| B9-005 | 复审与阻断结论 | 全量 focused scope | `qa_engineer` | pending |

## 执行原则
- 不回退到全仓逐篇重读：本轮以 focused scope 为正式分母，避免再做一次过宽治理。
- 先定消费角色，再定文档类型：优先回答“谁来读、用来做什么”，再判断应落到 `PRD/Design/Project/Manual/Runbook/素材说明` 中哪一类。
- 先给动作结论，再决定是否改名迁移：允许某些文档先被判定为 `migrate`，但不要求同一天完成实体改名。
- 手册优先看使用路径，不先看命名：若正文是稳定操作步骤，即使当前位于 `*.prd.md`，也按手册问题处理。
- 素材包不冒充权威源：运营/社区素材可以保留，但必须与 canonical 规范入口分层。

## 验收口径
- required：ROUND-009 台账、清单、批次、问题池和 focused scope 已冻结，且每个对象已有首轮动作建议。
- full：P0/P1 对象已形成明确迁移或保留决议，互链与静态镜像影响已登记，复审结论已落档。

## 通用验收命令
- `test -f doc/core/reviews/consistency-review-round-009.md`
- `test -f doc/core/reviews/round-009-reviewed-files.md`
- `test -f doc/core/reviews/round-009-kickoff-worklist.md`
- `test -f doc/core/reviews/round-009-audit-progress-log.md`
- `rg -n "ROUND-009|I9-|B9-|keep|migrate|split|defer" doc/core/reviews/consistency-review-round-009.md doc/core/reviews/round-009-reviewed-files.md doc/core/reviews/round-009-kickoff-worklist.md`
