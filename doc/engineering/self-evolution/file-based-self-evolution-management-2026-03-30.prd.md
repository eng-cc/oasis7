# oasis7：自我进化文件化项目管理（2026-03-30）

- 对应设计文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`

审计轮次: 6

- 对应标准执行入口: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`

## 1. Executive Summary
- Problem Statement: 当前仓库已经具备 `PRD / project / devlog / handoff / worktree` 等治理部件，但它们主要解决“正式规格”和“单次执行”问题，尚未形成可持续运行的角色长期 memory/backlog 系统。7 个标准角色的跨天状态、阶段判断、候选任务池与真实反馈回流仍依赖人工阅读当天 `devlog` 和散落文档，无法支撑项目自我进化。
- Proposed Solution: 在仓库内新增一套基于文件、可审计、可在 worktree 中独立演化的项目管理层，作为 `engineering/self-evolution` 专题长期治理对象。该层以 Git 为存储、以 `.pm/` 为运行态目录，统一承载角色 memory、角色 backlog、signal inbox、task registry、stage gate 与自动化脚本，并与既有 `doc/` 正式文档体系保持分工。
- Success Criteria:
  - SC-1: 首批 7 个标准角色全部具备独立长期 memory namespace 和 backlog 容器，且角色扩容时无需修改历史文件结构。
  - SC-2: 进入长期 memory 的记录 100% 带有 `source_refs`、`effective_at`、`last_reviewed_at` 和 `status(active/superseded)` 字段，不再直接把 `devlog` 条目当最终真值。
  - SC-3: 候选任务 100% 通过 task registry 文件建档，具备 `owner_role`、`status`、`priority`、`source_signal`、`related_prd`、`acceptance` 字段，并可在 1 次脚本扫描中枚举。
  - SC-4: `qa_engineer` 与 `liveops_community` 的信号进入 signal inbox 后，能够在 1 个工作日内被提升为 `candidate` task 或显式标记 `discarded/deferred`，不得长期停留在非正式口头状态。
  - SC-5: 当前阶段判断、claim envelope、关键 gate lane 状态可以从文件化 stage/gate 层一键汇总，制作人不再依赖手工跨文档拼装阶段评审输入。
  - SC-6: 文件化项目管理层与现有 `PRD / project / devlog` 的职责边界清晰，`devlog` 继续是原始事件流，正式规格继续留在 `doc/`，运行态数据留在 `.pm/`，重复定义率为 0。
  - SC-7: 每个标准角色在“开始任务 / 收口任务 / 阶段评审”三个场景下都有统一 `workflow-report` 入口与固定 checklist，不再依赖人工拼接 `role-report`、`memory-report`、`stage-report` 与 signal inbox 状态。
  - SC-8: `workflow-report --phase close` 的 checklist 必须明确要求“commit 前启动独立 subagent review 当前 diff，并先处理 findings 再提交”，不得只在人工约定层存在。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要跨天掌握阶段、规则口径、候选改进项与多角色回流，不希望每次靠全文检索 `devlog` 重新拼装上下文。
  - `qa_engineer`：需要把失败签名、阻断建议、真实反馈稳定提升为长期记忆和候选任务，而不是停留在当日执行记录。
  - `liveops_community`：需要把社区信号、事故摘要、对外口径与 follow-up 维护为可回流、可分桶、可追踪的运行态资产。
  - `runtime_engineer` / `viewer_engineer` / `agent_engineer` / `wasm_platform_engineer`：需要可审计的长期任务池、可 supersede 的历史结论，以及不依赖外部 SaaS 的本地协作层。
  - 仓库治理维护者：需要一套符合当前 Git/worktree/CLI 约束的项目管理结构，而不是再引入第二真值系统。
- User Scenarios & Frequency:
  - 每日执行收口：每个活跃角色每天至少 1 次，把高价值信号从 `devlog` / 证据 / runbook 提升到 `.pm/`。
  - 候选任务评审：每周至少 1 次，owner 从 `candidate` 池里升格、阻断或延后任务。
  - 阶段评审：每个 release / phase 决策点至少 1 次，从 `stage/gate` 文件直接读取输入。
  - 真实反馈回流：每次出现 QA block、社区高频反馈、线上事故或重大裁决变化时立即执行。
  - 角色扩容：新增标准角色或调整职责边界时按模板增配 memory/backlog 容器，不改历史记录结构。
- User Stories:
  - PRD-ENGINEERING-SE-001: As a `producer_system_designer`, I want a file-native stage and role management layer, so that I can review project evolution without manually stitching scattered docs.
  - PRD-ENGINEERING-SE-002: As a `qa_engineer`, I want failure signatures and gate conclusions promoted into long-term memory and candidate tasks, so that quality signals survive beyond one-day logs.
  - PRD-ENGINEERING-SE-003: As a `liveops_community`, I want community/incident signals routed into a canonical inbox and backlog, so that external feedback becomes actionable engineering input.
  - PRD-ENGINEERING-SE-004: As an engineering owner, I want one task registry format for role backlog items, so that worktree-local execution and repo-wide traceability stay aligned.
  - PRD-ENGINEERING-SE-005: As a governance maintainer, I want role memory records to support `superseded` lifecycle and source references, so that evolving decisions remain auditable instead of overwritten.
  - PRD-ENGINEERING-SE-006: As a future role owner, I want the file layout to support role expansion without schema breakage, so that the system can evolve beyond the current 7 roles.
  - PRD-ENGINEERING-SE-007: As any active role owner, I want one canonical workflow entrypoint for `start/close/review`, so that `.pm` becomes the default operating loop instead of a set of optional low-level scripts.
- Critical User Flows:
  1. Flow-SE-001: `角色完成当日执行 -> 写入 doc/devlog/YYYY-MM-DD.md -> promoter 脚本抽取高价值信号 -> 进入 .pm/inbox/signals.jsonl -> owner 决定提升为 memory 或 candidate task`
  2. Flow-SE-002: `qa_engineer 发现 block / failure signature -> 生成 signal -> promote 到 qa backlog -> 若影响阶段或对外口径则同步 stage/gate -> producer 在阶段评审时直接读取`
  3. Flow-SE-003: `liveops_community 收到真实反馈 / 事故 -> 归档 signal -> 聚类后生成 candidate task 或 incident memory -> 相关 owner 接收并回写 follow-up`
  4. Flow-SE-004: `producer_system_designer 进行阶段评审 -> stage/gate 报表汇总 role backlog、关键 blocker、claim envelope 和 trend inputs -> 输出 continue / hold / reassess`
  5. Flow-SE-005: `历史结论被新结论取代 -> 原 memory 记录转为 superseded -> 新记录写入 active -> superseded_by / source_refs / effective range 形成链路`
  6. Flow-SE-006: `新增标准角色 -> 基于角色模板生成 memory/backlog 容器 -> 注册到 registry -> 既有脚本自动将其纳入 lint / report / stage aggregation`
  7. Flow-SE-007: `owner 进入新 worktree -> 执行 workflow-report --phase start --role <owner> -> 读取 backlog/memory/signal/stage 汇总 -> 开发完成后执行 workflow-report --phase close -> 回写 devlog + signal/memory/backlog -> commit 前启动独立 subagent review 当前 diff 并处理 findings -> producer/owner 在评审时执行 workflow-report --phase review`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 角色长期 memory | `id`、`role`、`topic`、`summary`、`source_refs[]`、`effective_at`、`last_reviewed_at`、`status`、`superseded_by` | `promote-signal` 生成新记录，`supersede-memory` 废止旧记录 | `draft -> active -> superseded/retired` | 默认按 `role/topic/effective_at desc`；active 优先于 superseded | 角色 owner 可新增/更新；producer 与治理维护者可联审关键跨角色结论 |
| 角色 backlog 条目 | `task_id`、`title`、`owner_role`、`status`、`priority`、`source_signal`、`related_prd[]`、`acceptance[]`、`handoff_to[]` | `new-task` 创建，`review-task` 升降级，`close-task` 完结 | `candidate -> committed -> blocked -> done/deferred` | 按 `priority`、`updated_at`、`stage_impact` 排序 | owner role 主责；producer 可调优先级；QA 可对 blocker 给阻断建议 |
| Signal inbox | `signal_id`、`source_type`、`source_ref`、`role_hint`、`severity`、`summary`、`promotion_state` | `ingest-signal` 录入，`promote-signal` 提升，`discard-signal` 放弃 | `new -> triaged -> promoted/discarded/deferred` | `severity` 高于时间；QA / liveops / gate signal 优先 | 全角色可提交；对应 owner 负责处置；治理维护者可审计 |
| Task registry | `task_id`、`owner_role`、`worktree_hint`、`status`、`source_refs[]`、`doc_refs[]`、`updated_at` | 统一扫描 `.pm/tasks/*.yaml`，生成索引与报告 | `missing -> registered -> active -> closed` | 任务 ID 单调递增；按 owner_role 分组 | 任务创建者负责建档；owner role 负责状态更新 |
| Stage / gate 汇总 | `stage_id`、`claim_envelope`、`lane_status[]`、`blocking_tasks[]`、`updated_from` | `stage-report` 读取 role backlog 和 gate 文件汇总 | `draft -> aligned -> adopted -> superseded` | 任一 blocking lane 优先显示；更新时间最近优先 | 仅 `producer_system_designer` 可修改正式阶段结论；各 owner 提供输入 |
| Workflow 汇总入口 | `phase`、`role`、`signal_counts`、`checklist[]`、`pending_signals[]` | `workflow-report` 聚合 `role-report`、`memory-report`、`stage-report` 与 signal inbox，输出 `start/close/review` 固定动作建议；producer 的 `review` 额外汇总全部角色 pending signals | `start -> close -> review` | 先按 `phase` 固定分桶，再按 `severity`、blocked task、needs_review memory 排序；已 `promoted/rejected/deferred` 的 signal 不再计为 pending | 全角色可读；对应 owner 负责执行 checklist；producer 额外负责阶段裁决相关步骤 |
| 角色 registry | `role_name`、`memory_path`、`backlog_path`、`is_active`、`introduced_at` | `register-role` 新增角色，lint 自动纳入 | `pending -> active -> retired` | 稳定按 role_name 排序 | 治理维护者维护；角色扩容需 producer 联审 |
| 自动化脚本 | `script_name`、`inputs`、`outputs`、`failure_signature` | 执行 `lint / report / promote / scaffold` | `available -> verified -> blocked` | required-tier 脚本先于 full-tier 扩展脚本 | 所有人可执行；治理维护者维护契约 |
- Acceptance Criteria:
  - AC-1: `engineering/self-evolution` 专题明确定义 `.pm/` 文件化项目管理层的对象边界、字段、状态机、权限和运行流程。
  - AC-2: `devlog`、`.pm/` 与 `doc/` 的职责边界写清，禁止把 `devlog` 直接当长期 memory，禁止在 `.pm/` 中重写正式 PRD 规格。
  - AC-3: 首期目标态覆盖 7 个标准角色，并显式支持后续角色扩容，不要求为新增角色重构现有目录和脚本输入格式。
  - AC-4: Stage/gate 汇总路径明确可审计，制作人可以从文件层读取阶段评审输入，而不是手工拼接多份文档。
  - AC-5: 任务 registry、signal inbox、memory 记录全部具备 machine-readable 格式，并能在 repo 内被脚本枚举和 lint。
  - AC-6: 专题 project 文档给出分阶段实施计划，且至少将 `.pm` 目录脚手架、signal promotion、role backlog、stage report、QA gate 五条实施线拆成独立任务。
  - AC-7: topic 文档、engineering 根入口、索引和 devlog 全部完成互链，进入正式治理链。
  - AC-8: `AGENTS.md`、角色职责卡与 `new-task-worktree` 提示明确要求在任务开始/收口/评审时执行 `workflow-report`，且 required/full smoke 会覆盖该入口。
  - AC-9: `workflow-report --phase close`、根 `AGENTS.md` 与工程主项目口径一致要求 commit 前启动独立 subagent review，且 required-tier smoke 会断言该 checklist 项存在。
- Non-Goals:
  - 不引入 OpenProject、Mem0、Graphiti、Supabase 或外部 SaaS 作为首期真值系统。
  - 不要求首期自动修改 `doc/**/prd.md` 或 `doc/**/project.md`；正式规格仍由 owner 审核回写。
  - 不把 `.pm/` 设计成对外产品功能或玩家面系统。
  - 不让 agent 绕过现有 worktree / owner role / QA / LiveOps 协作规则直接自治执行高风险改动。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - Git/worktree 感知脚本，用于在当前 worktree 内独立维护 `.pm/` 运行态。
  - 文件读写与 lint 脚本，用于检查 YAML/JSONL/Markdown 结构完整性。
  - signal promotion 脚本，用于把 `devlog`、证据文件或手工录入事件提升为 memory/task。
  - role report 脚本，用于按角色读取 backlog、active memory、needs_review 和 superseded 视图。
  - stage report 脚本，用于按角色 backlog、gate、signal 汇总阶段输入。
  - workflow report 脚本，用于给 owner 提供 `start/close/review` 固定工作流视图和 checklist。
- Evaluation Strategy:
  - 结构正确性：lint 通过率、字段完整率、角色 registry 覆盖率。
  - 运行正确性：signal 从录入到 `promoted/discarded` 的流转时延、orphan task 数量、dangling source ref 数量。
  - 组织有效性：阶段评审准备时长是否下降，QA/liveops 信号是否能稳定进入 backlog，角色长期记忆是否减少重复阅读成本。

## 4. Technical Specifications
- Architecture Overview:
  - 正式文档层：`doc/**`，继续保存 PRD / design / project / devlog / runbook / evidence。
  - 运行态项目管理层：仓库根目录 `.pm/`，保存 role memory、role backlog、task registry、signal inbox、stage/gate 和模板。
  - 自动化层：`scripts/pm/*.sh`，提供 scaffold、lint、promote、report 等入口。
  - 回流层：`.pm/` 中的结论再经 owner 审核，必要时回写 `doc/**/project.md`、`prd.md` 和 `devlog`。
- Integration Points:
  - `AGENTS.md`
  - `.agents/roles/*.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/devlog/YYYY-MM-DD.md`
  - `testing-manual.md`
  - `doc/playability_test_result/*`
  - `doc/readme/*`
  - 仓库根目录 `.pm/`（新增）
  - `scripts/pm/*.sh`（新增）
- Edge Cases & Error Handling:
  - 角色扩容：新增角色时必须通过角色 registry 注册；未注册角色不得直接写入 `.pm/roles/<role>/` 并宣称有效。
  - 角色退役：角色停用时 `is_active=false`，保留历史 memory/backlog，不删除旧路径。
  - 同一信号多次提升：必须通过 `signal_id` 去重；重复提升时返回已存在 task/memory 引用。
  - 历史结论被推翻：旧记录改为 `superseded`，不得原地覆盖并丢失来源。
  - worktree 并发编辑：若不同 worktree 同时修改同一 `.pm` 对象，以 landing 后的主干重放 lint/report 为准；必要时通过单任务单文件拆分减少冲突。
  - orphan task：若 task 缺失 owner_role、source_ref 或 acceptance，lint 直接失败，不允许进入 `committed`。
  - dangling memory：若 memory 记录引用不存在的文档/信号，lint 直接失败。
  - stage drift：若 stage 文件未包含当前 blocking task 或 claim envelope 与正式对外口径冲突，report 必须标红并拒绝输出 `aligned`。
  - devlog promotion 漏提：若高严重度 QA/liveops signal 在 SLA 内未被处理，report 需显式列出 overdue。
  - `.pm/` 与 `doc/` 冲突：当 `.pm/` 中的建议和正式 PRD/project 冲突时，以正式文档为准，并将 `.pm/` 记录为待裁决而非自动覆盖。
- Non-Functional Requirements:
  - NFR-SE-1: `.pm/` 全量 lint 单次执行时间 <= 20 秒。
  - NFR-SE-2: task registry、role registry、stage/gate、signal inbox 的 machine-readable 结构完整率 100%。
  - NFR-SE-3: active memory 记录 100% 带 `source_refs` 和 `effective_at`，superseded 记录 100% 带 `superseded_by`。
  - NFR-SE-4: 新 signal 从录入到 `promoted/discarded/deferred` 的 P95 时延 <= 1 个工作日。
  - NFR-SE-5: `qa_engineer` 与 `liveops_community` 的高严重度信号 backlog 化覆盖率 100%。
  - NFR-SE-6: 新增角色接入后，既有 lint/report 脚本无需修改历史数据格式即可纳入。
  - NFR-SE-7: `.pm/` 中任何对象都不得要求访问网络或外部托管服务才能成为真值。
  - NFR-SE-8: 正式阶段结论与 `.pm/stage/*.yaml` 的一致率 100%，不得出现“文件汇总 pass，但正式口径仍未知”的漂移状态。
  - NFR-SE-9: 角色 backlog 的 `candidate/committed/blocked/done/deferred` 状态定义在所有角色间一致率 100%。
  - NFR-SE-10: `.pm/` 与 `doc/` 之间的引用可达性覆盖率 100%。
- Security & Privacy:
  - `.pm/` 仅保存工程治理元信息，不存储凭据、密钥或第三方平台 cookies。
  - 任何自动提炼脚本都不得把敏感数据从 runbook / incident 原文复制进 `.pm/`。
  - Stage/gate 文件中若涉及对外口径，必须遵守 `README` 与正式 PRD 的禁语边界。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 建立 `engineering/self-evolution` 专题三件套、`.pm/` 目录骨架、role registry 和 task registry 模板。
  - v1.1: 打通 `signal inbox -> candidate task` 基础链路，优先覆盖 `qa_engineer` 和 `liveops_community`。
  - v2.0: 完成 7 个标准角色的长期 memory/backlog 收口，并交付 stage/gate 汇总脚本。
  - v2.1: 建立 `devlog -> signal -> memory/task -> doc backflow` 的固定操作规约与 required-tier lint。
  - v2.2: 建立 `workflow-report` 统一入口，并接入 `AGENTS.md`、角色职责卡、`new-task-worktree.sh`、commit 前 subagent review 规则与 smoke，使 `.pm` 成为默认执行链路。
  - v3.0: 在角色扩容、阶段评审和多 worktree 并行场景下稳定运行，形成仓库级自我进化操作层。
- Technical Risks:
  - 风险-1: `.pm/` 与 `doc/` 双层体系若分工不清，会产生第二真值和重复维护。
  - 风险-2: 过早追求自动化，可能让错误 signal 进入长期 memory/backlog，反而放大噪声。
  - 风险-3: 若对象建模过粗，角色 backlog 会退化为另一份 `devlog`；若过细，又会造成编辑负担过高。
  - 风险-4: 多 worktree 并发下，若没有单任务单文件原则和 lint/merge 规约，冲突会迅速增多。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-SE-001 | TASK-ENGINEERING-074/078 | `test_tier_required` | stage/gate 文件结构检查、stage 汇总样例验证 | 制作人阶段评审输入、跨角色裁决链 |
| PRD-ENGINEERING-SE-002 | TASK-ENGINEERING-076/079 | `test_tier_required` | QA signal ingestion / promotion / overdue 报表验证 | QA block 回流、required/full 放行链 |
| PRD-ENGINEERING-SE-003 | TASK-ENGINEERING-076/079 | `test_tier_required` | liveops signal inbox、candidate task 生成与 follow-up 链检查 | 社区反馈回流、事故收口 |
| PRD-ENGINEERING-SE-004 | TASK-ENGINEERING-075/077/084 | `test_tier_required` | task registry 模板、状态机、lint、索引生成与 `role-report` backlog 视图验证 | worktree 任务追踪、角色 backlog |
| PRD-ENGINEERING-SE-005 | TASK-ENGINEERING-075/077/084 | `test_tier_required` | memory active/superseded 生命周期、source ref 可达性、superseded_by 链与 `role-report` memory 视图检查 | 长期记忆审计与历史裁决回放 |
| PRD-ENGINEERING-SE-006 | TASK-ENGINEERING-075/079/084 | `test_tier_required` + `test_tier_full` | 新角色注册、模板脚手架、全量 report/lint/role-report 扩容验证 | 角色扩容、治理脚本兼容性 |
| PRD-ENGINEERING-SE-007 | TASK-ENGINEERING-085 | `test_tier_required` + `test_tier_full` | `workflow-report` start/close/review 视图、close checklist 中的 subagent review 要求、signal 汇总、`new-task-worktree` 提示和角色扩容场景验证 | 日常开发工作流、角色收口动作、阶段评审入口 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SE-001 | 在仓库内建立文件化项目管理层 `.pm/` | 直接接入外部 PM/SaaS 作为真值 | 当前仓库已有 Git/worktree/文档治理体系，先在本地闭环成本更低、审计一致性更强。 |
| DEC-SE-002 | `devlog` 保持原始事件流，长期 memory/backlog 下沉到 `.pm/` | 继续把 `devlog` 同时当日志和长期记忆 | `devlog` 适合时间线回放，不适合长期语义整理和候选任务治理。 |
| DEC-SE-003 | 采用 role memory + role backlog + signal inbox + task registry + stage/gate 五对象模型 | 仅做一份共享任务清单 | 自我进化需要同时解决长期记忆、候选任务、阶段判断和真实反馈回流，单一清单不够。 |
| DEC-SE-004 | memory 支持 `superseded` 生命周期和 source refs | 直接覆盖旧结论 | 角色判断和阶段口径会演化，必须保留历史链路。 |
| DEC-SE-005 | 首期禁止外部网络依赖，脚本与真值全部在仓库内运行 | 依赖远程数据库、消息队列或托管服务 | 当前目标是仓库内最强自治，不是平台集成展示。 |
| DEC-SE-006 | 用 `workflow-report` 把 `role/memory/stage/signal` 四类视图收敛成统一 workflow 入口 | 继续要求 owner 手工组合多个低层 report / promote 命令 | 基础脚本已经齐全，真正缺的是默认操作入口；若不收敛入口，`.pm` 仍会停留在“可用但不默认使用”。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 是否明确说明本期解决什么问题：第 1 章说明了现有 `PRD/project/devlog` 只能覆盖正式规格和单次执行，尚缺长期 memory/backlog 层。
  - ✔ 是否定义成功指标（可量化）：SC-1~SC-7、NFR-SE-1~10 给出角色覆盖、字段完整率、时延和一致率指标。
  - ✔ 是否与公司/项目阶段目标一致：与当前项目“自我进化”和角色协作治理方向一致。
  - ✔ 是否说明优先级来源：来自当前 7 角色长期状态缺失、阶段评审输入手工拼装和 QA/liveops 信号难沉淀的问题。
- 用户与场景（Who / When）:
  - ✔ 是否明确目标用户是谁：7 个标准角色和治理维护者均已定义。
  - ✔ 是否区分主用户与边缘用户：producer/QA/liveops 为首期主用户，其他工程角色和未来新增角色为后续扩展用户。
  - ✔ 是否定义使用场景：每日执行收口、阶段评审、真实反馈回流、角色扩容均已定义。
  - ✔ 是否说明频率与关键路径：User Scenarios & Frequency 与 Critical User Flows 已明确。
- 范围定义（Scope Control）:
  - ✔ 是否列出本期功能清单：role memory、role backlog、signal inbox、task registry、stage/gate、workflow-report、role registry、自动化脚本均已列出。
  - ✔ 是否明确 Out of Scope：Non-Goals 已排除外部 SaaS 真值、自动改正式 PRD、对外产品化等范围。
  - ✔ 是否避免隐性功能：功能矩阵对字段、行为、状态和权限做了显式定义。
  - ✔ 是否有版本拆分说明：第 5 章给出 MVP -> v1.1 -> v2.0 -> v2.1 -> v3.0。
- 功能规格（What）:
  - ✔ 每个功能是否描述完整：功能矩阵逐项说明。
  - ✔ 是否有交互流程说明：Critical User Flows 已覆盖。
  - ✔ 是否明确字段定义：各对象的关键字段均已列出。
  - ✔ 是否描述所有按钮行为：本专题无 UI 按钮，已改为脚本/动作行为说明。
  - ✔ 是否定义状态变化逻辑：memory、task、signal、stage 等状态机均已定义。
  - ✔ 是否描述排序规则 / 计算规则：优先级、severity、blocking lane 等排序规则已说明。
  - ✔ 是否明确权限控制逻辑：各角色 owner、producer、治理维护者的权限边界已写明。
- 异常与边界（Edge Cases）:
  - ✔ 网络异常如何处理：本专题首期禁止网络依赖，已转化为本地文件和 worktree 并发边界处理。
  - ✔ 空数据如何展示：通过 lint/report 的 empty/overdue/orphan 状态定义处理。
  - ✔ 权限不足如何反馈：未注册角色、缺 owner、正式结论越权均已定义失败行为。
  - ✔ 接口超时如何处理：NFR-SE-1 定义 lint 执行时长预算。
  - ✔ 并发冲突如何处理：worktree 并发编辑和单任务单文件原则已说明。
  - ✔ 数据异常如何兜底：orphan task、dangling memory、stage drift 均已覆盖。
- 非功能需求（NFR）:
  - ✔ 是否定义性能要求：NFR-SE-1。
  - ✔ 是否定义兼容性要求：NFR-SE-6、NFR-SE-7。
  - ✔ 是否定义安全要求：Security & Privacy 已覆盖。
  - ✔ 是否定义数据规模预期：首批 7 角色 + 后续扩容的治理约束已定义。
  - ✔ 是否定义可扩展性约束：新增角色无需改历史结构和真值层。
- 可测试性（Testability）:
  - ✔ 是否定义验收标准：AC-1~AC-7。
  - ✔ 是否定义完成标准：SC、AC、专题 project 任务和 traceability 表共同构成 done。
  - ✔ 是否定义数据验证方式：lint、report、promotion、registry 结构检查均已定义。
  - ✔ 是否定义回归影响范围：Traceability 表已列出。
- 逻辑一致性（Consistency）:
  - ✔ 是否存在逻辑冲突：未发现明显冲突；`.pm/` 运行态与 `doc/` 正式文档的边界已区分。
  - ✔ 是否存在目标与设计不匹配：目标直接映射到五类核心对象与脚本层。
  - ✔ 是否存在自相矛盾：未发现。
  - ✔ 是否与历史版本冲突：与现有 `PRD/project/devlog` 形成补充分层，而不是替代。
- 依赖与影响分析（Impact）:
  - ✔ 是否明确依赖系统：`AGENTS.md`、角色卡、engineering 主文档、`testing-manual.md`、`.pm/` 和 `scripts/pm` 均已列出。
  - ✔ 是否明确接口依赖：Integration Points 已覆盖。
  - ✔ 是否评估影响模块：producer/QA/liveops/工程角色和 stage gate 均已覆盖。
  - ✔ 是否评估数据迁移：已说明从 `devlog` 到 signal/memory/task 的提升链路。
  - ✔ 是否识别上线风险：第 5 章技术风险已列出。
- 决策透明度（Decision Record）:
  - ✔ 是否说明方案选择原因：DEC-SE-001~005。
  - ✔ 是否记录被否决方案：外部 SaaS 真值、直接覆盖旧结论、单一共享清单等已列为否决方案。
  - ✔ 是否有数据支持：以当前 7 角色、多 worktree、阶段评审和 QA/liveops 回流痛点为证据。
- 文档树一致性与结构约束（Documentation Architecture）:
  - ✔ 本 PRD 是否明确归属于某个模块目录：`doc/engineering/self-evolution/`。
  - ✔ 是否符合文档树层级规范：按 `*.prd.md / *.design.md / *.project.md` 三件套建档。
  - ✔ 是否重复定义已有模型：未重写正式玩法/运行时规则，只定义项目管理运行层。
  - ✔ 是否清晰标注跨模块依赖：Integration Points 已列出。
  - ✔ 是否遵守抽象层级：本文聚焦 Why/What/Done，实施细节下沉到 design/project。
  - ✔ 是否保证依赖可追溯性：Traceability、root engineering 追踪、devlog 和入口索引均已定义。
- 总体 Gate 结果: 🟢 Ready
