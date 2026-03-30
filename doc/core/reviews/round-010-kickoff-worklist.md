# ROUND-010 延期模块入口分流执行清单

审计轮次: 10

## 目标
- 将 ROUND-010 定义为“ROUND-009 延期模块 README follow-up 轮”，只处理延期模块入口分流，不重开已收口的手册与静态镜像议题。
- 形成“冻结 deferred scope -> 先做高体量模块 -> 复核工具/证据模块 -> QA 复审”的闭环。

## 执行阶段
| 阶段 | 动作 | 产物 | 状态 |
| --- | --- | --- | --- |
| P0 | 建立 ROUND-010 执行台账骨架 | `consistency-review-round-010.md`、`round-010-reviewed-files.md`、`round-010-kickoff-worklist.md`、`round-010-audit-progress-log.md` | done |
| P1 | 冻结 deferred focused scope、问题池与批次 | `ROUND-010` 台账内的总范围、`G10-*`、`I10-*` | done |
| P2 | 完成首片 `world-runtime` 入口治理 | 文档回写、互链修复、状态更新 | pending |
| P3 | 完成剩余模块 README 的 keep/split/defer 决议 | `round-010-reviewed-files.md` | pending |
| P4 | 运行 QA 复审与验收门禁 | `doc-governance-check` + focused scope 抽样复核 | pending |

## 并行批次
| 批次 | 范围 | 目标问题 | owner role | 状态 |
| --- | --- | --- | --- | --- |
| B10-001 | `world-runtime` | D10-001/D10-003 | `producer_system_designer` | pending |
| B10-002 | `p2p` + `scripts` | D10-001/D10-002 | `producer_system_designer` | pending |
| B10-003 | `game` + `playability_test_result` + `headless-runtime` | D10-002/D10-004 | `producer_system_designer` / `qa_engineer` | pending |
| B10-004 | ROUND-010 复审与关轮 | 全量 focused scope | `qa_engineer` | pending |

## 执行原则
- 先处理“读者起点”而非专题扩容：本轮优先回答“第一次进入该模块时应该先看什么”。
- 不为所有模块强行加矩阵：若抽样确认当前 README 已足够可消费，允许维持 `keep`。
- 证据库与工具库不冒充新手入口：`playability_test_result`、`scripts` 若本质是 QA/工程入口，应明确读者边界，而不是硬加通用 landing。
- 延期要有理由：若再次 defer，必须写清触发条件与后续轮次预期。

## 验收口径
- required：ROUND-010 台账、清单、批次和问题池已冻结，且 6 个对象均有首轮动作建议。
- full：至少完成 `world-runtime` 首片回写，并给出 6 个对象的 `aligned/deferred` 终态与 QA 复审结论。

## 通用验收命令
- `test -f doc/core/reviews/consistency-review-round-010.md`
- `test -f doc/core/reviews/round-010-reviewed-files.md`
- `test -f doc/core/reviews/round-010-kickoff-worklist.md`
- `test -f doc/core/reviews/round-010-audit-progress-log.md`
- `rg -n "ROUND-010|I10-|B10-|keep|split|defer" doc/core/reviews/consistency-review-round-010.md doc/core/reviews/round-010-reviewed-files.md doc/core/reviews/round-010-kickoff-worklist.md`
