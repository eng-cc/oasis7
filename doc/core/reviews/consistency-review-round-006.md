# Core 文档结构治理执行台账（第006轮）

审计轮次: 6

## 目的
- 为 `TASK-CORE-010` / `TASK-ENGINEERING-035` 提供 ROUND-006 的统一执行台账，将本轮目标从“纯审计”升级为“按 `doc-structure-standard` 逐文档治理与改造”。
- 本轮不仅登记问题，还要求对不符合规范的文档逐个执行改名、拆分、补齐、互链回写与入口回写，直到文档树结构符合规范。
- 所有 ROUND-006 结论均以 `doc/engineering/doc-governance/doc-structure-standard.prd.md` 为裁定依据；如发现规范空白，再先回写规范，再继续治理。

## 权威依据
- 结构规范需求：`doc/engineering/doc-governance/doc-structure-standard.prd.md`
- 结构规范执行：`doc/engineering/doc-governance/doc-structure-standard.project.md`
- 工程主台账：`doc/engineering/project.md`
- 项目级 ROUND 台账：`doc/core/project.md`
- 开发工作流：`AGENTS.md`

## 轮次信息
- 轮次编号: `ROUND-006`
- 轮次状态: `completed` (`not_started` | `in_progress` | `completed`)
- 轮次类型: `structure_governance`
- 审查/治理时间窗: `2026-03-09`
- owner role: `producer_system_designer`
- 协作角色: `qa_engineer`（验收/阻断）、各模块 owner（按目录落地改造）
- 当前阶段说明: 已完成全量范围治理与命名收口（当前分母 870 份），`.project.md` 已成为唯一执行入口。

状态判定：
- `not_started`: 已完成台账、批次骨架和字段定义，但尚未开始逐文档治理。
- `in_progress`: 已固定总范围并生成清单，正在按批次逐文档改造并同步回写索引/引用/状态。
- `completed`: 本轮范围内文档已完成治理，结构违规项关闭或已登记批准延期。

## 文档级审计/治理标记方法（缺省=0）
- 每个受治理文档采用字段 `审计轮次: <整数>` 标识最新已完成治理轮次。
- ROUND-006 执行规则：
  - 单篇文档完成 ROUND-006 治理并通过该轮复核后，在同一提交回写 `审计轮次: 6`。
  - 若仅发现问题但未完成结构改造，不得提前回写 `审计轮次: 6`。
  - 每次治理必须同步检查并回写：模块入口、`prd.index.md`、专题互链、历史引用、project 映射。
  - 若文档内容职责本身不清，需先判定应落为 `*.prd.md` / `*.design.md` / `*.project.md` / `*.manual.md` / `*.runbook.md` 中的哪一种，再做改造。
- 本轮完成条件：ROUND-006 总范围文档全部完成结构治理、引用回写与复审结论；批次仅用于执行切片，不改变总范围。

建议统计命令（正式执行后使用）：
```bash
rg -n "^审计轮次:\s*6$" doc --glob '*.md' -g '!doc/devlog/**'
```

## 治理维度（以结构改造为目标）
| 编号 | 维度 | 治理目标 | 严重度判定 |
| --- | --- | --- | --- |
| D6-001 | 类型落位 | 每篇文档落到正确职责后缀：`prd/design/project/manual/runbook` | 错放=high |
| D6-002 | basename 一致性 | 同专题 `PRD / Design / Project` 使用同一 basename | 不一致=high |
| D6-003 | 模块入口完整性 | 模块入口满足 `README.md`、`prd.md`、`project.md`、`prd.index.md`，必要时补 `design.md` | 缺失=high |
| D6-004 | 互链与索引 | 模块入口、专题文档、project 文档与索引之间双向可达 | 断链=high |
| D6-005 | 内容职责边界 | Why/What/Done 留在 PRD；How/Structure/Contract 留在 Design；How/When/Who 留在 Project | 混写=medium |
| D6-006 | 目录对象语义 | 目录表达模块/专题/分册，对象层级清晰且可判定 | 语义漂移=medium |
| D6-007 | 历史命名收敛 | 旧命名、临时命名、无日期稳定名策略符合规范 | 偏差=medium |

## 总范围（ROUND-006 固定分母）
- `doc/**/*.md`，排除 `doc/devlog/**`。
- 包含模块入口、专题文档、`manual` / `runbook`、根级入口与 redirect 文档。
- 总范围为本轮固定分母；后续批次仅表示执行顺序，不改变是否纳入 ROUND-006。

## 执行批次（已完成）
- G6-001: 模块入口治理（`README.md` / `prd.md` / `project.md` / `prd.index.md` / `design.md`）
- G6-002: 专题级 basename / 类型落位 / 三件套治理
- G6-003: 索引、互链、历史引用与 redirect 收口

## 逐文档治理清单（S_round006）
- 清单文件：`doc/core/reviews/round-006-reviewed-files.md`
- 统计口径：`doc/**/*.md` 排除 `doc/devlog/**`，即 ROUND-006 全量治理分母。
- 当前基线（2026-03-09，已完成最终收口）：`870` 份文档
- 用途：作为 ROUND-006 固定分母，逐文档记录当前类型、目标类型、改造动作与完成状态。

## 治理进度日志（逐文档）
- 日志文件：`doc/core/reviews/round-006-audit-progress-log.md`
- 记录粒度：1 文档 1 记录（正式执行后即时写入）。
- 字段：`时间`、`执行角色`、`文档路径`、`改造动作`、`结果(pass/issue_open/blocked)`、`问题编号`、`备注`。
- 当前状态：已完成全量治理、互链回写、命名收口与快速复审。

## 结构问题池
| 编号 | 来源 | 问题描述 | 影响范围 | 建议动作 | 当前判定 |
| --- | --- | --- | --- | --- | --- |
| I6-001 | `doc-structure-standard` | 文档职责后缀与内容边界不一致 | 全仓文档 | 按职责重落位并回写入口/索引 | closed |
| I6-002 | `doc-structure-standard` | 同专题 basename 不一致，相关文档不可发现 | 全仓文档 | 主从收敛或同名化改造 | closed |
| I6-003 | `doc-structure-standard` | 模块入口或专题三件套缺失 | 全仓文档 | 补齐入口/设计/项目文档并互链 | closed |
| I6-004 | `doc-structure-standard` | 索引与互链未随改名/拆分同步 | 全仓文档 | 统一回写索引、README 与 project 互链 | closed |
| I6-005 | `doc-structure-standard` | PRD / Design / Project 内容职责混写 | 全仓文档 | 拆分内容并明确唯一权威源 | closed |

## 执行项
| 编号 | 执行动作 | owner role | 截止时间 | 验收命令 | 状态 |
| --- | --- | --- | --- | --- | --- |
| A6-001 | 建立 ROUND-006 结构治理执行台账（本文件 + 治理清单 + 执行清单 + 进度日志） | `producer_system_designer` | 2026-03-09 | `test -f doc/core/reviews/consistency-review-round-006.md && test -f doc/core/reviews/round-006-reviewed-files.md && test -f doc/core/reviews/round-006-kickoff-worklist.md && test -f doc/core/reviews/round-006-audit-progress-log.md` | done |
| A6-002 | 冻结 ROUND-006 执行批次与模块 owner，并确认“全量范围、分批执行”的分母口径 | `producer_system_designer` | 2026-03-09 | `rg -n "总范围|执行批次|G6-|I6-" doc/core/reviews/consistency-review-round-006.md` | done |
| A6-003 | 生成逐文档治理台账并标注当前/目标类型、改造动作与依赖回写项 | `producer_system_designer` | 2026-03-09 | `test "$(rg -c "^\| `doc/" doc/core/reviews/round-006-reviewed-files.md)" -eq 870` && rg -n "当前类型|目标类型|改造动作|索引回写|引用回写|状态" doc/core/reviews/round-006-reviewed-files.md` | done |
| A6-004 | 按批次执行文档改造并同步回写入口/索引/引用 | 各模块 owner | 2026-03-09 | `find doc -type f -name "*.project.md" | wc -l`（配合 legacy=0 验证） | done |
| A6-005 | ROUND-006 复审与门禁验收 | `qa_engineer` | 2026-03-09 | `python - <<'PY' ...` 快速结构校验 + 引用可达性校验 | done |

## 复审结果
- 当前结论: `completed`
- 备注: ROUND-006 已改为“结构治理轮”，已完成全仓执行入口统一为 `.project.md`，legacy 文件与引用均已清零（总计 870 份文档）。
