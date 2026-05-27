# 文档迁移并行协作方案（2026-03-03）

- 对应设计文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.design.md`
- 对应项目管理文档: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`

审计轮次: 4
- 对应标准执行入口: `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`

## 1. Executive Summary
- Problem Statement: 当前仓库仍有 391 份老格式专题项目文档待迁移，单人串行推进成本高、周期长，已成为文档治理的关键瓶颈。
- Proposed Solution: 采用四人并行迁移机制，按目录前缀拆分互斥范围，统一迁移原则、提交流程、复核标准与进度口径。
- Success Criteria:
  - SC-1: `391/391` 待迁移目标全部完成新命名迁移（`*.md -> *.prd.md`，`*.project.md -> *.project.md`）。
  - SC-2: 每份迁移后的 PRD 均对齐 strict 6 章结构，并附“原文约束点映射（内容保真）”。
  - SC-3: 每个迁移任务均满足“一任务一提交 + devlog + 文档治理校验”闭环。
  - SC-4: 四人并行迁移过程中文件冲突数为 0（目录边界互斥）。
  - SC-5: 模块级 `prd.md` / `project.md` 对专题迁移任务保持 100% PRD-ID 映射。

## 2. User Experience & Functionality
- User Personas:
  - 协调人（Migration Lead）：负责分工切片、节奏管理、冲突仲裁、收口验收。
  - 迁移执行人（Owner-A/B/C/D）：负责所辖目录逐篇阅读与人工重写迁移。
  - 质量复核人（Reviewer，可由协调人兼任）：负责抽检内容保真、命名规范、引用完整性。
- User Scenarios & Frequency:
  - 每日开始前：同步迁移池与分工状态，确认“无人抢占、无遗漏”。
  - 每完成一篇：执行文档治理校验，提交 commit，追加当日日志。
  - 每日收口：更新燃尽统计（已迁移/剩余/阻塞项）。
- User Stories:
  - PRD-ENGINEERING-005: As a 迁移协调人, I want a single collaboration document with clear principles and owner scopes, so that 4-person parallel migration is deterministic.
  - PRD-ENGINEERING-006: As a 迁移执行人, I want non-overlapping file scopes and commit rules, so that I can migrate continuously without merge conflicts.
  - PRD-ENGINEERING-007: As a 质量复核人, I want measurable acceptance and evidence format, so that migrated docs remain content-faithful and auditable.
- Critical User Flows:
  1. Flow-ENG-MIG-001: `生成待迁移快照 -> 按目录切片 -> 分配 Owner-A/B/C/D -> 冻结责任边界`
  2. Flow-ENG-MIG-002: `Owner 领取单篇文档 -> 阅读旧 .md/.project.md -> 人工重写 -> 重命名为 .prd/.project`
  3. Flow-ENG-MIG-003: `更新模块 prd/project -> 追加 devlog -> 执行 doc-governance-check -> 单任务提交`
  4. Flow-ENG-MIG-004: `每日汇总 -> 更新燃尽 -> 调整下一个批次`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 待迁移池快照 | 路径、模块、子目录、Owner | 每日生成并冻结当日基线 | `inventory -> assigned -> done` | 按模块规模与风险优先级排序 | 协调人可修改，Owner 只读 |
| 并行分工 | Owner、目录范围、目标数量 | 领取并执行本范围文档 | `idle -> in_progress -> blocked/done` | 范围互斥，不允许跨区抢文件 | 协调人分配，Owner 执行 |
| 迁移执行 | 旧文档路径、新文档路径、PRD-ID、任务号 | 人工重写并提交 | `draft -> validated -> committed` | 每任务单文档，单独 commit | Owner 可写，Reviewer 可驳回 |
| 质量复核 | 命名规范、章节完整、保真映射、引用可达 | 抽检与问题回退 | `pending_review -> pass/fail` | 失败任务优先修复 | Reviewer 与协调人可写 |
| 燃尽跟踪 | 总量、已完成、剩余、阻塞 | 日更看板 | `tracking -> updated` | 统计口径固定为待迁移池快照 | 协调人维护 |
- Acceptance Criteria:
  - AC-1: 形成唯一协作入口文档，明确四人分工、边界和节奏。
  - AC-2: 提供可执行待迁移池快照，并可追溯到具体路径。
  - AC-3: 迁移原则明确禁止脚本批量改写正文，必须逐篇阅读后人工重写。
  - AC-4: 明确命名规范、提交规范、日志规范与测试校验规范。
  - AC-5: 形成分工完成后的汇总收口规则（燃尽、抽检、收尾）。
- Non-Goals:
  - 不在本方案内改动业务代码或测试脚本实现。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - 文件扫描：`find`, `rg`
  - 治理校验：`./scripts/doc-governance-check.sh`
  - 进度追踪：模块 `project.md` 与 `doc/devlog/YYYY-MM-DD.md`
- Evaluation Strategy:
  - 以“迁移完成率、抽检通过率、冲突率、回退率”四个指标评估并行迁移质量。

## 4. Technical Specifications
- Architecture Overview: 采用“模块级追踪 + 专题级执行 + 日志级证据”的三层治理架构。模块主文档负责总追踪，专题文档负责逐篇迁移执行，devlog 负责时序证据。
- Integration Points:
  - 协作主文档：`doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
  - 协作项目文档：`doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`
  - 待迁移清单快照：`doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md`
  - 模块追踪：`doc/engineering/prd.md`、`doc/engineering/project.md`
  - 任务日志：`doc/devlog/README.md`
- Owner-C 执行分批（TASK-ENGINEERING-013）：
  - Batch-C2: `doc/headless-runtime/**`（4 篇）
  - Batch-C3: `doc/world-runtime/governance/**` + `doc/world-runtime/module/**` + `doc/world-runtime/wasm/**`（9 篇）
  - Batch-C4: `doc/world-runtime/runtime/**`（17 篇）
- Edge Cases & Error Handling:
  - 分工冲突：若同一文档被多人认领，以协作项目文档登记的 Owner 为唯一执行者，其余回退。
  - 内容不保真：复核发现约束缺失时，任务状态回退为 `in_progress`，不得合并。
  - 命名偏差：未转为 `.prd.md/.project.md` 的提交视为未完成。
  - 引用断链：迁移后引用不可达时必须在同提交修复。
  - 统计漂移：待迁移池口径变更时必须更新快照日期与差异说明。
  - 根入口 redirect 争议：对已切为 legacy redirect 的根入口项目文档允许拆分为独立子阶段执行，但必须在项目文档标注“剩余项 + 计划日期”。
- Non-Functional Requirements:
  - NFR-1: 每天至少完成 16 篇迁移（4 人 * 人均 4 篇）以控制总周期。
  - NFR-2: 迁移抽检通过率 >= 95%。
  - NFR-3: 迁移任务从领取到提交的平均周期 <= 1 个工作日。
  - NFR-4: 每个迁移提交均可追溯到 PRD-ID 与任务号。
- Security & Privacy: 文档迁移不应引入本地绝对路径、密钥信息或隐私数据；如原文含敏感样例，迁移时需脱敏。

## 5. Risks & Roadmap
- Phased Rollout:
  - Phase-0 (2026-03-03): 完成协作文档、分工与待迁移快照冻结。
  - Phase-1: Owner-A/B/C/D 按目录并行迁移，按日收口。
  - Phase-1 进展（2026-03-03）: Owner-A 已完成 `doc/world-simulator/**` 146 篇、Owner-B 已完成 `doc/p2p/**` 70 篇、Owner-C 已完成 `doc/world-runtime/**`+`doc/headless-runtime/**` 30 篇，Owner-D 责任域 57 篇已完成。
  - Phase-2: 全量收尾抽检，清理残留旧命名与断链引用。
- Technical Risks:
  - 风险-1: 超大目录（`world-simulator`、`p2p`）导致 Owner 负载不均。
  - 风险-2: 多人并行期间对模块主项目文档产生写冲突。
  - 风险-3: 赶进度导致“格式合规但语义失真”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-005 | TASK-ENGINEERING-010 | `test_tier_required` | 校验协作主文档与项目文档结构、分工边界、待迁移快照挂载 | 多人协作入口清晰度 |
| PRD-ENGINEERING-006 | TASK-ENGINEERING-011/012/013/014 | `test_tier_required` | 抽样检查四个 Owner 是否仅修改分配范围文件 | 并行效率与冲突控制 |
| PRD-ENGINEERING-007 | TASK-ENGINEERING-015 | `test_tier_required` + `test_tier_full` | 全量迁移收尾扫描、命名一致性与引用可达性复核 | 全仓文档治理质量 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-ENG-MIG-001 | 按目录前缀切四个互斥责任域 | 按文件随机平均分配 | 目录切分可显著降低冲突与上下文切换成本。 |
| DEC-ENG-MIG-002 | 逐篇人工重写，不做脚本批量改写正文 | 全自动脚本迁移 | 保证语义与历史约束不丢失。 |
| DEC-ENG-MIG-003 | 采用“单任务单提交 + devlog + 治理校验”强约束 | 批量合并多个迁移任务 | 便于回溯、审计与失败回退。 |

### 原文约束点映射（内容保真）
- 约束-1（迁移耗时高）: 通过第 1 章问题定义和第 5 章分阶段路线回应。
- 约束-2（四人并行）: 通过第 2 章角色与流程、第 4 章分工边界实现。
- 约束-3（梳理待迁移文档）: 通过第 4 章集成点挂载待迁移快照文档实现。
- 约束-4（在文档中写清分工和原则）: 本文作为唯一协作入口，明确原则、分工、验收与决策。
