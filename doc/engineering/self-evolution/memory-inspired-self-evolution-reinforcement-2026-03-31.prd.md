# oasis7：记忆启发式自我进化补强（2026-03-31）

- 对应设计文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`

审计轮次: 6

- 对应标准执行入口: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`

## 1. Executive Summary
- Problem Statement: 当前 `self-evolution` 与长期 memory 专题已经冻结了 `.pm/` 文件真值、role memory schema 与 `signal -> memory/task` 基础链路，但仍缺少两层关键规格：一是“外部 memory/reflective agent 方案里哪些值得借鉴、哪些必须拒绝”；二是“做事过程中的会话/临时判断该如何被记录，而不污染长期 memory”。若不先冻结这层借鉴与分层边界，后续很容易把“记忆增强”“反思归纳”“过程笔记”“自我进化自治”混成一个不受控的大口袋。
- Proposed Solution: 在 `engineering/self-evolution` 子专题中，对 `memoryOSS` 与论文《Hindsight》做结构化对照，并新增 `working_memory` 与 `conversation transcript analysis` 分层。明确 oasis7 只借鉴记忆分类、预算化召回、namespace 隔离、会话到工作记忆的提炼与反思审核链路；对 Codex/engineering task，phase 1 允许从 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl` 读取 raw evidence，若 `history.jsonl` 无该会话消息则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，但默认不把“同一 live session 自读”当作收口路径。owner 若确实要从 `.codex` transcript 提炼 `working_memory`，必须显式提供 `--session-id`，或显式传 `--allow-auto-session` 做 opt-in；同时暂不把 wrapper 导出的 transcript artifact 设为前置依赖，也不允许 agent 直接绕过 owner/worktree/QA 链路执行高风险改动。
- Success Criteria:
  - SC-1: 新专题明确区分 `memory_kind = fact | experience | summary | belief` 四类记忆，并给出与现有 `.pm/roles/*/memory/*.yaml` 的字段映射。
  - SC-2: 对 `belief`/暂定判断类记忆，100% 定义 `confidence`、`review_due_at`、`superseded_by`/`superseded_at` 等审计字段，不允许无限期以“猜测”身份停留为 active 真值。
  - SC-3: 召回策略 100% 通过 repo 内可审计配置表达 `role`、`phase`、`kind_allowlist`、`max_items`、`budget_chars` 与 `freshness_days`，禁止无上限把历史 memory 全量注入上下文。
  - SC-4: 反思产物 100% 先进入 `signal`/候选对象，再由 owner 决定提升为 memory、task 或 rejected，不允许自动覆盖正式 PRD / project。
  - SC-5: 专题对 `memoryOSS` / `Hindsight` 的 adopted / rejected / deferred 项均给出明确理由，并与现有 `self-evolution` / `role-long-term-memory` 边界保持零冲突。
  - SC-6: 会话记录只能作为 `raw evidence` 输入；对 Codex/engineering task，phase 1 允许使用 `~/.codex/session_index.jsonl` + `~/.codex/history.jsonl`，若 `history.jsonl` 无该会话消息则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`（`~/.codex/logs_1.sqlite` 仅作可选后续解析层），但默认不得依赖“同一 live session 自读”作为关闭任务的标准步骤。若要从 `.codex` transcript 提炼，必须显式指定 `session_id` 或显式 opt-in 自动解析，且 100% 先提炼到 `working_memory` 或 `signal`，不得整段会话直接写入长期 memory。
  - SC-7: `working_memory` 100% 带 `task_uid`、`role`、`worktree_hint`、`entry_kind`、`source_refs`、`captured_at`、`expires_at` 字段，并在任务关闭后显式清理、转 task、转 signal 或丢弃。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要判断外部“memory agent”方案是否会破坏 oasis7 的 owner/worktree/审计链。
  - `agent_engineer`：需要知道后续在线 Agent 可以借用哪些记忆分类、召回预算与反思流程。
  - `qa_engineer`：需要保证 failure signature、回归模式与反思结论不会因“智能记忆”引入更多噪声。
  - `liveops_community`：需要把社区/事故模式沉淀为可追踪经验，而不是让 agent 私有会话状态吞掉外部反馈。
  - 当前 task owner：需要在一个 worktree 内保留“试过什么、为什么放弃、下一步验证什么”的过程记忆，而不把这些临时内容误升格为长期结论。
  - 仓库治理维护者：需要保证 `.pm/` 仍是仓库内唯一运行态真值，不被外部 memory 产品或隐式会话状态替代。
- User Scenarios & Frequency:
  - 外部方案评估：每次考虑引入新 memory/agent 产品、论文思路或上下文工程机制时执行。
  - 工作流补强设计：每次要扩展 `workflow-report`、`memory-report`、`role-report` 的召回语义时执行。
  - 任务内过程跟踪：当单个任务跨多轮会话、多次试错或多次 handoff 时执行。
  - 重复失败复盘：当同类 failure signature、社区反馈或阶段误判连续出现时执行。
  - 长期治理复核：每个 release / phase 评审前至少 1 次，确认当前 memory 与 reflection 规则仍符合审计边界。
- User Stories:
  - PRD-ENGINEERING-MIR-001: As a `producer_system_designer`, I want external memory ideas mapped to file-native oasis7 objects, so that future self-evolution upgrades stay auditable.
  - PRD-ENGINEERING-MIR-002: As an `agent_engineer`, I want budgeted recall policies by role/phase/memory kind, so that runtime agents consume only the memory slice they actually need.
  - PRD-ENGINEERING-MIR-003: As a `qa_engineer`, I want reflection outputs gated through signals and owner review, so that repeated failures produce structured learning without bypassing validation.
  - PRD-ENGINEERING-MIR-004: As a governance maintainer, I want adopted/rejected external patterns documented with rationale, so that the system evolves by explicit decisions instead of ad hoc imitation.
  - PRD-ENGINEERING-MIR-005: As a future role owner, I want `fact/experience/summary/belief` memory kinds to coexist with the existing schema, so that new roles can join without a storage migration reset.
  - PRD-ENGINEERING-MIR-006: As a current task owner, I want conversation transcripts and intermediate reasoning distilled into task-scoped working memory, so that useful process context survives within a worktree without polluting long-term memory.
- Critical User Flows:
  1. Flow-MIR-001: `producer_system_designer` 评估外部方案 -> 按 adopted / rejected / deferred 维度记录可借鉴点 -> 回写本专题 PRD/design/project -> 后续实现任务仅从正式文档读取口径。
  2. Flow-MIR-002: 角色进入 `workflow-report --phase start/review` -> 按 `role + phase + kind_allowlist + max_items + budget_chars` 读取预算化 memory 视图 -> 仅把当前步骤所需 memory 注入上下文。
  3. Flow-MIR-003: 运行或评审过程中出现“重复模式/新的判断” -> 先以 `source_type=reflection` 或等价 signal 进入 inbox -> owner 审核后提升为 `task`、`memory` 或 `rejected/deferred`。
  4. Flow-MIR-004: `belief` 类 active memory 与新事实冲突 -> 旧记录进入 `superseded`，新记录根据证据改写为 `fact`、`summary` 或新的 `belief` -> stage/backlog 引用更新到新记录。
  5. Flow-MIR-005: 新角色或新工作流需要记忆扩容 -> 复用 role-agnostic schema 与 recall profile -> lint/report 在不迁移历史数据的前提下自动纳入。
  6. Flow-MIR-006: Codex/engineering task 若确实需要从 `.codex` transcript 提炼过程记忆，owner 先显式提供 `session_id`，或显式 opt-in 自动 session 解析；系统随后从 `~/.codex/session_index.jsonl` + `~/.codex/history.jsonl` 读取会话 raw evidence，若命中为空则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，再结合 task execution log / 手工 evidence 进入系统 -> 提取 `attempt/hypothesis/decision/open_question/next_step` 到 `working_memory` -> 当内容稳定时再提升为 reflection signal、task 或长期 memory -> 任务关闭时清理或归档剩余 working memory。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 记忆分类层 | `memory_kind=fact|experience|summary|belief`、`confidence`、`review_due_at`、`recall_priority` | `promote-memory` 新增记录时必须选择 kind；`belief` 必须附 `confidence` 与 review 截止 | `draft -> active -> needs_review -> superseded/retired` | `fact` 高于 `summary`，`experience` 高于 `belief`；同类内按 `recall_priority`、`effective_at desc` 排序 | role owner 可写本角色；`shared` 仍仅 producer/治理维护者可写 |
| Recall Profile | `profile_id`、`role`、`phase`、`kind_allowlist[]`、`max_items`、`budget_chars`、`freshness_days`、`topic_filters[]` | `workflow-report` / `memory-report` 根据 profile 输出预算化视图；超预算时截断并给出原因 | `draft -> active -> superseded` | 先按 `kind` 权重、再按 `recall_priority`、再按 freshness 排序；超出 `budget_chars` 的条目不注入 | 所有人可读；owner role 与 producer 决定各自 profile |
| Working Memory | `task_uid`、`role`、`worktree_hint`、`entry_id`、`entry_kind=attempt|hypothesis|decision|open_question|next_step`、`summary`、`source_refs[]`、`captured_at`、`expires_at`、`promoted_to[]` | 会话/过程分析先写入 task-scoped `working_memory`；Codex/engineering task 的 phase 1 仅在 owner 显式指定 `session_id` 或显式 opt-in 自动 session 解析后，才从 `~/.codex/session_index.jsonl` + `~/.codex/history.jsonl` 提炼，若命中为空则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，owner 决定保留、提炼成 signal/task、或丢弃 | `captured -> active -> promoted/discarded/expired` | 默认按 `captured_at desc`；`decision/next_step` 高于 `attempt`；过期条目不进入 recall | 当前 task owner 可写；handoff 接收方可续写；不得直接当正式真值 |
| Reflection Signal Contract | `source_type=reflection`、`summary`、`source_ref`、`role_hint`、`severity`、`candidate_kind`、`candidate_topic` | `promote-signal` 写入反思信号；owner 决定 `promoted_candidate_task`、`promoted_memory`、`rejected` 或 `deferred` | `new -> triaged -> promoted/rejected/deferred` | 已有同 topic active memory 时优先复用/更新，避免重复创建 | 全角色可提交；对应 owner 负责处置；QA/producer 可阻断高风险反思 |
| Belief Review Gate | `confidence`、`review_due_at`、`last_reviewed_at`、`contradicted_by[]` | `memory-report` / `workflow-report` 对过期 `belief` 标记 `needs_review`；`supersede-memory` 处理冲突 | `active -> needs_review -> superseded/retired` | 过期 `belief` 在 review 视图优先级高于普通 stale memory | owner role 负责复核；producer 可升级为阶段阻断 |
| External Inspiration Matrix | `source_name`、`source_ref`、`pattern`、`decision=adopted|rejected|deferred`、`rationale`、`target_object` | 设计/PRD 中冻结 adopted / rejected 项；后续实现只能引用已 adopted 模式 | `proposed -> adopted/rejected/deferred -> superseded` | adopted 项按影响范围和依赖前置排序 | 仅 producer_system_designer 可冻结正式结论；相关工程 owner 联审实现影响 |
- Acceptance Criteria:
  - AC-1: 新专题明确给出 `memoryOSS` 与 `Hindsight` 的 adopted / rejected / deferred 结构化映射，且不与现有 `self-evolution` / `role-long-term-memory` 冲突。
  - AC-2: `fact/experience/summary/belief` 四类 memory 的定义、必填字段、提升门槛与 supersede 规则写清。
  - AC-3: 召回策略必须显式限制 `max_items` / `budget_chars` / `freshness_days`，并说明超预算时的截断行为。
  - AC-4: 反思产物必须先走 signal/owner review，再决定是否进入 memory/task；不得自动覆盖正式文档或直接触发高风险代码修改。
  - AC-5: 会话记录与过程日志必须先提炼到 task-scoped `working_memory`，不得整段直接写入长期 memory；Codex/engineering task 的 phase 1 必须优先读取 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl`，若 `history.jsonl` 无该会话消息则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，而不是先要求 wrapper 导出独立 transcript artifact。
  - AC-6: project 文档至少拆出“建档、schema 扩展、召回 profile、working memory、reflection 契约、验证回归”六类任务。
  - AC-7: 专题文档、engineering 根入口、索引、主项目与相关 task execution log 全部完成互链。
  - AC-8: `working_memory` 必须定义过期/清理/提升规则，避免任务结束后残留未治理临时认知。
- Non-Goals:
  - 不把 `memoryOSS`、向量数据库或外部 SaaS 接入为 oasis7 首期运行态真值。
  - 不在本期引入 embedding 检索、图数据库、远程同步记忆服务或自动学习型权重更新。
  - 不允许 agent 根据反思结果绕过 owner/worktree/QA 流程直接修改正式 PRD、代码或 stage 结论。
  - 不把“记忆增强”误写成“自治执行授权”。
  - 不把完整会话 transcript 原文直接复制进长期 memory。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - 会话记录分析器：对 Codex/engineering task 优先读取 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl`，若 `history.jsonl` 无该会话消息则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，将 transcript / 临时过程记录提炼为 `working_memory` 条目，并附 source refs。
  - `scripts/pm/promote-signal.sh`：承接 `source_type=reflection` 的反思输入。
  - `scripts/pm/promote-memory.sh` / `scripts/pm/supersede-memory.sh`：承接 memory kind、belief review 与 supersede 规则。
  - `scripts/pm/memory-report.sh` / `scripts/pm/workflow-report.sh`：输出预算化 recall 视图。
  - 文档治理与 smoke：`./scripts/doc-governance-check.sh`、`./scripts/pm/lint.sh`、未来 `required-tier/full-tier` memory recall smoke。
- Evaluation Strategy:
  - 召回质量：同一 role/phase 下，注入 memory 的低价值噪声率、重复率、超预算截断率。
  - 过程提炼质量：transcript 到 `working_memory` 的结构化抽取准确率、重复条目率、过期残留率。
  - 反思质量：reflection signal 被 rejected 的比例、转化为有效 task/memory 的比例、重复 failure signature 的复发率变化。
  - 审计一致性：`belief` 过期未 review 数、superseded 链完整率、外部 adopted/rejected 决策可回放率。

## 4. Technical Specifications
- Architecture Overview:
  - 正式真值仍分层为 `doc/**` + `.pm/**`；本专题只扩展 `.pm` memory 与 workflow 的对象模型，不引入第二真值系统。
  - `memoryOSS` 提供的借鉴点仅限本地优先、显式 mode/namespace、预算化召回与 fail-open 工程习惯；不引入其产品形态作为正式依赖。
  - 《Hindsight》提供的借鉴点仅限 `fact/experience/summary/belief` 记忆分层，以及 `retain/recall/reflect` 闭环；不把论文实验结果直接等同于 oasis7 工程治理结论。
  - 原始会话与过程日志属于 `raw evidence`，先进入 task-scoped `working_memory`；Codex/engineering task 的 phase 1 raw evidence 默认优先直读 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl`，若 `history.jsonl` 未命中则回退到 `~/.codex/sessions/**/rollout-*.jsonl`，只有被提炼过的结论才进入 `signal`、`task` 或长期 `memory`。
  - 反思链路统一为 `.codex/task execution log/evidence -> working_memory -> signal(reflection) -> owner review -> memory/task/rejected`，正式 PRD/project 仍由 owner 手工回写；wrapper 导出的 `output/.../<task_uid>.jsonl` 仅作为后续可替代输入，不是 phase 1 前置条件。
- Integration Points:
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
  - `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
  - `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
  - `doc/engineering/project.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/README.md`
  - `.pm/tasks/task_231ca618613d564ca2c9ec758253c7b7.execution.md`
  - `.pm/roles/*/memory/{active,superseded}.yaml`
  - `.pm/working_memory/*.yaml`（新增）
  - `.pm/inbox/signals.jsonl`
  - `scripts/pm/promote-signal.sh`
  - `scripts/pm/promote-memory.sh`
  - `scripts/pm/memory-report.sh`
  - `scripts/pm/workflow-report.sh`
  - `~/.codex/session_index.jsonl`
  - `~/.codex/history.jsonl`
  - `~/.codex/sessions/**/rollout-*.jsonl`（当 `history.jsonl` 无该会话消息时的 phase 1 fallback）
  - `~/.codex/logs_1.sqlite`（可选后续解析层，不是 phase 1 必需）
  - `https://github.com/memoryOSScom/memoryOSS`
  - `https://arxiv.org/abs/2512.12818`
- Edge Cases & Error Handling:
  - 误把未验证猜测写成 `fact`：lint 或 review 必须阻断，并要求降级为 `belief` 或 rejected。
  - `~/.codex/session_index.jsonl` / `history.jsonl` 缺失或不可用：允许回退到 `~/.codex/sessions/**/rollout-*.jsonl`；若仍不可用，则只依据 task execution log / 手工 evidence 写 working memory，不阻断任务执行。
  - `~/.codex/logs_1.sqlite` 无解析器或环境无 `sqlite3`：phase 1 继续使用 JSONL 来源，不阻断任务执行。
  - 同一 live session 的 transcript extraction 产生自读污染或审计争议：默认关闭隐式 auto-resolution；只有显式 `--session-id` 或显式 `--allow-auto-session` 才允许读取 `.codex` transcript。
  - `working_memory` 长时间未清理：任务关闭时必须转 `promoted/discarded/expired`，不得无限期留在 active。
  - transcript 含敏感信息：只允许抽取脱敏摘要与 source ref，不得原样复制进 `working_memory`/memory。
  - `belief` 长时间无人复核：report 标记 `needs_review`，producer review 视图必须显式暴露。
  - Recall profile 超预算：输出必须说明是因 `max_items` / `budget_chars` / `freshness_days` 被裁剪，不能静默吞掉。
  - 同一 topic 同时存在互斥 `fact` 与 `belief`：lint 直接失败，要求 supersede 或改 topic。
  - 同一 transcript 被重复抽取：若 `task_uid + source_ref + summary hash` 已存在 active `working_memory`，默认复用旧条目而不是重复创建。
  - 反思信号重复：若同一 `source_ref + candidate_topic + summary hash` 已存在未闭环记录，默认复用旧 signal/task，而不是再次创建。
  - 外部方案升级/失效：若 `memoryOSS` 或论文后续版本与现有 adopted 结论冲突，应新增 review task，旧结论走 superseded，不原地改写历史。
  - 网络不可用：外部资料只作为专题决策输入，仓库运行态不依赖在线访问；离线时不得阻断既有 `.pm` 工作流。
- Non-Functional Requirements:
  - NFR-MIR-1: Recall profile 在单次 `workflow-report` 运行内完成筛选，不引入额外网络依赖。
  - NFR-MIR-2: `belief` 类 active memory 的 `review_due_at` 填写率 100%。
  - NFR-MIR-3: 预算化 recall 的超预算截断必须可解释，静默丢弃率为 0。
  - NFR-MIR-4: 新 memory kind 与 recall profile 的引入不得破坏现有 role-agnostic 扩容能力。
  - NFR-MIR-5: 反思产物进入正式 memory/task 前，owner review 覆盖率 100%。
  - NFR-MIR-6: 外部借鉴结论必须有 `source_ref` 和 adopted/rejected rationale，缺失率为 0。
  - NFR-MIR-7: `working_memory` 活跃条目在任务关闭时的未治理残留率为 0。
  - NFR-MIR-8: transcript -> `working_memory` 的抽取必须带 source refs；来自 `~/.codex` 的抽取需至少能回指 `session_id` 与原始文件来源，缺失率为 0。
- Security & Privacy:
  - 外部方案借鉴不改变现有最小授权原则；agent 不得因“自我进化”而自动获得更高写权限。
  - working_memory / reflection / memory 只保存脱敏摘要与可追溯 source ref，不复制密钥、会话凭据或敏感原文。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 建立本专题 `prd/design/project`，冻结 adopted/rejected/deferred 边界与对象映射。
  - v1.1: 扩展 memory schema，增加 `memory_kind`、`review_due_at`、`recall_priority` 等字段及 lint/report。
  - v2.0: 为 `workflow-report` / `memory-report` 增加 recall profile 与预算化输出。
  - v2.1: 建立 `working_memory` 与会话分析契约，定义 `显式 session 选择/显式 opt-in auto-resolution + ~/.codex/session_index.jsonl + history.jsonl (+ sessions rollout fallback) -> working_memory -> reflection signal` 的 canonical 路径，并为后续 wrapper artifact 留出替换位。
  - v3.0: 建立 recall/working_memory/reflection smoke 与质量评估基线，验证噪声率、复发率、stale belief 与 working_memory 残留指标。
- Technical Risks:
  - 风险-1: 记忆分类过细但没有预算约束，会把 `workflow-report` 重新做成“第二份全文索引”。
  - 风险-2: 把 `belief` 误当正式事实，会放大错误判断并污染阶段评审。
  - 风险-3: 反思输入若缺少去重和 owner review，会把噪声重新包装成“学习能力”。
  - 风险-4: 过早引入外部 memory 产品，可能破坏 Git/worktree 审计链与离线自治能力。
  - 风险-5: 若没有 `working_memory` 过期与清理规则，会把任务内临时判断演变成第二份无法收口的日记层。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-MIR-001 | TASK-ENGINEERING-086/087 | `test_tier_required` | 专题三件套建档、schema 字段与 mapping 文档校验 | `self-evolution` 文档边界、memory 对象模型 |
| PRD-ENGINEERING-MIR-002 | TASK-ENGINEERING-087/088 | `test_tier_required` + `test_tier_full` | recall profile schema、`workflow-report` 预算化输出与截断说明回归 | 角色工作流、memory 注入质量 |
| PRD-ENGINEERING-MIR-003 | TASK-ENGINEERING-089/090 | `test_tier_required` + `test_tier_full` | `working_memory -> reflection signal` 契约、去重规则、promote-memory/task 回归 | QA failure signature 回流、重复问题复盘 |
| PRD-ENGINEERING-MIR-004 | TASK-ENGINEERING-086/090 | `test_tier_required` | adopted/rejected/deferred 决策记录、source refs 与 review 口径校验 | 外部方案评估、治理决策可回放性 |
| PRD-ENGINEERING-MIR-005 | TASK-ENGINEERING-087/088/090 | `test_tier_required` + `test_tier_full` | 新 role + memory kind 扩容 smoke、旧数据兼容与 lint/report 回归 | 角色扩容、长期 schema 兼容性 |
| PRD-ENGINEERING-MIR-006 | TASK-ENGINEERING-089/090/091 | `test_tier_required` + `test_tier_full` | transcript 提炼、task-scoped `working_memory` 生命周期、close-phase 清理与扩容回归 | 任务内过程记忆、handoff 连续性、临时认知治理 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-MIR-001 | 借鉴外部方案的对象模型与工程习惯，但继续以 `.pm/` + `doc/` 为真值 | 直接接入 `memoryOSS` 或其他外部 memory 产品为运行态真值 | oasis7 当前最重要的是可审计、可离线、可 worktree 隔离的治理链，而不是产品化 memory 服务。 |
| DEC-MIR-002 | 采用 `fact/experience/summary/belief` 四类记忆作为补强方向 | 继续只保留单一 `summary` 语义 | 单一 summary 不足以表达已证事实、经验模式、综合摘要与暂定判断的不同治理要求。 |
| DEC-MIR-003 | 反思结果先走 signal/owner review，再提升为 memory/task | 允许 agent 把 reflection 直接写入正式 memory 或 PRD | 直接写真值会绕过 owner、QA 与 stage 审计链，风险过高。 |
| DEC-MIR-004 | 召回必须预算化并按 phase/role/kind 控制 | 允许 agent 自由检索并全量注入历史记忆 | 无预算的长上下文会放大噪声和相互矛盾记忆，不符合 oasis7 的 deterministic governance 目标。 |
| DEC-MIR-005 | 对 `belief` 施加 review_due_at 与 superseded 约束 | 把 `belief` 与 `fact` 一视同仁长期保留 active | 暂定判断本质上是待验证假设，必须更快过期和复核。 |
| DEC-MIR-006 | 将会话/过程记忆先落到 task-scoped `working_memory` | 直接把 transcript 或中间推理写入长期 memory | 过程认知变化快、噪声高，且主要服务于当前任务，不适合直接变成长期真值。 |
| DEC-MIR-007 | Codex/engineering task 的 phase 1 允许读取 `~/.codex/session_index.jsonl` 与 `~/.codex/history.jsonl`，若 `history.jsonl` 无该会话消息则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，但默认必须显式指定 `session_id`；只有显式 `--allow-auto-session` 才允许 registry/worktree pattern 自动解析 | 继续把“读取当前/最近 live session”当成默认收口路径；或先要求 wrapper 导出 `output/.../<task_uid>.jsonl` 作为唯一 transcript 来源 | 当前环境已存在可直接读取的本地 Codex 会话索引与 rollout 存档，所以可以继续复用现成 raw evidence；但同一 live session 的隐式自读不够优雅且容易引入自污染/审计歧义，因此改为显式 opt-in。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 是否明确说明本期解决什么问题：已明确当前缺的是“外部记忆/反思方案借鉴边界”，而不是再造一份 memory 功能说明。
  - ✔ 是否定义成功指标（可量化）：SC-1~SC-7 与 NFR-MIR-1~8 已量化字段完整率、预算化约束、working_memory 清理与 review 覆盖率。
  - ✔ 是否与公司/项目阶段目标一致：与现有 `self-evolution` / 长期 memory 运行层补强一致。
  - ✔ 是否说明优先级来源：来自外部方案评估输入与后续 recall/reflection 扩展需求。
- 用户与场景（Who / When）:
  - ✔ 是否明确目标用户是谁：producer、agent、QA、liveops、治理维护者均已定义。
  - ✔ 是否区分主用户与边缘用户：producer / governance 为主，agent/QA/liveops 为直接协作者。
  - ✔ 是否定义使用场景：外部方案评估、工作流补强、重复失败复盘、阶段复核均已定义。
  - ✔ 是否说明频率与关键路径：User Scenarios & Frequency 与 Critical User Flows 已覆盖。
- 范围定义（Scope Control）:
  - ✔ 是否列出本期功能清单：记忆分类、recall profile、working memory、reflection signal、belief review、external inspiration matrix 均已列出。
  - ✔ 是否明确 Out of Scope：已排除外部 SaaS 真值、embedding/图数据库、自动自治执行。
  - ✔ 是否避免隐性功能：矩阵中显式写清字段、动作、状态与权限。
  - ✔ 是否有版本拆分说明：MVP -> v1.1 -> v2.0 -> v2.1 -> v3.0 已给出。
- 功能规格（What）:
  - ✔ 每个功能是否描述完整：规格矩阵覆盖字段、动作、状态、排序与权限。
  - ✔ 是否有交互流程说明：Critical User Flows 已覆盖。
  - ✔ 是否明确字段定义：各功能点字段已列出。
  - ✔ 是否描述所有按钮行为：本专题无 UI，已转化为脚本/动作行为说明。
  - ✔ 是否定义状态变化逻辑：memory、profile、signal、decision 均有状态变化。
  - ✔ 是否描述排序规则 / 计算规则：记忆优先级、预算化截断与 freshness 已定义。
  - ✔ 是否明确权限控制逻辑：owner、producer、治理维护者边界已定义。
- 异常与边界（Edge Cases）:
  - ✔ 网络异常如何处理：外部资料只作决策输入，运行态不依赖在线访问。
  - ✔ 空数据如何展示：Recall profile 无命中时由 workflow/report 输出空视图而非隐式失败。
  - ✔ 权限不足如何反馈：shared memory、formal decision 由 producer/治理维护者收口。
  - ✔ 接口超时如何处理：NFR-MIR-1 要求在单次本地 report 内完成，不引入额外网络超时依赖。
  - ✔ 并发冲突如何处理：重复 reflection、互斥 fact/belief 和 supersede 场景已覆盖。
  - ✔ 数据异常如何兜底：误分类、缺 review、超预算静默丢弃均有阻断/提示策略。
- 非功能需求（NFR）:
  - ✔ 是否定义性能要求：NFR-MIR-1。
  - ✔ 是否定义兼容性要求：NFR-MIR-4、NFR-MIR-6。
  - ✔ 是否定义安全要求：Security & Privacy 已覆盖。
  - ✔ 是否定义数据规模预期：通过 recall budget、freshness 与扩容兼容约束定义。
  - ✔ 是否定义可扩展性约束：新 role / 新 kind 不得破坏 role-agnostic schema。
- 可测试性（Testability）:
  - ✔ 是否定义验收标准：AC-1~AC-6 已给出。
  - ✔ 是否定义完成标准：SC、AC 与 traceability 表共同定义。
  - ✔ 是否定义数据验证方式：lint/report/smoke/决策记录检查已定义。
  - ✔ 是否定义回归影响范围：Traceability 表已覆盖。
- 逻辑一致性（Consistency）:
  - ✔ 是否存在逻辑冲突：未发现；本专题显式继承 `.pm` 真值边界。
  - ✔ 是否存在目标与设计不匹配：目标直接映射到 memory kind、recall、reflection 三类补强。
  - ✔ 是否存在自相矛盾：未发现。
  - ✔ 是否与历史版本冲突：明确保持与 `self-evolution` 总专题、长期 memory 子专题兼容。
- 依赖与影响分析（Impact）:
  - ✔ 是否明确依赖系统：`doc/**`、`.pm/**`、`scripts/pm/*.sh` 与外部参考已列出。
  - ✔ 是否明确接口依赖：Integration Points 已覆盖。
  - ✔ 是否评估影响模块：producer/agent/QA/liveops 与工作流入口均已覆盖。
  - ✔ 是否评估数据迁移：通过 schema 扩展而非重置存储，兼容旧数据。
  - ✔ 是否识别上线风险：技术风险已覆盖。
- 决策透明度（Decision Record）:
  - ✔ 是否说明方案选择原因：Decision Log 已说明。
  - ✔ 是否记录被否决方案：外部真值、无预算 recall、直接写真值均已否决。
  - ✔ 是否有数据支持：以现有 `self-evolution` 边界、外部资料结构与治理约束为依据。
- 文档树一致性与结构约束（Documentation Architecture）:
  - ✔ 本 PRD 是否明确归属于某个模块目录：`doc/engineering/self-evolution/`。
  - ✔ 是否符合文档树层级规范：属于 engineering/self-evolution 子专题。
  - ✔ 是否重复定义已有模型：未重复改写原有 `.pm` 真值，只在其上定义增量补强。
  - ✔ 是否引用已有定义，而不是重写：已引用 `self-evolution` 总专题与长期 memory 子专题。
  - ✔ 是否清晰标注跨模块依赖：已列明 `scripts/pm`、`.pm` 与外部参考边界。
  - ✔ 是否混合错误抽象层级：未把实现细节混入模块级 Why/What/Done 范围之外。
  - ✔ 是否具备依赖可追溯性：已补齐 PRD/project/design/task execution log/engineering 根入口互链。
- 总体 Gate 结果: 🟢 Ready
