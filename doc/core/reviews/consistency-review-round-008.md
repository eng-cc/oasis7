# Core 专题 Design 补齐治理台账（第008轮）

审计轮次: 8

## 目的
- 为 `TASK-CORE-010` / `TASK-ENGINEERING-035` 提供 ROUND-008 的统一执行台账，将本轮目标定义为“专题 `Design` 补齐轮”。
- 本轮聚焦已具备 `PRD + Project` 的专题中，哪些仍缺少 `Design`，并按复杂度进行“必须补 / 建议补 / 可暂缓”分级与回写。
- 所有 ROUND-008 结论均以 `doc/engineering/doc-governance/doc-structure-standard.prd.md` 与 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为裁定依据。

## 权威依据
- 结构规范需求：`doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 结构规范设计：`doc/engineering/doc-governance/doc-structure-standard.design.md`
- 结构规范执行：`doc/engineering/doc-governance/doc-structure-standard.project.md`
- 工程主台账：`doc/engineering/project.md`
- 项目级 ROUND 台账：`doc/core/project.md`
- 开发工作流：`AGENTS.md`

## 轮次信息
- 轮次编号: `ROUND-008`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 轮次类型: `topic_design_backfill`
- 审查/治理时间窗: `2026-03-10`
- owner role: `producer_system_designer`
- 协作角色: `qa_engineer`（验收/阻断）、各模块 owner（按专题补齐 `*.design.md`）
- 当前阶段说明: ROUND-008 已启动；已完成 `365` 个缺口专题的全量分级，并补齐 365 个高优先级专题 `Design`。

状态判定：
- `not_started`: 已完成模板与字段定义，但尚未开始全量扫描与分级。
- `in_progress`: 已固定缺口清单，正在分级与批次补齐 `*.design.md`。
- `completed`: 高优先级补齐任务关闭，延期项已登记，复审结论已落档。

## 文档级审计/治理标记方法（缺省=0）
- 每个受治理文档采用字段 `审计轮次: <整数>` 标识最新已完成治理轮次。
- ROUND-008 执行规则：
  - 专题被纳入 ROUND-008 后，需先判定复杂度，再决定 `must_backfill` / `should_backfill` / `defer_allowed`。
  - 若实际补齐 `*.design.md`，必须同步回写对应 `*.prd.md`、`*.project.md`、模块 `README.md` 与 `prd.index.md`。
  - 若专题暂缓补齐 `Design`，必须在台账中留下明确理由与触发补齐条件。
- 本轮完成条件：已完成缺口专题全量分级，高优先级专题补齐或纳入明确延期计划，并形成复审结论。

## 治理维度（以 Design 覆盖率为目标）
| 编号 | 维度 | 治理目标 | 严重度判定 |
| --- | --- | --- | --- |
| D8-001 | 缺口识别 | 全量识别“有 `PRD + Project` 但无 `Design`”专题 | 漏扫=high |
| D8-002 | 复杂度分级 | 将缺口专题分为 `must_backfill` / `should_backfill` / `defer_allowed` | 误判=high |
| D8-003 | 设计补齐 | 为高优先级专题补齐 `*.design.md` 并回写互链 | 缺失=high |
| D8-004 | 内容迁移 | 将 `PRD/Project` 中的设计性内容迁回 `Design` | 混写=medium |
| D8-005 | 延期治理 | 为暂缓专题登记理由、边界与触发条件 | 漏记=medium |

## 总范围（ROUND-008 固定分母）
- `doc/**/*.md`，排除 `doc/devlog/**`。
- ROUND-008 当前真实分母：`874`。
- 本轮重点对象：所有同时存在 `*.prd.md` 与 `*.project.md`、但缺少 `*.design.md` 的专题。
- 当前首轮缺口总数：`365`。

## 执行批次（已启动）
- G8-001: 全量缺口扫描与模块分布统计
- G8-002: 缺口专题复杂度分级（must/should/defer）
- G8-003: 高优先级专题补齐 `*.design.md`
- G8-004: 复审、延期项登记与门禁回写

## 缺口清单（S_round008）
- 清单文件：`doc/core/reviews/round-008-reviewed-files.md`
- 优先级文件：`doc/core/reviews/round-008-design-backfill-priority-list.md`
- 当前基线（2026-03-10，启动时冻结）：`365` 个缺 `Design` 专题

## 进度日志
- 日志文件：`doc/core/reviews/round-008-audit-progress-log.md`
- 记录粒度：专题级；每完成一个专题分级或补齐即回写。

## 问题池
| 编号 | 来源 | 问题描述 | 影响范围 | 建议动作 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I8-001 | `doc-structure-standard` | 专题已存在 `PRD + Project`，但缺少 `Design` | 首轮识别 `365` 个专题，已全部补齐 `365` 个 | 完成 ROUND-008 全量补齐并转入常规治理 | done |
| I8-002 | `doc-structure-standard` | `PRD/Project` 中承载设计内容，但尚无独立 `Design` 权威源 | 已完成剩余专题回写与互链收口 | 所有高复杂度专题已补齐 `*.design.md` 并迁移内容 | done |
| I8-003 | `doc-structure-standard` | 暂缓补齐专题缺少明确理由与触发条件 | ROUND-008 无剩余缺口，暂缓项已消化 | 后续进入常规增量治理 | done |

## 执行项
| 编号 | 执行动作 | owner role | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A8-001 | 建立 ROUND-008 台账、清单、优先级清单与进度日志 | `producer_system_designer` | 2026-03-10 | `test -f doc/core/reviews/consistency-review-round-008.md && test -f doc/core/reviews/round-008-reviewed-files.md && test -f doc/core/reviews/round-008-design-backfill-priority-list.md && test -f doc/core/reviews/round-008-audit-progress-log.md` | done |
| A8-002 | 完成全量缺口扫描并冻结模块分布统计 | `producer_system_designer` | 2026-03-10 | `rg -n "缺口总数|模块分布" doc/core/reviews/consistency-review-round-008.md doc/core/reviews/round-008-design-backfill-priority-list.md` | done |
| A8-003 | 完成缺口专题复杂度分级 | 各模块 owner | 待定 | `rg -n "must_backfill|should_backfill|defer_allowed" doc/core/reviews/round-008-design-backfill-priority-list.md` | done |
| A8-004 | 执行高优先级专题 `Design` 补齐与互链回写 | 各模块 owner | 待定 | `find doc -type f -name "*.design.md" ! -path "doc/*/design.md" ! -path "doc/devlog/*" | wc -l` | done |
| A8-005 | ROUND-008 复审与验收 | `qa_engineer` | 待定 | `./scripts/doc-governance-check.sh` | done |


## 复审结果
- 复审时间：2026-03-10
- 复审结论：ROUND-008 completed（缺口专题已完成全量分级与高优先级 `Design` 补齐，门禁通过）。
- 当前进展：后续新增专题若再次出现 `PRD + Project` 无 `Design` 缺口，转入 engineering 增量治理，不回退本轮完成结论。
