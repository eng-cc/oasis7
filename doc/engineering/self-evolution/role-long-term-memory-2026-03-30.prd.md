# oasis7：角色长期记忆自建方案（2026-03-30）

- 对应设计文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`

审计轮次: 6

- 对应标准执行入口: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`

## 1. Executive Summary
- Problem Statement: 即使 `.pm/` 文件化项目管理层落地，若“长期 memory”只被笼统定义为 role memory 文件，而没有单独冻结对象模型、promote 规则、`superseded` 生命周期和查询边界，它很容易退化成另一份 `devlog`、另一份 task list，或另一份无法稳定被脚本消费的自由文本笔记。
- Proposed Solution: 在 `engineering/self-evolution` 下新增“角色长期记忆自建方案”专题，单独冻结长期 memory 的目标、对象层次、状态机、promotion/supersede 规则、字段 schema、脚本契约和验证口径。该专题明确长期 memory 是“结构化的、可审计的、以 source-backed 结论为核心的项目语义层”，不承担正式 PRD 真值，也不承担任务执行真值。
- Success Criteria:
  - SC-1: 首批 7 个标准角色全部具备结构一致的长期 memory 容器，且每条 active 记录 100% 带 `id`、`role`、`topic`、`summary`、`source_refs`、`effective_at`、`last_reviewed_at`、`status`。
  - SC-2: 被新结论取代的记录 100% 走 `superseded` 生命周期，带 `superseded_by` 和 `superseded_at`，不得原地覆盖丢失历史。
  - SC-3: `devlog -> signal -> memory` 的提升规则明确，长期 memory 只接受裁决、失败签名、稳定模式、阶段判断、跨天约束等高价值语义，不接收一次性流水操作。
  - SC-4: 角色 memory 的查询视图可被脚本在 1 次扫描内按 `role/topic/status` 枚举，并可生成 active/superseded 报表。
  - SC-5: memory 与 backlog、stage/gate、正式 PRD/project 的职责边界清晰，重复定义和越权写真值的新增违规数为 0。
  - SC-6: 新角色加入时，无需修改既有 memory schema 或迁移旧数据即可接入。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要跨天持久保留阶段判断、禁语边界、已冻结规则、被推翻的旧决策与当前有效结论。
  - `qa_engineer`：需要长期保存 failure signature、阻断条件、历史放行依据和重复出现的问题模式。
  - `liveops_community`：需要持续积累社区高频诉求、事故模式、渠道边界和已确认对外口径。
  - `runtime_engineer` / `agent_engineer` / `viewer_engineer` / `wasm_platform_engineer`：需要记录长期技术约束、热点退化模式、兼容风险和典型分层原则。
  - 治理维护者：需要确保 memory 不是自由发挥的笔记，而是有 schema、有状态机、有 promotion 门槛的正式运行态对象。
- User Scenarios & Frequency:
  - 当日结论固化：每天至少 1 次，把高价值信号提升为长期记忆。
  - 历史结论 supersede：每次阶段判断、规则边界或失败签名被新真值替代时立即执行。
  - 跨天复盘：每周至少 1 次，review active memory 是否过期、冲突或需要复核。
  - 阶段评审准备：每个 release / phase 决策点至少 1 次，从长期 memory 汇总当前有效结论。
  - 新角色接入：新增标准角色时按模板生成 memory 容器并纳入 lint/report。
- User Stories:
  - PRD-ENGINEERING-MEM-001: As a `producer_system_designer`, I want role memories with effective time ranges and superseded links, so that evolving phase decisions remain auditable.
  - PRD-ENGINEERING-MEM-002: As a `qa_engineer`, I want failure signatures and blocker rationales stored as long-term memory instead of daily logs, so that regressions and gate decisions are comparable across weeks.
  - PRD-ENGINEERING-MEM-003: As a `liveops_community`, I want stable community/incident patterns preserved as memory with source refs, so that external signals can influence backlog without being re-triaged from scratch every time.
  - PRD-ENGINEERING-MEM-004: As a governance maintainer, I want a strict promotion contract from signal to memory, so that low-value logs do not pollute long-term memory.
  - PRD-ENGINEERING-MEM-005: As a future role owner, I want the memory schema to be role-agnostic and append-only enough for expansion, so that new roles can join without schema migration.
- Critical User Flows:
  1. Flow-MEM-001: `角色在 devlog / evidence / runbook 中形成高价值结论 -> 生成 signal -> 通过 promote-memory 脚本写入 active memory -> role report 可见`
  2. Flow-MEM-002: `已有 memory 被新事实取代 -> 旧记录写入 superseded_at / superseded_by -> 新记录进入 active -> stage/backlog 引用更新到新记录`
  3. Flow-MEM-003: `QA 发现重复 failure signature -> 复用现有 memory 或创建新 memory -> backlog 条目引用该 memory 而不是重新描述一遍`
  4. Flow-MEM-004: `producer 准备阶段评审 -> 读取 shared/producer active memory -> 汇总当前 claim envelope、阻断边界与最新有效阶段结论`
  5. Flow-MEM-005: `新增角色 -> register-role -> scaffold memory files -> lint 自动纳入 role report`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Active Memory Record | `id`、`role`、`topic`、`summary`、`source_refs[]`、`tags[]`、`effective_at`、`last_reviewed_at`、`status=active`、`confidence`、`promotion_reason` | `promote-memory` 新建 active 记录 | `draft -> active` | 默认按 `role/topic/effective_at desc`；同 topic 最近有效记录优先 | role owner 可创建；跨角色关键结论需 producer 或治理维护者联审 |
| Superseded Memory Record | `id`、`superseded_by`、`superseded_at`、`supersede_reason`、原 active 字段快照 | `supersede-memory` 把 active 移入 superseded | `active -> superseded` | 按 `superseded_at desc`，保留原 `effective_at` | role owner 可 supersede；不得删除历史记录 |
| Memory Promotion Rule | `signal_id`、`role`、`decision`、`promotion_reason`、`rejection_reason` | `promote-memory` 判定 signal 是否足够稳定 | `triaged -> promoted/rejected/deferred` | `stage decision`、`failure signature`、`policy boundary` 高于一般 note | owner role 主责；治理维护者可审计 |
| Memory Query View | `role`、`topic`、`status`、`effective_range` | `role-report` / `memory-report` 生成 active/superseded 报表 | `fresh -> stale/needs_review` | stale 优先按 `last_reviewed_at asc` 报警 | 所有人可读；owner 负责 review |
| Shared Memory | `scope=shared`、`topics`、`source_refs`、`effective_at` | 为跨角色稳定结论建档 | `draft -> active -> superseded` | `shared` topic 在 stage report 中高优先级 | 仅 producer 与治理维护者可写 shared 正式记录 |
- Acceptance Criteria:
  - AC-1: 专题明确长期 memory 与 `devlog`、signal、backlog、stage/gate、正式 PRD/project 的边界。
  - AC-2: 长期 memory schema 明确 active/superseded 两套结构及其必填字段。
  - AC-3: `promote-memory` 与 `supersede-memory` 的输入、输出、失败条件和角色权限明确。
  - AC-4: 至少覆盖 `producer_system_designer`、`qa_engineer`、`liveops_community` 三类高价值 memory 场景，并给出 role-agnostic 扩容规则。
  - AC-5: project 文档给出记忆容器、模板、脚本、report 和验证任务拆解。
  - AC-6: 专题文档与 `self-evolution` 总专题、engineering 根入口、索引和 devlog 全部完成互链。
- Non-Goals:
  - 不在首期实现 embedding、向量检索、图数据库或复杂语义搜索 UI。
  - 不把长期 memory 作为正式 PRD/project 的自动覆盖源。
  - 不让长期 memory 直接承担任务状态管理；任务真值仍在 backlog/task registry。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - `promote-memory`：把 signal 提升为 active memory。
  - `supersede-memory`：把 active record 转为 superseded。
  - `memory-report`：按 role/topic/status 输出报表。
  - `memory-lint`：校验字段完整性、source refs 可达性、superseded 链完整性。
- Evaluation Strategy:
  - 质量：active memory 的低价值噪声率、重复记录率、source ref 缺失率。
  - 审计：superseded 链完整率、可回放率。
  - 可用性：阶段评审和 QA/liveops 回流时，是否能直接引用 memory 而不是重读历史日志。

## 4. Technical Specifications
- Architecture Overview:
  - `.pm/roles/<role>/memory/active.yaml`：当前有效结论。
  - `.pm/roles/<role>/memory/superseded.yaml`：已失效但保留历史链路的结论。
  - `.pm/shared/memory/{active,superseded}.yaml`：跨角色共享记忆。
  - `scripts/pm/promote-memory.sh`、`scripts/pm/supersede-memory.sh`、`scripts/pm/memory-report.sh`、`scripts/pm/memory-lint.sh`：memory 相关脚本层。
- Integration Points:
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
  - `doc/devlog/YYYY-MM-DD.md`
  - `.pm/inbox/signals.jsonl`
  - `.pm/roles/*/backlog/*.yaml`
  - `.pm/stage/*.yaml`
  - `AGENTS.md`
  - `.agents/roles/*.md`
- Edge Cases & Error Handling:
  - 低价值日志误提升：若 signal 只是一次性操作记录，`promote-memory` 必须返回 rejected 并要求保留在 signal/inbox，不得写入长期 memory。
  - 冲突结论并存：同一 `role/topic` 出现互斥 active 记录时，lint 直接失败。
  - source refs 失效：若 memory 指向的 `doc`/signal 路径不存在，lint 直接失败。
  - supersede 缺链：旧记录被标记 superseded 但没有 `superseded_by`，lint 直接失败。
  - shared memory 越权：非 producer/治理维护者试图写 shared 正式 memory 时阻断。
  - stale memory：active memory 长时间未 review 时，report 标记 `needs_review`，但不自动删除。
  - 角色退役：退役角色的 active memory 转为只读历史，不自动并入其他角色。
- Non-Functional Requirements:
  - NFR-MEM-1: `memory-lint` 单次执行时间 <= 10 秒。
  - NFR-MEM-2: active memory 必填字段完整率 100%。
  - NFR-MEM-3: superseded record 的 `superseded_by/superseded_at` 完整率 100%。
  - NFR-MEM-4: `role/topic` 层面 active 记录互斥冲突误放过率为 0。
  - NFR-MEM-5: 新角色接入时无需调整历史 memory 数据格式。
  - NFR-MEM-6: active memory 到 stage/backlog 的引用可达性覆盖率 100%。
- Security & Privacy:
  - memory 只允许记录工程治理语义，不允许复制敏感原文、token、cookie、密钥。
  - 若 incident/runbook 含敏感信息，只记录脱敏摘要和来源引用。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结 memory schema、active/superseded 模型和脚本契约。
  - v1.1: 为 `producer_system_designer` / `qa_engineer` / `liveops_community` 建立首批 memory 模板和样例。
  - v2.0: 覆盖 7 个标准角色和 shared memory。
  - v2.1: 打通 `signal -> memory -> backlog/stage` 引用链和 lint/report。
- Technical Risks:
  - 风险-1: 没有 promotion 门槛，memory 会迅速被流水日志污染。
  - 风险-2: 没有 superseded 机制，历史裁决会被覆盖丢失。
  - 风险-3: 把 memory 和 backlog 混在一起，会导致对象职责漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-MEM-001 | TASK-ENGINEERING-080/082 | `test_tier_required` | active/superseded 时间链、shared memory 与 producer stage 关联验证 | 阶段裁决、历史决策审计 |
| PRD-ENGINEERING-MEM-002 | TASK-ENGINEERING-081/083 | `test_tier_required` | QA failure signature memory 模板、report 与 stale review 验证 | QA block、回归证据链 |
| PRD-ENGINEERING-MEM-003 | TASK-ENGINEERING-081/083 | `test_tier_required` | liveops memory 模板、incident/community pattern 引用链验证 | 社区信号回流、事故复盘 |
| PRD-ENGINEERING-MEM-004 | TASK-ENGINEERING-082/083 | `test_tier_required` | promotion rejection / accepted cases、noise filtering 验证 | signal 提升质量、memory 纯度 |
| PRD-ENGINEERING-MEM-005 | TASK-ENGINEERING-080/083 | `test_tier_required` + `test_tier_full` | 新角色 memory scaffold 与全量 lint/report 扩容验证 | 角色扩容、长期 schema 兼容性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-MEM-001 | 长期 memory 自建在仓库内文件层 | 直接引入外部 memory 产品为真值 | 当前仓库以 Git/worktree 为核心协作环境，本地文件更符合审计与隔离模型。 |
| DEC-MEM-002 | active/superseded 双层模型 | 只保留当前最新记录 | 历史结论与阶段判断需要可回放，不能靠覆盖更新。 |
| DEC-MEM-003 | memory 只接收高价值语义结论 | 把全部 devlog 内容都提升为 memory | 否则 memory 很快退化成流水日志。 |
| DEC-MEM-004 | memory 与 backlog 分层 | 用一套对象同时表达记忆和任务 | 长期结论与执行状态的变更频率、字段和权限都不同。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 明确说明本期解决“长期 memory 若不单独建模会退化”的问题。
  - ✔ 成功指标可量化，覆盖字段完整率、superseded 链和冲突误放过率。
  - ✔ 与 `self-evolution` 总体方向一致。
  - ✔ 优先级来源于当前 role memory 尚未独立冻结 schema。
- 用户与场景（Who / When）:
  - ✔ 明确用户为 producer/QA/liveops/工程 owner/治理维护者。
  - ✔ 区分主用户与扩展用户。
  - ✔ 定义了日常提升、supersede、周复盘、阶段评审、新角色接入等场景。
  - ✔ 频率与关键路径明确。
- 范围定义（Scope Control）:
  - ✔ 功能范围明确为 schema、promotion、supersede、report、lint。
  - ✔ Non-Goals 排除了向量检索、自动覆盖正式 PRD、任务状态管理。
  - ✔ 无隐性功能，矩阵中字段和权限均显式定义。
  - ✔ 版本拆分明确。
- 功能规格（What）:
  - ✔ 功能矩阵完整。
  - ✔ 有交互流程说明。
  - ✔ 字段定义清晰。
  - ✔ 本专题无 UI 按钮，改为脚本动作说明。
  - ✔ 状态变化逻辑清晰。
  - ✔ 排序/计算与权限逻辑明确。
- 异常与边界（Edge Cases）:
  - ✔ 覆盖误提升、冲突 active、断链、越权、stale review、角色退役。
- 非功能需求（NFR）:
  - ✔ 定义了性能、完整性、误放过率、扩容兼容性。
- 可测试性（Testability）:
  - ✔ 定义了验收标准、完成标准、验证方法和回归范围。
- 逻辑一致性（Consistency）:
  - ✔ 与 `self-evolution` 总专题分层一致，无明显冲突。
- 依赖与影响分析（Impact）:
  - ✔ 依赖 `.pm`、signal、backlog、stage、角色卡和总专题均已列出。
- 决策透明度（Decision Record）:
  - ✔ 说明了自建、双层模型和分层的选择原因，并列出否决方案。
- 文档树一致性与结构约束（Documentation Architecture）:
  - ✔ 明确归属于 `doc/engineering/self-evolution/`。
  - ✔ 符合 `prd/design/project` 三件套。
  - ✔ 不重复定义正式业务模型，只定义长期 memory 运行层。
- 总体 Gate 结果: 🟢 Ready
