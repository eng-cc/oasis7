# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）

- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`

审计轮次: 1

- 对应标准执行入口: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`

## 1. Executive Summary
- Problem Statement: 外部 agent workflow repo 已提供成套的 planning、TDD、subagent、visual companion 与 closeout 语义；若 oasis7 不把哪些模式可变成默认、哪些仍必须拒绝写成正式规则冻结下来，默认编排、owner/task 真值和 review 边界会继续漂移。
- Proposed Solution: 保留 adopted / rejected / deferred 矩阵，只把与当前主链兼容的部分翻译成 repo-owned surface：default workflow bootstrap、bounded brainstorming、默认角色 subagent 编排、bounded subagent-driven execution、behavior-first testing、workflow router，以及 planning / execution / skill-authoring surface。整体继续维持单 owner role、单 `.pm` task、单 canonical worktree 与 GitHub PR review 主链；外部 bootstrap、第二套计划真值、universal ceremony 与 packaging-before-governance 继续拒绝或 deferred。
- Success Criteria:
  - SC-1: `superpowers` 首批借鉴项 100% 进入 `adopted / rejected / deferred` 三态矩阵，并为每项给出理由与 repo-owned target object。
  - SC-2: 每个 adopted 项都必须映射到一个 repo-owned follow-up task 或明确的模块参考入口，不允许停留在“聊天建议”层。
  - SC-3: `dispatching-parallel-agents` 与 `subagent-driven-development` 必须被翻译成 repo-owned 默认角色编排层与 bounded subagent-driven execution：`tpm` 负责 orchestrate，其他专业角色默认作为 subagent 参与分析 / 实现 / 验证 / 补充 review，但不得打破单 owner role、单 `.pm` task、单 canonical worktree 与 GitHub PR review 主链。
  - SC-4: Viewer Web 视觉/结构类专题必须明确：browser-based visual companion 仅是前置设计手段，不替代 `agent-browser` 回归、repo-owned UI regression 或正式实现 task。
  - SC-5: 外部 workflow 借鉴不得引入新的运行态真值系统；默认 subagent 编排只能叠加在现有 `.pm`、`project.md`、task execution log 与 GitHub PR review 之上。
  - SC-6: 从 `writing-plans` salvage 的 planning discipline 必须被翻译成 repo-owned 规划约束：`project.md` 的 `File Structure / Affected Paths`、handoff 原子步骤模板、以及轻量 planning 自检；它们只能补强现有真值链，不能形成第二套计划系统。
  - SC-7: 从 `executing-plans`、`writing-skills` 与 `test-driven-development` salvage 的 execution / authoring / behavior-first discipline 必须被翻译成 repo-owned surface，并明确哪些 upstream 部分仍保持 rejected/deferred，避免文档继续停留在“未来可能吸收”的过时口径。
  - SC-8: 仍与当前主链冲突的外部规则必须被显式拒绝，包括“fresh subagent-per-task + local two-stage review ritual”“universal brainstorming gate”“universal TDD”。
  - SC-9: 从 `brainstorming` salvage 的 ideation discipline 必须被翻译成 repo-owned bounded brainstorming surface：只在范围仍模糊、需要 2-3 方案对比、或问题本身偏视觉/结构时启用；结论必须回写 `prd.md` / `project.md` / handoff / execution log，不能停留在聊天里，也不能强制逐段审批。
  - SC-10: 从 `using-superpowers` salvage 的流程编排价值必须被翻译成 repo-owned workflow router：负责判断何时进入 bounded brainstorming、behavior-first TDD、execution、verification 与 closeout；它只能路由本地 skill 和 root workflow，不能成为新的外部 bootstrap 或第二套真值。
  - SC-11: `requesting-code-review` 不得继续停留在 adopted 的口头裁定；必须补一个 repo-owned review-request surface，明确何时触发、回传 `findings / no_findings / residual_risk` 哪一种结果，以及这些结果落到哪个 formal sink。
  - SC-12: execution surface 必须把 step-level evidence 从“计划要跑什么”收紧为“实际观察到了什么”，原子步骤至少记录 `Action / Validation Command / Expected Result / Actual Result`，并在重复失败时显式升级 blocker。
  - SC-13: `using-superpowers` 的“对话一开始先进入正确流程”体验若要继续借鉴，必须翻译成 repo-owned `default-workflow-bootstrap`：只负责仓库状态影响判断、标准 task worktree / `.pm` task / owner role 真值检查、formal doc 入口选择与 handoff 到 workflow router；它不得替代 `AGENTS.md`、`.pm`、`prd.md`、`project.md` 或 GitHub PR review。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要判断外部方法论哪些能补强当前流程，哪些会冲掉现有 owner/task/review 边界。
  - `agent_engineer`：需要把 adopted 项转成 repo-owned helper、eval 或 skill，而不是照搬第三方话术。
  - `qa_engineer`：需要对“agent 是否真的遵守流程”建立可复现验证，而不接受口头宣称。
  - `viewer_engineer`：需要一条适用于 UI-heavy 设计题的 optional visual ideation 手段，但不希望它升级为所有实现前的硬门禁。
  - 全体角色 owner / contributor：需要知道“默认成为 subagent”后各自能做什么、不能越过哪条真值边界。
- User Scenarios & Frequency:
  - 外部 workflow 方案评估：每次准备借鉴新的 agent methodology、plugin 或 workflow 契约时执行。
  - 工程工作流补强：每次准备新增 workflow helper、评估 harness 或 completion gate 时执行。
  - 默认多角色任务编排：每次 `tpm` 需要跨 producer/runtime/wasm/agent/viewer/qa/liveops 协作推进一个 task 时执行。
  - Viewer Web 结构/视觉迭代前置设计：只在涉及线框图、结构对比、信息层级验证的任务中按需执行。
- User Stories:
  - PRD-ENGINEERING-AWB-001: As a `producer_system_designer`, I want every external workflow pattern classified as adopted, rejected, or deferred, so that oasis7 only evolves by explicit governance decisions.
  - PRD-ENGINEERING-AWB-002: As a `qa_engineer`, I want repo-owned workflow behavior evals and completion-verification gates, so that agent compliance is proven with evidence rather than trust.
  - PRD-ENGINEERING-AWB-003: As a `viewer_engineer`, I want an optional visual companion pattern for UI-heavy design loops, so that I can compare IA/wireframe options before implementation without turning browser ideation into a universal gate.
  - PRD-ENGINEERING-AWB-004: As a workflow maintainer, I want multi-harness packaging and pluginization held in deferred status until repo-owned behavior and evals are stable, so that distribution does not outrun governance truth.
  - PRD-ENGINEERING-AWB-005: As a workflow maintainer, I want repo-owned planning surfaces to require affected paths, atomic validation steps, and a lightweight self-check, so that `writing-plans` discipline strengthens execution without replacing `prd.md` / `project.md` / `.pm`.
  - PRD-ENGINEERING-AWB-006: As a `tpm`, I want every professional role to participate as a default subagent under one orchestrator, so that multi-role collaboration becomes the normal execution mode without creating parallel task/worktree/review truth.
  - PRD-ENGINEERING-AWB-007: As a behavior-changing feature owner, I want stable implementation tasks to default to behavior-first tests or regression-first verification, so that automated evidence leads production changes without turning universal TDD into a hard gate.
  - PRD-ENGINEERING-AWB-008: As a workflow owner, I want ambiguous or design-heavy tasks to support bounded brainstorming before implementation, so that scope decomposition, option comparison, and optional visual exploration happen intentionally without becoming a universal ceremony.
  - PRD-ENGINEERING-AWB-009: As a workflow owner, I want the repo-owned workflow skills chained through one local router, so that repository-changing tasks enter the right phase in the right order without depending on external bootstrap instructions.
  - PRD-ENGINEERING-AWB-010: As a workflow owner, I want a repo-owned review-request surface for high-risk local diffs, so that review packets and residual risk are explicit before commit or before GitHub review without creating a second formal review chain.
  - PRD-ENGINEERING-AWB-009A: As a workflow owner, I want a repo-owned default bootstrap before routing, so that new repository-changing work automatically establishes isolated task truth before phase selection without importing an external bootstrap.
- Critical User Flows:
  1. Flow-AWB-001: `producer_system_designer` 评估外部 workflow repo -> 提取 planning / review / verification / visual-companion / packaging 模式 -> 冻结 adopted / rejected / deferred 矩阵 -> 只将 adopted 项回写为 repo-owned follow-up。
  2. Flow-AWB-002: adopted 的 workflow 行为补强进入 `engineering` 主项目 -> 形成 helper/eval/smoke/root rule -> 以 repo truth 验证 agent 是否真的遵守 `new-task-worktree -> workflow-report -> TPM orchestrate / professional role subagent dispatch -> task-closeout -> prepare-task-pr -> review-thread-closeout`。
  3. Flow-AWB-003: adopted 的 completion gate 在任务收口前要求 fresh verification evidence -> owner 只有在命令已重新执行并读取结果后，才可宣称“通过/完成/可提 PR”。
  4. Flow-AWB-004: Viewer Web 新一轮结构/视觉专题开始前，若问题本身包含 wireframe/IA/布局对比，则可先启用 visual companion 产出浏览器侧 mockup；确认方向后再创建实现 task，并继续按现有 `agent-browser` / repo-owned UI regression 收口。
  5. Flow-AWB-005: 当一个 task 需要多角色协作时，`tpm` 默认派生所需专业角色 subagent -> 将分析、实现、验证、补充 review 切成 role-owned slices -> 为每个 subagent 明确目标、输入、输出、验证方式与 write scope -> 若 write scope 不互斥则串行，若互斥则可限域并行 -> 所有结果统一回收到同一 owner / `.pm` task / canonical worktree / PR。
  6. Flow-AWB-006: 外部 workflow 若要求替换现有 owner role、GitHub PR review 默认边界或 `.pm` task 真值，则直接标记 rejected；若能翻译成“默认角色 subagent 编排”或“bounded subagent-driven execution”且不替代正式真值，则允许 adopted；若只是 distribution/packaging 问题，则列入 deferred。
  7. Flow-AWB-007: 当 `writing-plans` 的结构化拆分被判定值得借鉴时，owner 先把它翻译为 repo-owned planning surface：在 `project.md` 写 `File Structure / Affected Paths`，在 handoff 写原子步骤、验证命令和预期结果，再按轻量 self-checklist 清掉占位词、遗漏 task 和命名漂移后才进入实现。
  8. Flow-AWB-008: 当任务已经具备 `project.md` / handoff / `.pm` truth 且准备开始实施时，owner 先做一次 execution gap review，确认影响面、步骤、验证和命名一致性，再按原子步骤逐步执行并在每一步后读取实际验证结果；若遇到 scope drift 或重复失败，则先报告 blocker 而不是继续猜测。
  9. Flow-AWB-009: 当任务会改变可自动化验证的行为时，owner 先定义 behavior contract -> 选择窄 scope RED 命令与目标测试文件/测试面 -> 优先通过 `tdd-test-writer` 或等价手工流程让新测试先失败 -> 再落生产实现并复跑同一命令转绿；若任务不适合 RED，则必须写明 skip 原因并继续走现有 evidence-first 主链。
  10. Flow-AWB-010: 当任务方向仍模糊、范围过大，或本质上是产品 / 架构 / UI 取舍题时，owner 先做 bounded brainstorming -> 判断是否需要拆 scope、是否需要 2-3 方案对比、是否值得用 visual companion -> 选定推荐方向后再回写 `prd.md` / `project.md` / handoff / execution log，并进入现有 implementation/verification 主链。
  11. Flow-AWB-011: 当 diff 属于 major feature、跨角色收敛前的高风险切片，或 commit 前 claim risk 明显偏高时，owner 先发起 repo-owned review request -> 明确 review scope、review question、expected return contract 与 formal sink -> 收到 `findings / no_findings / residual_risk` 后再进入 `claim-ready`、commit 或 GitHub PR 下一步。
  12. Flow-AWB-012: 当一个已建立 task truth 的仓库变更 task 启动时，owner 先通过 repo-owned workflow router 判断当前阶段 -> 依次决定是否需要 bounded brainstorming、behavior-first TDD、execution、verification-before-completion 与 finishing-a-development-branch -> 每一步只路由到本地 skill / root workflow surface，不生成新的 bootstrap/spec/task truth。
  13. Flow-AWB-013: 当一个会改变仓库状态的新 task 启动时，owner 先通过 repo-owned `default-workflow-bootstrap` 确认标准 worktree / `.pm` task / owner role 真值与正式文档入口；若 task truth 缺失则先创建，再交给 workflow router 选择 bounded brainstorming、TDD、execution、verification 或 closeout 阶段。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| External workflow inspiration matrix | `source_name`、`source_ref`、`pattern`、`decision=adopted|rejected|deferred`、`rationale`、`target_object`、`followup_ref` | 评估外部 repo/skill 后必须逐项入表；只有 `adopted` 才允许继续拆 follow-up | `proposed -> adopted/rejected/deferred -> superseded` | 先按 `decision`，再按对当前默认流程影响范围排序 | 仅 `producer_system_designer` 可冻结正式结论；相关 owner 联审 |
| Workflow behavior eval contract | `workflow_path`、`fixture_scope`、`expected_agent_behavior`、`verification_surface`、`failure_signature` | 为 adopted 的 workflow rules 建立 repo-owned eval/smoke，验证 agent 在真实回合中是否遵守 | `planned -> implemented -> required/full gated` | 先覆盖主链路，再补压力场景和 drift 场景 | `agent_engineer`/`qa_engineer` 实现，producer 冻结验收口径 |
| Repo-owned default workflow bootstrap contract | `repository_state_impact`、`workspace_state`、`reuse_allowed`、`worktree_action`、`owner_role`、`task_truth`、`routed_next_phase` | 让会改变仓库状态的新 work 先进入 repo-owned bootstrap，建立 task/worktree/doc 真值后再路由到下一个 workflow surface | `unclassified -> bootstrapped -> routed -> phase_active` | 只处理 workflow setup，不替代 routing/execution/closeout；read-only/chat-only 请求可直接处理 | `producer_system_designer` 或当前 owner 触发；正式真值仍由 `AGENTS.md` / `.pm` / `prd.md` / `project.md` 维护 |
| Default role-subagent orchestration contract | `orchestrator_role=tpm`、`owner_role`、`subagent_role`、`write_scope`、`handoff_ref`、`review_boundary` | 默认由 `tpm` 派生所需专业角色 subagent；subagent 仅在声明好的边界内分析、实现、验证或回流，结果统一回收到 TPM owner chain | `not_needed -> dispatched -> returned -> integrated -> closed` | 先保证单 owner / 单 task / 单 worktree，再决定是否允许 disjoint 并行写入 | `tpm` 冻结边界并决定合流；`qa_engineer`/`liveops_community` 可阻断 claim 或对外口径 |
| Repo-owned workflow router contract | `current_phase`、`selected_skills`、`skipped_skills`、`routing_reason`、`writeback_surface` | 对已建立 task truth 的仓库变更 task 先判断当前应进入哪一个 repo-owned workflow phase，并按顺序路由到 bounded brainstorming、TDD、execution、verification 与 closeout skill | `unrouted -> routed -> phase_active -> next_phase_ready -> closed` | 先判定当前阶段与风险面，只路由必要 skill；不得把所有 skill 强制串成 ceremony | `producer_system_designer` 或当前 owner 触发；正式真值仍由 `AGENTS.md` / `.pm` / `prd.md` / `project.md` 维护 |
| Bounded brainstorming contract | `decision_question`、`scope_split`、`options_considered`、`recommended_direction`、`visual_companion_needed`、`writeback_surface` | 对仍需定方向或拆 scope 的任务，先做有限度的 brainstorming：判断是否需要拆分、列 2-3 个方案、给出推荐，并只在问题本身偏视觉/结构时才启用 visual companion | `not_needed -> option_framing -> recommended -> written_back -> implementation_ready` | 先判断是否真的存在方向歧义；若 scope 已明确，就不进入 brainstorming；若启用 visual companion，必须保持 optional 并回到正式实现链 | `producer_system_designer` 或相关 owner 触发；`viewer_engineer` 可辅助视觉比较；最终 owner 冻结方向并回写正式文档 |
| Behavior-first testing contract | `behavior_contract`、`target_test_surface`、`red_command`、`expected_red_failure`、`green_command`、`skip_reason` | 对行为变更且存在稳定自动化 harness 的任务，优先先写失败测试/回归测试，再写生产实现；若不适用则记录 skip 原因 | `not_applicable -> red_defined -> red_verified -> implementation_in_progress -> green_verified` | 先判断是否真有稳定测试面；若没有，则不能伪造 RED，只能记录 skip reason 并继续走现有验证链 | feature owner 决定是否适用；`qa_engineer` 可审查 skip reason；相关实现 owner 执行 RED/GREEN |
| Completion verification gate | `claim_type`、`required_command`、`freshness_rule`、`allowed_evidence`、`blocked_phrases` | 在 owner 宣称“完成/通过/可合并”前，要求 fresh 跑验证并读取结果；失败时只能报告实际状态 | `undefined -> documented -> helper-backed -> smoke-verified` | 每条 claim 必须映射到一个完整命令；禁止 partial evidence 替代 | 全体 owner 遵守；`qa_engineer` 可阻断 |
| Visual companion contract | `applicability`、`artifact_kind=wireframe|layout_compare|IA_mockup|diagram`、`handoff_boundary`、`non_goal` | 只在 UI-heavy 设计前置阶段可选启用；完成方向确认后回到 repo-owned实现/回归链路 | `optional -> used -> retired` | 仅当“看比读更清楚”时启用；不涉及实现时不强制 | `viewer_engineer` 决定是否启用；producer 审核边界 |
| Planning surface tightening contract | `affected_paths`、`read_only_dependencies`、`validation_entrypoints`、`doc_writebacks`、`atomic_steps`、`planning_self_check` | 对复杂 task 把 `writing-plans` 的执行纪律翻译成 repo-owned `project.md`/handoff/checklist 约束，不新建并行计划系统 | `implicit -> documented -> template-backed -> enforced by review` | 先要求影响面可见，再要求步骤和验证可执行，最后检查命名/占位词一致性 | `producer_system_designer` 冻结规则；各 owner 按 task 落地 |
| Execution surface tightening contract | `execution_inputs`、`plan_gap_review`、`atomic_execution_steps`、`step_verification`、`blocker_report_rule`、`closeout_handoff` | 对已有正式计划的 task 把 `executing-plans` 的可 salvage 部分翻译成 repo-owned execution skill 与 root workflow 规则，不生成第二套计划系统 | `implicit -> skill-backed -> review-enforced` | 先确认计划足够可执行，再按步骤落地并逐步验证；一旦 scope drift 或重复失败则停下报告 | 全体 owner 遵守；`producer_system_designer` 冻结规则，`qa_engineer` 可据此阻断 completion claim |
| Deferred packaging track | `target_harness`、`distribution_mode`、`bootstrap_contract`、`eval_requirement` | 只有当 repo-owned workflow 与 eval 稳定后，才允许评估 pluginization/multi-harness packaging | `deferred -> re-opened -> adopted/rejected` | 先本仓库真值，再分发形态 | `producer_system_designer` 开题，相关平台 owner 联审 |
- Superpowers skill decision table | `skill_name`、`decision`、`oasis7_mapping`、`rationale` | 对 `obra/superpowers` 当前 `main` 分支的每个 skill 明确 adopted/rejected/deferred，并写清映射对象或限域边界 | `inventory_snapshot -> adopted/rejected/deferred -> superseded` | 先看是否会引入第二套 workflow 真值，再看是否已有更强 repo-native 等价物 | 仅 `producer_system_designer` 可冻结正式结论；相关 owner 联审 |
- Current `superpowers` skill matrix (`main` snapshot on 2026-05-19):
| skill | decision | oasis7 mapping | rationale |
| --- | --- | --- | --- |
| `verification-before-completion` | adopted | repo-owned `.agents/skills/verification-before-completion` + `scripts/pm/claim-ready.sh` + PR/closeout claim checklist | 与当前 evidence-first 收口完全同向，且现已同时具备 fresh verification helper 与本地 skill 入口。 |
| `using-git-worktrees` | adopted | `./scripts/new-task-worktree.sh` + root `AGENTS.md` 的“一需求一 worktree”规则 | 与当前隔离执行模型一致；仓库内已有更强的 repo-native 原子 bootstrap。 |
| `requesting-code-review` | adopted（bounded） | repo-owned review-request surface + `./scripts/prepare-task-pr.sh` + GitHub PR review 默认边界 | “收口前显式请求 review” 与当前默认 PR 主链一致；只吸收高风险本地 diff 的 review-request packet，不照搬 upstream 的 universal reviewer-dispatch ritual。 |
| `receiving-code-review` | adopted | repo-owned `.agents/skills/receiving-code-review` + `./scripts/pr-review-thread-closeout.sh` + same-PR review fix/verify loop | 强调先验证评论、再修复、再回看 PR 状态，和当前 review-thread closeout 方向一致；现已本地化为 skill。 |
| `finishing-a-development-branch` | adopted | repo-owned `.agents/skills/finishing-a-development-branch` + `task-closeout -> prepare-task-pr -> merge/cleanup` 收口链 | 其“分支收尾、决定如何集成”的结构可直接映射到当前标准收口主链，且现已本地化为 skill。 |
| `systematic-debugging` | adopted | repo-owned `.agents/skills/systematic-debugging` | 价值高且不引入第二套 workflow 真值；现已收口成 repo-owned debugging skill。 |
| `dispatching-parallel-agents` | adopted | repo-owned 默认 `tpm` orchestrator + professional role subagents，要求单 owner / 单 `.pm` task / 单 canonical worktree / GitHub PR review 真值 | 可借其拆分原则，但必须翻译成受限的 repo-native 角色编排，而不是无边界 swarm。 |
| `executing-plans` | deferred | bounded local execution surface: `.agents/skills/executing-project-tasks` + `AGENTS.md` execution-gap rules + existing `finishing-a-development-branch` closeout path | 已借到“已有正式计划后的执行 discipline”：先做 plan-gap review、按原子步骤推进、逐步验证、遇到 blocker 明确停下；但 upstream 的单独执行会话包装与末尾 handoff 假设仍不作为新的默认真值。 |
| `writing-skills` | deferred | bounded local skill-authoring governance (`.agents/skills/README.md` + `writing-repo-owned-skills` + template/checklist) | 已借 authoring surface，但 upstream 的 TDD/subagent gate、分发与部署部分仍保持 deferred。 |
| `brainstorming` | adopted | repo-owned bounded brainstorming contract：只在任务仍需定方向、拆 scope 或比较方案时启用 2-3 方案对比与推荐方向，并按需启用 visual companion | 只借 ideation discipline、scope decomposition、option framing 与 optional visual companion；universal gate、逐段审批与强制转入 `writing-plans` 继续保持 rejected。 |
| `subagent-driven-development` | adopted | repo-owned bounded subagent-driven execution：`tpm` 在单 owner / `.pm` task / worktree / PR 真值内派生专业角色 subagent 处理分析、实现、验证与补充 review 切片 | 只借执行切片、上下文最小化与实现/验证分工；fresh subagent-per-task + 本地双阶段 review ritual 继续保持 rejected。 |
| `test-driven-development` | adopted | repo-owned bounded behavior-first testing contract：行为变更且存在稳定自动化 harness 时，默认先补失败测试/回归测试，再写生产实现；不适用时必须写 skip 原因 | 只借 behavior-first / regression-first discipline 与 RED 验证；universal TDD 继续保持 rejected。 |
| `writing-plans` | rejected | repo-owned `File Structure / Affected Paths` + handoff atomic-step templates + planning self-checklist | skill 本体仍不能升成默认前置，但其结构化拆分纪律已被限域翻译为当前 planning surface。 |
| `using-superpowers` | rejected（overall bootstrap） | repo-owned workflow router：`.agents/skills/repo-owned-workflow-router` + `.agents/skills/default-workflow-bootstrap` + `AGENTS.md` 的默认 phase order + `.agents/skills/README.md` entrypoint | 外部 bootstrap 不能取代当前主链，但其中“先选对本地 process skill 再进入下一阶段”的编排价值已被翻译成 repo-owned bootstrap + router。 |
- Acceptance Criteria:
  - AC-1: 专题必须明确写出 `superpowers` 当前 `main` 分支 skill inventory 的 adopted / rejected / deferred 清单，且每项都带 rationale 与 oasis7 mapping。
  - AC-2: adopted 项至少形成十条正式落点：workflow behavior eval harness、completion-claim verification gate、Viewer visual companion pilot、root `AGENTS.md` 的 repo-owned default bootstrap、repo-owned workflow router、bounded brainstorming 规则、默认 role-subagent orchestration 规则、bounded subagent-driven execution 规则、repo-owned review-request surface，以及 bounded behavior-first testing contract；同时 `verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch`、`executing-project-tasks`、`tdd-test-writer`、`bounded-brainstorming`、`requesting-repo-owned-review`、`default-workflow-bootstrap` 与 `repo-owned-workflow-router` 已允许并落为本地 repo-owned skills / workflow entry points。
  - AC-3: `brainstorming`、`dispatching-parallel-agents`、`subagent-driven-development`、`test-driven-development`、repo-owned workflow router 与 repo-owned default bootstrap 必须在正式文档中被翻译为 adopted（bounded），并明确它们只等于“按需启用的 bounded brainstorming + 默认角色 subagent 编排 + 同一真值链内的 subagent-driven execution + 行为变更任务上的 bounded behavior-first testing + 本地 bootstrap/routing”，不等于“强制 brainstorming gate + fresh subagent-per-task + 本地两阶段 review + 无条件 universal TDD + 外部 bootstrap”。
  - AC-4: rejected 项必须显式覆盖与 oasis7 当前默认流程冲突的三类外部规则：强制 brainstorming gate、fresh subagent-per-task + local two-stage review ritual、无条件 universal TDD。
  - AC-5: deferred 项必须把 multi-harness pluginization 与自动 skill bootstrap 维持在“非当前默认流程”边界，不得混入 root `AGENTS.md` 现行口径。
  - AC-6: `engineering` 根入口、主项目、文件级索引和 `world-simulator` Viewer 后续参考口径必须完成回写。
  - AC-7: 默认 subagent 编排与 bounded subagent-driven execution 都不得直接修改 owner/review/task 真值；所有 adopted 项都必须以 repo-owned root rule、skill、helper 或 follow-up task 落地。
  - AC-7A: root workflow 与 handoff template 必须显式要求每个默认 subagent slice 声明 `slice type / write scope / return contract / integration owner`，复杂场景还需补 `integration order`；若缺任一项，不得宣称符合默认 subagent-driven 流程。
  - AC-7B: 对行为变更且存在稳定自动化测试面的实现任务，root workflow 与 handoff template 必须显式要求 `behavior contract / target test surface / RED command or skip reason`；若跳过 RED，必须能从正式文档或 execution log 回放原因。
  - AC-8: `writing-plans` 的可 salvage 部分必须被收口成 repo-owned planning surface，而不是继续停留在“以后可以借”的抽象结论。
  - AC-9: `executing-plans` 的可 salvage 部分必须被收口成 repo-owned execution surface：进入实施前先做 execution gap review、实施时按原子步骤逐步验证、遇到 blocker 明确停下并回写真值。
- Non-Goals:
  - 不把 `superpowers` 或其他外部 workflow repo 直接接入为 oasis7 当前默认 bootstrap。
  - 不把“默认角色 subagent 编排”放宽成无 owner、无 write-scope、无 worktree 约束的自由 swarm。
  - 不让 visual companion 替代 `agent-browser`、repo-owned UI regression 或 GitHub PR review。
  - 不在本期实现 multi-harness plugin packaging。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - repo-owned shell/PM helpers：承接 adopted 的 workflow closeout、PR preflight、review-thread closeout 与已落地的 completion gate / claim helper。
  - `agent-browser`：仅服务于 adopted 的 visual companion pilot 和现有 Web 回归，不作为默认万能工作流。
  - workflow behavior eval fixtures：由 `scripts/pm/workflow-behavior-eval.sh` 统一调用 default bootstrap / root workflow contract surface 校验、task-worktree bootstrap smoke、subagent contract surface 校验、PM runtime smoke、`prepare-task-pr.test.sh` 与 `pr-review-thread-closeout.test.sh`，用于证明 agent 在真实 workflow helper 主链上是否仍遵守规则。
- Evaluation Strategy:
  - 以 repo-owned eval/smoke 验证 adopted workflow rules 是否被 agent 实际执行。
  - 以 targeted fresh verification checks 验证 completion claims 是否具备足够证据。
  - 以 Viewer Web 专题前置设计样例验证 visual companion 是否真能降低结构/视觉分歧，而不是只增加 ceremony。
  - 以至少一个 high-risk local diff 的 review-request packet 验证 repo-owned review surface 是否真能沉淀 findings / residual risk，而不是继续停留在 adopted 口号。

## 4. Technical Specifications
- Architecture Overview:
  - 本专题负责“借鉴边界”和“repo-owned workflow 映射”；在保留单 owner / task / review 真值的前提下，允许把默认多角色 subagent 编排与 bounded subagent-driven execution 接回当前 workflow 入口。
  - adopted 项统一通过 repo-owned helper、eval、smoke、root rule 或模块专题 follow-up 落地；rejected 项明确写入 guardrail；deferred 项保持在 backlog，不进入默认主链。
  - Viewer 方向的 visual companion 只作为 `world-simulator/viewer` 专题的前置设计辅助手段，和实现 task、browser regression、repo-owned UI 测试分层存在。
- Integration Points:
  - `AGENTS.md`
  - `.agents/roles/producer_system_designer.md`
  - `.agents/roles/templates/handoff-brief.md`
  - `.agents/roles/templates/handoff-detailed.md`
  - `.agents/roles/templates/planning-self-checklist.md`
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/README.md`
  - `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
  - `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
  - `doc/world-simulator/project.md`
  - `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.project.md`
  - `testing-manual.md`
  - `https://github.com/obra/superpowers/tree/main`
- Edge Cases & Error Handling:
  - adopted 项仍停留在聊天结论：必须视为未完成，直到进入正式 project/task 追踪。
  - 外部规则与当前流程局部相似但默认假设不同：必须按 repo truth 重写，不允许直接复述原规则。
  - bounded brainstorming 若既没有 option framing / 推荐方向，也没有回写正式文档：视为仍停留在聊天层，不得宣称已吸收 upstream `brainstorming`。
  - repo-owned workflow router 若只是罗列 skill 名称，却没有说明当前阶段、为何跳过其他阶段或需要回写哪一份正式文档：视为没有真正把流程串起来。
  - 默认角色 subagent 或 subagent-driven execution 未声明 owner、write scope、return contract 或 handoff：视为未绑定真值，不得执行并行写入或宣称流程合规。
  - 行为变更任务若声称采用 TDD / behavior-first，但没有 RED 命令、目标测试面或 skip 原因：视为未满足 bounded TDD contract。
  - visual companion 被误升级为所有需求的 mandatory pre-step：必须回退到 optional 设计辅助边界。
  - completion verification gate 只验证部分命令或旧结果：视为无效 evidence，不得宣称完成。
  - workflow eval 只验证静态文案而不验证 agent 行为：视为 coverage 不足，不得声称 adopted 项已经落地。
  - planning surface 只有“多写了一个段落”，但没有把验证命令、预期结果或命名漂移写清：视为借到了形式，没借到执行纪律。
  - execution surface 只有“照着做”，但没有在步骤间实际跑验证，或遇到重复失败仍继续猜测：视为借到了口号，没借到执行 discipline。
- Non-Functional Requirements:
  - NFR-AWB-1: 借鉴矩阵中的每一条 adopted / rejected / deferred 结论都必须可通过正式文档回放。
  - NFR-AWB-2: adopted 项不得引入新的在线依赖、外部真值或多 harness bootstrap 作为当前默认前提。
  - NFR-AWB-3: workflow behavior eval 的首批覆盖必须至少命中 default bootstrap、task-worktree、closeout、PR preflight、review-thread closeout 五段主链。
  - NFR-AWB-3A: 默认角色 subagent 编排与 bounded subagent-driven execution 不得改变 `workflow-report start/close`、`.pm` task 状态、task execution log 记录责任人与 GitHub PR review 正式边界。
  - NFR-AWB-3B: 默认 subagent-driven 流程必须能在 handoff / planning surface 中回放出每个 slice 的 write scope、return contract 与 integration order，避免“已经派了 subagent，但无法审计它被要求交付什么”。
  - NFR-AWB-3C: bounded TDD contract 不得把 universal TDD 写回 root 默认门禁；它只能约束“行为变更且存在稳定自动化测试面”的实现任务，并且仍需落在现有 `test_tier_required/full` 与 GitHub PR 主链内。
  - NFR-AWB-3D: bounded brainstorming contract 不得把 universal brainstorming gate、逐段审批或单独 spec 流程写回 root 默认门禁；它只能约束“方向仍模糊、范围过大或问题本身偏视觉/结构”的任务，并且产出必须回写到现有 `prd.md` / `project.md` / handoff / execution log。
  - NFR-AWB-3E: repo-owned workflow router 不得取代 root workflow 真值；它只能编排当前已经 adopted 的本地 workflow surface，并且阶段切换必须可回放到 `AGENTS.md`、`.agents/skills/README.md`、handoff、`project.md` 或 execution log。
  - NFR-AWB-4: visual companion pilot 不得增加 world-simulator Viewer 默认 required gate 的在线依赖。
  - NFR-AWB-5: planning surface tightening 不得要求额外在线依赖、外部 bootstrap 或第二套 plan storage；所有新增约束必须落在现有 repo-owned 文档和模板里。
  - NFR-AWB-6: execution surface tightening 不得绕开 `project.md` / `.pm` / task execution log / GitHub PR review，也不得把 step-level verification 替换成事后总结式宣称。
- Security & Privacy:
  - 外部 workflow 借鉴只保留结构化治理结论与公开来源链接，不导入第三方服务或隐式权限提升。
  - adopted 的 completion gate、默认 role-subagent orchestration 与 bounded subagent-driven execution 都必须继续遵守当前仓库的显式 owner、review、task traceability 规则。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP（completed）: 建立本专题三件套，冻结 `superpowers` 的 adopted / rejected / deferred 边界，并回写 engineering / Viewer 参考口径。
  - 已完成的 bounded adoption:
    - completion-claim verification gate
    - workflow behavior eval harness
    - repo-owned default workflow bootstrap
    - `writing-plans` -> repo-owned planning surface
    - `executing-plans` -> repo-owned execution surface
    - `writing-skills` -> repo-owned skill authoring surface
    - `dispatching-parallel-agents` -> 默认角色 subagent 编排
    - `subagent-driven-development` -> 默认 subagent-driven execution
    - `test-driven-development` -> behavior-first testing contract
    - `brainstorming` -> bounded brainstorming contract
    - `using-superpowers` 的 process-skill routing order -> repo-owned workflow router
    - `using-superpowers` 的“默认先进入正确流程”体验 -> repo-owned default workflow bootstrap
  - 剩余 follow-up:
    - workflow enforcement audit follow-up：把 helper 输出升级为 `.pm` task 真值，并让底层 `done` 状态迁移直接拒绝无 fresh verification evidence 的 closeout
    - Viewer visual companion pilot：仅在下一轮结构/视觉专题里验证 optional ideation 收益，并要求 IA / wireframe / layout compare artifact 与 formal writeback
    - repo-owned review-request surface：只对 high-risk local diff 启用，验证 review packet、return contract 与 formal sink 不会漂成第二条正式 review 主链
  - Deferred reopen:
    - multi-harness workflow packaging：仅在 repo-owned behavior/eval 稳定后重开
- Technical Risks:
  - 风险-1: 若只冻结 adopted 项、不补 repo-owned eval，最终会退化成“又一份 workflow 口号”。
  - 风险-2: 若不明确 rejected 项，外部 repo 的强制 ceremony 容易被误当成当前默认流程。
  - 风险-3: 若 visual companion 没有严格限域，可能把 Viewer 设计题和一般实现题混成统一前置门禁。
  - 风险-4: 若默认角色 subagent 没有 write scope / owner / return contract 约束，会造成 overlapping writes、`.pm` 漂移或 review 责任不清。
  - 风险-5: 若 packaging 先于 repo-owned truth 稳定，会造成“可分发但不可审计”的反向漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-AWB-001 | `agent-workflow-borrowing-governance`、`workflow-behavior-eval-and-closeout-hardening` | `test_tier_required` + `test_tier_full` | adopted/rejected/deferred 矩阵、repo-owned agent behavior eval、主链 workflow helper 回放 | `engineering` workflow 主链、agent 行为一致性 |
| PRD-ENGINEERING-AWB-002 | `completion-claim-verification-followup`、`pm-step-evidence-lint-defaulting`、`workflow-enforcement-audit-followup` | `test_tier_required` | fresh verification claim checklist/helper/smoke、execution-log step evidence lint、task file evidence 持久化、低层 `move-task done` 绕过阻断与失败签名 | task closeout、PR preflight、QA 报告口径、`.pm` 状态机真值 |
| PRD-ENGINEERING-AWB-003 | `viewer-visual-companion-pilot-followup` | `test_tier_required` | Viewer Web 前置 mockup/IA 对比样例、实现 task handoff、后续 `agent-browser`/repo-owned regression 不回退 | `world-simulator/viewer` 设计前置链路 |
| PRD-ENGINEERING-AWB-010 | `repo-owned-review-request-followup` | `test_tier_required` | `requesting-repo-owned-review` skill、root workflow trigger wording、review packet writeback 与治理文档对齐 | `engineering` 的 commit 前高风险 diff 补充 review 链路 |
| PRD-ENGINEERING-AWB-008 | `bounded-brainstorming-workflow-rollout` | `test_tier_required` | `AGENTS.md` 的 bounded brainstorming rule、handoff/planning option-framing 字段、本地 `bounded-brainstorming` skill，以及 borrowing/conflict/skill-surface 文档改判 | `engineering` 的方向探索、scope decomposition 与 optional visual ideation 边界 |
| PRD-ENGINEERING-AWB-009 | `repo-owned-workflow-router` | `test_tier_required` | root `AGENTS.md` 的 phase order、`.agents/skills/repo-owned-workflow-router`、`.agents/skills/README.md` 入口，以及 borrowing/conflict/skill-surface 文档对齐 | `engineering` 的端到端流程路由、阶段切换与本地 process-skill 组合 |
| PRD-ENGINEERING-AWB-009A | `default-workflow-bootstrap` | `test_tier_required` | `.agents/skills/default-workflow-bootstrap`、root `AGENTS.md` 默认入口、`.agents/skills/README.md` entrypoint 与 `workflow-behavior-eval.sh` contract surface 对齐 | `engineering` 的新任务默认入口、task truth 建立与路由前置 |
| PRD-ENGINEERING-AWB-004 | `multi-harness-workflow-packaging-deferred` | `test_tier_required` | 仅验证 deferred 口径与 reopen 条件是否写清 | pluginization / harness distribution 边界 |
| PRD-ENGINEERING-AWB-005 | `workflow-planning-surface-tightening` | `test_tier_required` | `AGENTS.md` 规则、handoff 模板、planning self-checklist、topic/root project 回写与文档治理校验 | `engineering` planning / handoff / review 准备链路 |
| PRD-ENGINEERING-AWB-006 | `default-role-subagent-orchestration`、`role-subagent-local-validation`、`subagent-driven-default-reconciliation`、`subagent-driven-default-workflow-rollout`、`workflow-behavior-eval-and-closeout-hardening` | `test_tier_required` + `test_tier_full` | `AGENTS.md` 默认 orchestrator/subagent 规则与 bounded subagent-driven execution、handoff/planning contract、borrowing/conflict 文档改判、repo-owned multi-agent behavior eval | `engineering` 多角色协作、owner/task/worktree/PR 真值边界 |
| PRD-ENGINEERING-AWB-007 | `bounded-tdd-workflow-rollout` | `test_tier_required` | `AGENTS.md` 的 behavior-first testing rule、handoff/planning/test-skill 回写、borrowing/conflict 文档改判与技能边界对齐 | `engineering` 行为变更类实现任务、自动化回归与 skip-reason 审计边界 |
| PRD-ENGINEERING-031 | `workflow-execution-surface-tightening`、`pm-step-evidence-lint-defaulting` | `test_tier_required` | repo-owned execution skill、`AGENTS.md` execution rule、execution-log step evidence lint、workflow-borrowing / conflict doc 回写与文档治理校验 | `engineering` task 执行、逐步验证与 blocker handling 链路 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-AWB-001 | 只借鉴结构化 workflow patterns，并把 adopted 项转成 repo-owned follow-up | 直接把 `superpowers` 当默认 bootstrap 或当前流程真值 | 当前真值已是 `AGENTS.md + .pm + GitHub PR review`，不能再并行引入第二条主链。 |
| DEC-AWB-002 | 采用 repo-owned workflow behavior eval 作为首批落地点 | 只更新文档口径，不验证 agent 实际是否遵守 | 风险核心是“文档结论”和真实 agent 行为脱钩。 |
| DEC-AWB-003 | 采用 fresh verification before completion claim 的治理方向 | 继续允许 owner 用旧结果、局部结果或 agent 自报成功来宣称完成 | 这与现有 evidence-first 收口同向，而且能补当前 claim 边界短板。 |
| DEC-AWB-004 | visual companion 只作为 Viewer 等 UI-heavy 设计题的 optional ideation layer | 把 browser ideation 升级成所有实现题的 universal gate | 强制设计门禁会破坏当前直接执行节奏。 |
| DEC-AWB-005 | 将 multi-harness pluginization 保持 deferred | 在 repo-owned eval 稳定前立即推进 Codex/OpenCode packaging | 分发形态不能跑在治理真值前面。 |
| DEC-AWB-006 | 明确拒绝 universal brainstorming / fresh subagent-per-task / universal TDD 三类默认规则 | 以“外部方法论更完整”为理由整体照搬 | 这些规则直接冲突当前 owner 授权、subagent 边界、测试分层和用户节奏。 |
| DEC-AWB-007 | 将 `writing-plans` 的可 salvage 部分限域翻译为 `project.md` affected paths、handoff atomic steps 和 lightweight self-check | 继续把 `writing-plans` 整体维持在“只有理论价值”的 rejected 状态，或反向把它升成新的默认计划入口 | 保留执行纪律增益，同时避免第二套计划真值。 |
| DEC-AWB-008 | 将 `executing-plans` 的可 salvage 部分限域翻译为 repo-owned execution skill 与 root execution 规则 | 继续把它停留在“未来也许有用”的纯 deferred 口头结论，或反向引入单独执行会话契约 | 真正值得借的是执行 discipline，不是新的计划存储或 session 包装。 |
| DEC-AWB-009 | 将 `dispatching-parallel-agents` 改判为 adopted，但只落成 `tpm` orchestrator + professional role subagents 的默认编排层 | 继续保持 deferred，或直接升级成无 owner / 无 scope 的自由多 agent swarm | 仓库需要默认多角色协作，但正式真值仍必须维持 owner/task/worktree/PR 单链。 |
| DEC-AWB-010 | 将 `subagent-driven-development` 改判为 adopted（bounded），但只吸收“同一真值链内的 subagent-driven execution”，继续拒绝 fresh subagent-per-task + local two-stage review ritual | 继续保持 rejected，或整体照搬其 fresh-subagent / local-review ceremony | 可兼容的是执行切片与上下文最小化，不是再造本地评审主链。 |
| DEC-AWB-011 | 将 `test-driven-development` 改判为 adopted（bounded），但只吸收“行为变更 + 稳定自动化 harness”上的 behavior-first / regression-first contract，并允许显式 skip reason | 继续保持整体 rejected，或把 universal TDD 升成所有任务的硬门禁 | 真正缺的是“何时必须让自动化行为证据先行”的统一口径。 |
| DEC-AWB-012 | 将 `brainstorming` 改判为 adopted（bounded），但只吸收“按需 scope decomposition + 2-3 方案对比 + 推荐方向 + optional visual companion”，继续拒绝 universal gate、逐段审批与强制转入 `writing-plans` | 继续保持整体 rejected，或把 brainstorming 升成所有任务的 mandatory pre-step | 真正缺的是“何时应先定方向再动手”的统一口径。 |
| DEC-AWB-013 | 保持 `using-superpowers` 的外部 bootstrap 语义 rejected，但把其中“串联本地 workflow skill 的 phase order”翻译成 repo-owned workflow router | 继续保持 `using-superpowers` 完全无落点，或整体采纳为对话默认入口 | 真正缺的是串联已 adopted 本地 skill 的总入口，而不是重新依赖外部 bootstrap。 |
| DEC-AWB-014 | 将 `requesting-code-review` 落成 bounded repo-owned review-request surface，只在高风险 local diff/claim risk 场景启用，并强制回写 findings/no-findings/residual-risk | 继续维持 adopted 但无本地落点，或照搬 upstream 的 every-task reviewer-dispatch ritual | 当前缺的是真正可回放的 review request sink，而不是再造一条 universal local review 主链。 |
| DEC-AWB-015 | 新增 repo-owned `default-workflow-bootstrap` 作为本地默认入口，但只负责建立 task/worktree/doc 真值并转交给 router | 继续把 bootstrap 体验散落在 `AGENTS.md` 文案里，或直接恢复外部 `using-superpowers` 入口语义 | 当前真正缺的是“新任务先建立正确 repo-owned 真值再路由”的统一入口，而不是回退成外部 bootstrap。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 是否明确说明本期解决什么问题：已明确“外部 workflow 借鉴边界不清”这一治理缺口。
  - ✔ 是否定义成功指标：SC-1~SC-10 已量化 adopted/rejected/deferred、repo-owned mapping 和边界约束。
- 用户与场景（Who / When）:
  - ✔ 是否明确目标用户与场景：producer、agent、QA、viewer 均已定义。
- 范围定义（Scope Control）:
  - ✔ 是否列出本期功能清单：借鉴矩阵、workflow eval、completion gate、visual companion、planning/execution surface 与 deferred packaging 已覆盖。
  - ✔ 是否明确 Out of Scope：未把外部 bootstrap、universal gate、pluginization 实施纳入本期。
- 功能规格（What）:
  - ✔ 是否定义动作、状态、权限和 follow-up 映射：规格矩阵已覆盖。
- 异常与边界（Edge Cases）:
  - ✔ 是否覆盖 adopted 无落地、visual companion 越界、partial verification 等关键风险：已覆盖。
- 非功能需求（NFR）:
  - ✔ 是否定义可审计、无新增真值、主链覆盖等约束：NFR-AWB-1~6 已覆盖。
- 可测试性（Testability）:
  - ✔ 是否给出 traceability、验证方法与回归范围：第 6 节已覆盖。
- 结论:
  - 🟢 Ready
