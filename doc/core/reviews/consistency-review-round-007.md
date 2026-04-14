# Core 文档内容职责边界复核台账（第007轮）

审计轮次: 7

## 目的
- 为 `TASK-CORE-010` / `TASK-ENGINEERING-035` 提供 ROUND-007 的统一执行台账，将本轮目标定义为“内容职责边界复核轮”。
- 本轮聚焦文档内容是否真正符合 `doc-structure-standard` 的职责分层，而不再以命名迁移、入口补齐、历史文件收口为主。
- 所有 ROUND-007 结论均以 `doc/engineering/doc-governance/doc-structure-standard.prd.md` 与 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为裁定依据；如发现标准缺口，先补标准，再继续复核。

## 权威依据
- 结构规范需求：`doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 结构规范设计：`doc/engineering/doc-governance/doc-structure-standard.design.md`
- 结构规范执行：`doc/engineering/doc-governance/doc-structure-standard.project.md`
- 工程主台账：`doc/engineering/project.md`
- 项目级 ROUND 台账：`doc/core/project.md`
- 开发工作流：`AGENTS.md`

## 轮次信息
- 轮次编号: `ROUND-007`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 轮次类型: `content_boundary_review`
- 审查/治理时间窗: `2026-03-09` ~ `2026-03-10`
- owner role: `producer_system_designer`
- 协作角色: `qa_engineer`（验收/阻断）、各模块 owner（按模块内容边界回写）
- 当前阶段说明: 已完成 ROUND-007 全量范围复核（874 份）。模块入口 `design.md` 的旧模板段落命名已完成收敛；其余 `PRD/Design/Project` 通过全量自动扫描与重点抽查，未发现新的高信号职责串层问题。

状态判定：
- `not_started`: 已建立 ROUND-007 台账、清单、批次与问题域，但尚未开始逐文档复核。
- `in_progress`: 已固定范围与批次，正在按批次复核内容职责边界并回写文档。
- `completed`: 本轮范围内内容职责问题已关闭、延期项已登记，复审结论已回写。

## 文档级审计/治理标记方法（缺省=0）
- 每个受复核文档采用字段 `审计轮次: <整数>` 标识最新已完成复核轮次。
- ROUND-007 执行规则：
  - 单篇文档完成 ROUND-007 复核并完成必要回写后，在同一提交回写 `审计轮次: 7`。
  - 若仅识别到问题但未完成内容边界整改，不得提前回写 `审计轮次: 7`。
  - 每次复核必须同时判断该文档是否仍应保留当前类型；若需从 `prd/design/project/manual/runbook` 间重落位，应与 ROUND-007 结果一并回写。
  - 内容边界调整若影响 README / `prd.index.md` / `project.md` / 专题互链，必须同批回写。
- 本轮完成条件：ROUND-007 范围内文档完成内容职责边界复核，混写问题关闭或登记延期，并形成复审结论。

建议统计命令（正式执行后使用）：
```bash
rg -n "^审计轮次:\s*7$" doc --glob '*.md' -g '!doc/devlog/**'
```

## 复核维度（以内容职责边界为目标）
| 编号 | 维度 | 复核目标 | 严重度判定 |
| --- | --- | --- | --- |
| D7-001 | PRD 目标态纯度 | `prd.md` / `*.prd.md` 仅承载 Why / What / Done，不混入执行排期或实现细节 | 混写=high |
| D7-002 | Design 结构契约纯度 | `design.md` / `*.design.md` 仅承载结构、接口、状态、约束与设计决策 | 混写=high |
| D7-003 | Project 执行闭环纯度 | `project.md` / `*.project.md` 仅承载任务、依赖、状态、测试层级与推进口径 | 混写=high |
| D7-004 | Manual/Runbook 角色清晰性 | `manual` / `runbook` 不回流为 PRD/Design/Project 的替代载体 | 越权=medium |
| D7-005 | 文档间权威源边界 | 同一事项存在唯一权威源，引用替代复制，避免多文档重复维护 | 漂移=medium |
| D7-006 | 完成定义一致性 | PRD Done、Design 验证点、Project 完成状态彼此可对齐 | 断裂=medium |

## 总范围（ROUND-007 固定分母）
- `doc/**/*.md`，排除 `doc/devlog/**`。
- 重点对象：模块入口 `prd.md/design.md/project.md` 与活跃专题 `*.prd.md` / `*.design.md` / `*.project.md`。
- `manual` / `runbook` 纳入复核范围，但仅在其内容越权时登记问题。
- ROUND-007 的固定分母沿用当前非 devlog 文档基线：`874`。

## 执行批次（已完成）
- G7-001: 模块入口内容边界复核（模块 `prd/design/project` 与 `README/prd.index` 对齐）
- G7-002: 活跃专题三件套内容边界复核（PRD / Design / Project）
- G7-003: `manual` / `runbook` 越权内容与权威源漂移复核
- G7-004: 复审、延期项登记与门禁回写

## 逐文档复核清单（S_round007）
- 清单文件：`doc/core/reviews/round-007-reviewed-files.md`
- 统计口径：`doc/**/*.md` 排除 `doc/devlog/**`，即 ROUND-007 固定分母。
- 当前基线（2026-03-10，完成态）：`874` 份文档
- 用途：逐文档记录当前类型、边界判定、问题编号、整改动作与完成状态。

## 复核进度日志（逐文档）
- 日志文件：`doc/core/reviews/round-007-audit-progress-log.md`
- 记录粒度：1 文档 1 记录（正式执行后即时写入）。
- 字段：`时间`、`执行角色`、`文档路径`、`复核动作`、`结果(pass/issue_open/blocked)`、`问题编号`、`备注`。
- 当前状态：已完成全量复核、问题收口与复审回写。

## 内容问题池
| 编号 | 来源 | 问题描述 | 影响范围 | 建议动作 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I7-001 | `doc-structure-standard` | PRD 混入执行步骤、排期或实现细节 | 全量已扫描 | 将 How/When/Who 下沉到 `project` / `design` | closed |
| I7-002 | `doc-structure-standard` | Design 缺少结构契约，或混入需求目标/执行推进 | 模块入口 `design.md` 已完成整改并全量复核收口 | 将 Why/What 回收至 PRD，将执行推进回收至 Project | closed |
| I7-003 | `doc-structure-standard` | Project 混入需求定义、接口设计或长期规范条文 | 全量已扫描 | 仅保留任务/依赖/状态/测试层级，其余内容迁回权威源 | closed |
| I7-004 | `doc-structure-standard` | Manual/Runbook 越权承载需求、设计或项目跟踪 | 当前范围 0 命中 | 改为引用权威源并保留操作说明/运行指引 | closed |
| I7-005 | `doc-structure-standard` | 多文档对同一事项重复维护，权威源不唯一 | 全量标题/职责扫描未发现新增高信号问题 | 明确唯一权威源并改为链接引用 | closed |

## 执行项
| 编号 | 执行动作 | owner role | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A7-001 | 建立 ROUND-007 内容职责边界复核台账（本文件 + 清单 + 工作清单 + 进度日志） | `producer_system_designer` | 2026-03-09 | `test -f doc/core/reviews/consistency-review-round-007.md && test -f doc/core/reviews/round-007-reviewed-files.md && test -f doc/core/reviews/round-007-kickoff-worklist.md && test -f doc/core/reviews/round-007-audit-progress-log.md` | done |
| A7-002 | 冻结 ROUND-007 范围、批次、问题池与判定口径 | `producer_system_designer` | 2026-03-09 | `rg -n "总范围|执行批次|G7-|I7-" doc/core/reviews/consistency-review-round-007.md` | done |
| A7-003 | 生成逐文档复核清单并标注字段定义 | `producer_system_designer` | 2026-03-09 | `rg -n "当前类型|边界判定|问题编号|整改动作|状态" doc/core/reviews/round-007-reviewed-files.md` | done |
| A7-004 | 按批次执行内容职责边界复核与文档回写 | 各模块 owner | 2026-03-10 | `rg -n "当前已完成复核文档数: 874|ROUND-007 总范围（\`doc/**/*.md\` - \`doc/devlog/**\`） \| 874 \| completed" doc/core/reviews/round-007-reviewed-files.md` | done |
| A7-005 | ROUND-007 复审与验收 | `qa_engineer` | 2026-03-10 | `python - <<'PY' ...` 全量边界启发式扫描 + 样本抽查 | done |

## 复审结果
- 当前结论: `completed`
- 备注: ROUND-007 已完成全量内容职责边界复核（874 份）；共修正 12 份模块 `design.md` 的旧模板段落命名，其余文档经自动扫描与重点抽查未发现新增高信号串层问题。
