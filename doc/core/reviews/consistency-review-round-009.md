# Core 文档消费入口与手册语义收口台账（第009轮）

审计轮次: 9

## 目的
- 为 `TASK-CORE-033` / `TASK-ENGINEERING-058` 提供 ROUND-009 的统一执行台账，将本轮目标定义为“文档消费入口与手册语义收口轮”。
- 本轮不再回到全仓结构大扫除，而是聚焦“读者如何进入文档树、哪些文档应被视为手册/运行说明、哪些目录已超出模块职责边界”三个高频使用问题。
- 所有 ROUND-009 结论均以 `doc/engineering/doc-structure-standard.prd.md` 与 `doc/engineering/doc-structure-standard.design.md` 为裁定依据；如发现规范缺口，先补标准，再继续治理。

## 权威依据
- 结构规范需求：`doc/engineering/doc-structure-standard.prd.md`
- 结构规范设计：`doc/engineering/doc-structure-standard.design.md`
- 结构规范执行：`doc/engineering/doc-structure-standard.project.md`
- 工程主台账：`doc/engineering/project.md`
- 项目级 ROUND 台账：`doc/core/project.md`
- 工程总入口：`doc/README.md`
- 系统测试手册：`testing-manual.md`
- 开发工作流：`AGENTS.md`

## 轮次信息
- 轮次编号: `ROUND-009`
- 轮次状态: `in_progress` (`not_started` | `in_progress` | `completed`)
- 轮次类型: `consumer_entry_and_manual_semantics`
- 审查/治理时间窗: `2026-03-30`
- owner role: `producer_system_designer`
- 协作角色: `qa_engineer`（验收/阻断）、`viewer_engineer`（Viewer 手册与静态镜像）、`liveops_community`（对外口径与素材包边界）
- 当前阶段说明: 已完成 focused scope 冻结、首轮问题池登记与执行清单建档；后续按“入口层 -> 手册层 -> 素材/例外层”三批次治理。

状态判定：
- `not_started`: 仅确认轮次意图，尚未冻结范围与问题域。
- `in_progress`: 已冻结 focused scope、问题池、清单与日志，正在按批次审读与回写。
- `completed`: focused scope 内问题已形成保留/迁移/拆分/延期结论，关键互链与入口已回写，复审结论已落档。

## 文档级审计/治理标记方法（缺省=0）
- ROUND-009 采用“小分母 focused scope”模式，不要求对 `doc/**/*.md` 全量回写 `审计轮次: 9`。
- 仅当文档被纳入 ROUND-009 focused scope 且已完成本轮结论回写时，才允许回写 `审计轮次: 9`。
- 若本轮仅完成问题识别、尚未完成迁移/改名/互链修复，则不得提前回写 `审计轮次: 9`。
- 若文档类型需在 `*.prd.md`、`*.manual.md`、`*.runbook.md`、普通素材说明间重落位，必须与对应入口/索引/静态镜像一起同批回写。

## 治理维度（以“可消费性”与“语义准确性”为目标）
| 编号 | 维度 | 治理目标 | 严重度判定 |
| --- | --- | --- | --- |
| D9-001 | 入口可消费性 | 根入口、模块入口、静态 docs hub 能提供清晰的读者起点与阅读顺序 | 入口失焦=high |
| D9-002 | 索引可用性 | 高体量模块的索引不会退化成仅供机器遍历的长表 | 难以消费=high |
| D9-003 | 手册语义准确性 | 手册/运行说明应使用正确载体，不再长期伪装为 `*.prd.md` | 语义漂移=high |
| D9-004 | 模块职责边界 | `readme`、`ui_review_result` 等目录的职责边界清晰，不混合“规范/素材/临时样本” | 边界膨胀=medium |
| D9-005 | 静态镜像一致性 | `site/doc/**` 的公开入口与仓库内权威文档保持同步 | 镜像漂移=medium |
| D9-006 | 例外可追溯性 | 无法立即迁移的 legacy 手册、素材包或样本目录具备明确保留理由与后续触发条件 | 延期失控=medium |

## 总范围（ROUND-009 固定分母）
- focused scope 固定为 `23` 个高频消费/高风险语义文档或静态镜像页面。
- 范围文件记录于 `doc/core/reviews/round-009-reviewed-files.md`。
- 范围分层：
  - 入口层：`README.md`、`doc/README.md`、12 个模块 `README.md`、`site/doc/cn/index.html`、`site/doc/en/index.html`
  - 手册层：`testing-manual.md`、`doc/world-simulator/viewer/viewer-manual.md`、`doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - 边界热点：`doc/readme/prd.index.md`、`doc/readme/README.md`、`doc/ui_review_result/README.md`、静态 `viewer-manual.html`
- ROUND-009 不把其它文档纳入正式分母，除非后续显式追加并同步更新清单。

## 首轮基线（2026-03-30）
- 仓库当前 `doc/` 下实际 `*.manual.md` 数量：`0`
- 仓库当前 `doc/` 下实际 `*.runbook.md` 数量：`2`
- 高频“手册型文档”仍主要存在于以下载体：
  - `doc/world-simulator/viewer/viewer-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - `testing-manual.md`
- 说明：这说明规范层已定义 `manual/runbook`，但消费层仍存在明显 legacy/语义漂移，适合作为 ROUND-009 的主问题域。

## 执行批次（已启动）
- G9-001: 入口层审读与读者路径收口
- G9-002: 手册层语义判定与迁移建议
- G9-003: `readme` / `ui_review_result` 边界热点复核
- G9-004: 静态 docs hub 镜像与仓库权威源对账
- G9-005: 复审、延期项登记与后续治理建议

## focused scope 清单（S_round009）
- 清单文件：`doc/core/reviews/round-009-reviewed-files.md`
- 当前固定分母：`23`
- 字段：`文档路径`、`当前角色`、`关注点`、`建议动作`、`优先级`、`owner role`、`当前状态`、`问题编号`、`备注`

## 进度日志
- 日志文件：`doc/core/reviews/round-009-audit-progress-log.md`
- 记录粒度：专题级/文档级动作；每完成一次范围冻结、问题登记、迁移决议或回写即更新。

## 问题池
| 编号 | 来源 | 问题描述 | 影响范围 | 建议动作 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I9-001 | 入口层抽样 | 当前入口主要按模块/文件类型导航，缺少按读者任务组织的消费层入口 | `README.md`、`doc/README.md`、模块 `README.md`、`site/doc/**` | 先冻结角色/任务型 landing 设计，再决定入口最小改造面 | open |
| I9-002 | 手册层抽样 | 高频手册仍主要落在 legacy `.md` 或 `*.prd.md` 壳子中，`manual` 语义未真正落地 | `viewer-manual.md`、`web-ui-agent-browser-closure-manual.prd.md`、`testing-manual.md` | 判定哪些保留 legacy、哪些迁为 `*.manual.md`、哪些维持权威总手册 | open |
| I9-003 | `readme` 模块抽样 | `readme` 模块混合了 README 权威源、发布口径、执行记录和素材包，消费边界不够清晰 | `doc/readme/**` | 将“规范口径”和“素材/执行包”分层，必要时拆子目录或显式标注类型 | open |
| I9-004 | 目录例外抽样 | `ui_review_result` 当前不在标准模块骨架中，定位更像活跃样本池而非正式模块 | `doc/ui_review_result/**` | 明确保留为临时样本目录、并回写进入/退出条件；或并回所属模块 | open |
| I9-005 | 索引可消费性抽样 | 高体量模块 `prd.index.md` 可达但不够易读，读者难以从长表中快速选择正确主题 | `doc/world-simulator/prd.index.md`、`doc/readme/prd.index.md` 等 | 先做 focused scope 设计结论，再决定是否引入二级索引/过滤式入口 | open |

## 执行项
| 编号 | 执行动作 | owner role | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A9-001 | 建立 ROUND-009 台账、focused scope 清单、kickoff worklist 与进度日志 | `producer_system_designer` | 2026-03-30 | `test -f doc/core/reviews/consistency-review-round-009.md && test -f doc/core/reviews/round-009-reviewed-files.md && test -f doc/core/reviews/round-009-kickoff-worklist.md && test -f doc/core/reviews/round-009-audit-progress-log.md` | done |
| A9-002 | 冻结 focused scope、问题池与批次口径 | `producer_system_designer` | 2026-03-30 | `rg -n "ROUND-009|focused scope|I9-|G9-" doc/core/reviews/consistency-review-round-009.md` | done |
| A9-003 | 完成首轮范围审读并对每个对象给出 keep/migrate/split/defer 建议 | `producer_system_designer` | 待定 | `rg -n "keep|migrate|split|defer" doc/core/reviews/round-009-reviewed-files.md` | in_progress |
| A9-004 | 对高优先级入口/手册对象执行首批回写与互链修复 | 对应模块 owner | 待定 | `./scripts/doc-governance-check.sh` | pending |
| A9-005 | ROUND-009 复审与阻断结论 | `qa_engineer` | 待定 | `./scripts/doc-governance-check.sh` + focused scope 抽样复核 | pending |

## 复审结果
- 当前结论: `in_progress`
- 备注: ROUND-009 已完成 focused scope 冻结与首轮问题登记，后续按小分母治理节奏推进，不回退到全仓逐篇重读模式。
