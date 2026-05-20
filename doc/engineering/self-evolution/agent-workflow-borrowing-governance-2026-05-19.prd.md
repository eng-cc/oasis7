# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）

- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`

审计轮次: 1

- 对应标准执行入口: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`

## 1. Executive Summary
- Problem Statement: 外部 agent workflow 方法论已经开始提供成套的规划、TDD、subagent、browser companion 与 PR 收口建议；但 oasis7 现有 repo-native 真值链是 `AGENTS.md + .agents/roles + .pm + GitHub PR review`。如果不先冻结“哪些可借鉴、哪些冲突、哪些仅可选”，后续很容易把外部 skill 文案或 harness 习惯误写成当前仓库默认流程。
- Proposed Solution: 在 `engineering/self-evolution` 下建立正式专题，首批以 `obra/superpowers` 为样本，冻结 adopted / rejected / deferred 边界，并把 adopted 项只转译为 repo-owned follow-up：workflow behavior eval harness、completion-claim fresh verification gate、Viewer Web 设计阶段的 optional visual companion。
- Success Criteria:
  - SC-1: `superpowers` 首批借鉴项 100% 进入 `adopted / rejected / deferred` 三态矩阵，并为每项给出理由与 repo-owned target object。
  - SC-2: 每个 adopted 项都必须映射到一个 repo-owned follow-up task 或明确的模块参考入口，不允许停留在“聊天建议”层。
  - SC-3: 与 oasis7 当前默认流程冲突的外部规则必须被显式拒绝或限域，包括“任何改动都先 brainstorming”“任何任务都默认 fresh subagent + 两轮 review”“任何改动都硬性 TDD”。
  - SC-4: Viewer Web 视觉/结构类专题必须明确：browser-based visual companion 仅是前置设计手段，不替代 `agent-browser` 回归、repo-owned UI regression 或正式实现 task。
  - SC-5: 外部 workflow 借鉴不得引入新的运行态真值系统，不得替代 `.pm`、`project.md`、task execution log 或 GitHub PR review。
  - SC-6: 从 `writing-plans` salvage 的 planning discipline 必须被翻译成 repo-owned 规划约束：`project.md` 的 `File Structure / Affected Paths`、handoff 原子步骤模板、以及轻量 planning 自检；它们只能补强现有真值链，不能形成第二套计划系统。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要判断外部方法论哪些能补强当前流程，哪些会冲掉现有 owner/task/review 边界。
  - `agent_engineer`：需要把 adopted 项转成 repo-owned helper、eval 或 skill，而不是照搬第三方话术。
  - `qa_engineer`：需要对“agent 是否真的遵守流程”建立可复现验证，而不接受口头宣称。
  - `viewer_engineer`：需要一条适用于 UI-heavy 设计题的 optional visual ideation 手段，但不希望它升级为所有实现前的硬门禁。
- User Scenarios & Frequency:
  - 外部 workflow 方案评估：每次准备借鉴新的 agent methodology、plugin 或 workflow 契约时执行。
  - 工程工作流补强：每次准备新增 workflow helper、评估 harness 或 completion gate 时执行。
  - Viewer Web 结构/视觉迭代前置设计：只在涉及线框图、结构对比、信息层级验证的任务中按需执行。
- User Stories:
  - PRD-ENGINEERING-AWB-001: As a `producer_system_designer`, I want every external workflow pattern classified as adopted, rejected, or deferred, so that oasis7 only evolves by explicit governance decisions.
  - PRD-ENGINEERING-AWB-002: As a `qa_engineer`, I want repo-owned workflow behavior evals and completion-verification gates, so that agent compliance is proven with evidence rather than trust.
  - PRD-ENGINEERING-AWB-003: As a `viewer_engineer`, I want an optional visual companion pattern for UI-heavy design loops, so that I can compare IA/wireframe options before implementation without turning browser ideation into a universal gate.
  - PRD-ENGINEERING-AWB-004: As a workflow maintainer, I want multi-harness packaging and pluginization held in deferred status until repo-owned behavior and evals are stable, so that distribution does not outrun governance truth.
  - PRD-ENGINEERING-AWB-005: As a workflow maintainer, I want repo-owned planning surfaces to require affected paths, atomic validation steps, and a lightweight self-check, so that `writing-plans` discipline strengthens execution without replacing `prd.md` / `project.md` / `.pm`.
- Critical User Flows:
  1. Flow-AWB-001: `producer_system_designer` 评估外部 workflow repo -> 提取 planning / review / verification / visual-companion / packaging 模式 -> 冻结 adopted / rejected / deferred 矩阵 -> 只将 adopted 项回写为 repo-owned follow-up。
  2. Flow-AWB-002: adopted 的 workflow 行为补强进入 `engineering` 主项目 -> 形成 helper/eval/smoke -> 以 repo truth 验证 agent 是否真的遵守 `new-task-worktree -> workflow-report -> task-closeout -> prepare-task-pr -> review-thread-closeout`。
  3. Flow-AWB-003: adopted 的 completion gate 在任务收口前要求 fresh verification evidence -> owner 只有在命令已重新执行并读取结果后，才可宣称“通过/完成/可提 PR”。
  4. Flow-AWB-004: Viewer Web 新一轮结构/视觉专题开始前，若问题本身包含 wireframe/IA/布局对比，则可先启用 visual companion 产出浏览器侧 mockup；确认方向后再创建实现 task，并继续按现有 `agent-browser` / repo-owned UI regression 收口。
  5. Flow-AWB-005: 外部 workflow 若要求替换现有 owner role、GitHub PR review 默认边界或 `.pm` task 真值，则直接标记 rejected；若只是 distribution/packaging 问题，则列入 deferred，不提前修改当前仓库默认流程。
  6. Flow-AWB-006: 当 `writing-plans` 的结构化拆分被判定值得借鉴时，owner 先把它翻译为 repo-owned planning surface：在 `project.md` 写 `File Structure / Affected Paths`，在 handoff 写原子步骤、验证命令和预期结果，再按轻量 self-checklist 清掉占位词、遗漏 task 和命名漂移后才进入实现。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| External workflow inspiration matrix | `source_name`、`source_ref`、`pattern`、`decision=adopted|rejected|deferred`、`rationale`、`target_object`、`followup_ref` | 评估外部 repo/skill 后必须逐项入表；只有 `adopted` 才允许继续拆 follow-up | `proposed -> adopted/rejected/deferred -> superseded` | 先按 `decision`，再按对当前默认流程影响范围排序 | 仅 `producer_system_designer` 可冻结正式结论；相关 owner 联审 |
| Workflow behavior eval contract | `workflow_path`、`fixture_scope`、`expected_agent_behavior`、`verification_surface`、`failure_signature` | 为 adopted 的 workflow rules 建立 repo-owned eval/smoke，验证 agent 在真实回合中是否遵守 | `planned -> implemented -> required/full gated` | 先覆盖主链路，再补压力场景和 drift 场景 | `agent_engineer`/`qa_engineer` 实现，producer 冻结验收口径 |
| Completion verification gate | `claim_type`、`required_command`、`freshness_rule`、`allowed_evidence`、`blocked_phrases` | 在 owner 宣称“完成/通过/可合并”前，要求 fresh 跑验证并读取结果；失败时只能报告实际状态 | `undefined -> documented -> helper-backed -> smoke-verified` | 每条 claim 必须映射到一个完整命令；禁止 partial evidence 替代 | 全体 owner 遵守；`qa_engineer` 可阻断 |
| Visual companion contract | `applicability`、`artifact_kind=wireframe|layout_compare|IA_mockup|diagram`、`handoff_boundary`、`non_goal` | 只在 UI-heavy 设计前置阶段可选启用；完成方向确认后回到 repo-owned实现/回归链路 | `optional -> used -> retired` | 仅当“看比读更清楚”时启用；不涉及实现时不强制 | `viewer_engineer` 决定是否启用；producer 审核边界 |
| Planning surface tightening contract | `affected_paths`、`read_only_dependencies`、`validation_entrypoints`、`doc_writebacks`、`atomic_steps`、`planning_self_check` | 对复杂 task 把 `writing-plans` 的执行纪律翻译成 repo-owned `project.md`/handoff/checklist 约束，不新建并行计划系统 | `implicit -> documented -> template-backed -> enforced by review` | 先要求影响面可见，再要求步骤和验证可执行，最后检查命名/占位词一致性 | `producer_system_designer` 冻结规则；各 owner 按 task 落地 |
| Deferred packaging track | `target_harness`、`distribution_mode`、`bootstrap_contract`、`eval_requirement` | 只有当 repo-owned workflow 与 eval 稳定后，才允许评估 pluginization/multi-harness packaging | `deferred -> re-opened -> adopted/rejected` | 先本仓库真值，再分发形态 | `producer_system_designer` 开题，相关平台 owner 联审 |
- Superpowers skill decision table | `skill_name`、`decision`、`oasis7_mapping`、`rationale` | 对 `obra/superpowers` 当前 `main` 分支的每个 skill 明确 adopted/rejected/deferred，并写清映射对象或限域边界 | `inventory_snapshot -> adopted/rejected/deferred -> superseded` | 先看是否会引入第二套 workflow 真值，再看是否已有更强 repo-native 等价物 | 仅 `producer_system_designer` 可冻结正式结论；相关 owner 联审 |
- Current `superpowers` skill matrix (`main` snapshot on 2026-05-19):
| skill | decision | oasis7 mapping | rationale |
| --- | --- | --- | --- |
| `verification-before-completion` | adopted | repo-owned `.agents/skills/verification-before-completion` + `scripts/pm/claim-ready.sh` + PR/closeout claim checklist | 与当前 evidence-first 收口完全同向，且现已同时具备 fresh verification helper 与本地 skill 入口。 |
| `using-git-worktrees` | adopted | `./scripts/new-task-worktree.sh` + root `AGENTS.md` 的“一需求一 worktree”规则 | 与当前隔离执行模型一致；仓库内已有更强的 repo-native 原子 bootstrap。 |
| `requesting-code-review` | adopted | `./scripts/prepare-task-pr.sh` + GitHub PR review 默认边界 | “收口前显式请求 review” 与当前默认 PR 主链一致，只是不照搬其 reviewer-dispatch 语义。 |
| `receiving-code-review` | adopted | repo-owned `.agents/skills/receiving-code-review` + `./scripts/pr-review-thread-closeout.sh` + same-PR review fix/verify loop | 强调先验证评论、再修复、再回看 PR 状态，和当前 review-thread closeout 方向一致；现已本地化为 skill。 |
| `finishing-a-development-branch` | adopted | repo-owned `.agents/skills/finishing-a-development-branch` + `task-closeout -> prepare-task-pr -> merge/cleanup` 收口链 | 其“分支收尾、决定如何集成”的结构可直接映射到当前标准收口主链，且现已本地化为 skill。 |
| `systematic-debugging` | adopted | repo-owned `.agents/skills/systematic-debugging` | 价值高且不引入第二套 workflow 真值；现已收口成 repo-owned debugging skill。 |
| `dispatching-parallel-agents` | deferred | bounded `spawn_agent` usage under explicit authorization | 可借其拆分原则，但不能回流成默认 subagent-first 工作流。 |
| `executing-plans` | deferred | future plan-execution follow-up if a repo-owned contract is needed | 可用于“已有正式计划后的执行会话”，但当前仓库已经有 `project.md`/`.pm`，不急于再引入单独会话契约。 |
| `writing-skills` | deferred | future local skill-authoring governance | 适合等本地 skill surface 进一步收缩后，再决定是否引入成正式作者手册。 |
| `brainstorming` | rejected | only the visual-companion subpattern is salvaged into `viewer-visual-companion-pilot-followup` | skill 自带“任何创意工作都必须先用”的强门禁，和当前直接执行节奏冲突。 |
| `subagent-driven-development` | rejected | none | 默认 fresh subagent-per-task + 双阶段 review 与当前显式 `spawn_agent` 语义、GitHub PR 默认边界冲突。 |
| `test-driven-development` | rejected | none | universal TDD 不适合当前 `test_tier_required/full`、文档治理和脚本任务的实际粒度。 |
| `writing-plans` | rejected | repo-owned `File Structure / Affected Paths` + handoff atomic-step templates + planning self-checklist | skill 本体仍不能升成默认前置，但其结构化拆分纪律已被限域翻译为当前 planning surface。 |
| `using-superpowers` | rejected | none | 外部 bootstrap 不能取代当前 `AGENTS.md + .pm + GitHub PR review` 主链。 |
- Acceptance Criteria:
  - AC-1: 专题必须明确写出 `superpowers` 当前 `main` 分支 skill inventory 的 adopted / rejected / deferred 清单，且每项都带 rationale 与 oasis7 mapping。
  - AC-2: adopted 项至少形成三条正式 follow-up：workflow behavior eval harness、completion-claim verification gate、Viewer visual companion pilot；其中 `verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch` 允许进一步落为本地 repo-owned skills。
  - AC-3: rejected 项必须显式覆盖与 oasis7 当前默认流程冲突的三类外部规则：强制 brainstorming gate、默认 fresh subagent+两轮 review、无条件 universal TDD。
  - AC-4: deferred 项必须把 multi-harness pluginization 与自动 skill bootstrap 维持在“非当前默认流程”边界，不得混入 root `AGENTS.md` 现行口径。
  - AC-5: `engineering` 根入口、主项目、文件级索引和 `world-simulator` Viewer 后续参考口径必须完成回写。
  - AC-6: 本专题不得直接修改当前默认 owner/review/task truth；所有 adopted 项只允许以 repo-owned follow-up 继续推进。
  - AC-7: `writing-plans` 的可 salvage 部分必须被收口成 repo-owned planning surface，而不是继续停留在“以后可以借”的抽象结论。
- Non-Goals:
  - 不把 `superpowers` 或其他外部 workflow repo 直接接入为 oasis7 当前默认 bootstrap。
  - 不在本期修改 `AGENTS.md` 的主链路为“brainstorming first”或“subagent-first”。
  - 不让 visual companion 替代 `agent-browser`、repo-owned UI regression 或 GitHub PR review。
  - 不在本期实现 multi-harness plugin packaging。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - repo-owned shell/PM helpers：承接 adopted 的 workflow closeout、PR preflight、review-thread closeout 与 future completion gate。
  - `agent-browser`：仅服务于 adopted 的 visual companion pilot 和现有 Web 回归，不作为默认万能工作流。
  - workflow behavior eval fixtures：后续用于证明 agent 在真实回合中是否遵守规则。
- Evaluation Strategy:
  - 以 repo-owned eval/smoke 验证 adopted workflow rules 是否被 agent 实际执行。
  - 以 targeted fresh verification checks 验证 completion claims 是否具备足够证据。
  - 以 Viewer Web 专题前置设计样例验证 visual companion 是否真能降低结构/视觉分歧，而不是只增加 ceremony。

## 4. Technical Specifications
- Architecture Overview:
  - 本专题只负责“借鉴边界”和“follow-up mapping”，不直接改写当前 workflow 入口。
  - adopted 项统一通过 repo-owned helper、eval、smoke 或模块专题 follow-up 落地；rejected 项明确写入 guardrail；deferred 项保持在 backlog，不进入默认主链。
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
  - visual companion 被误升级为所有需求的 mandatory pre-step：必须回退到 optional 设计辅助边界。
  - completion verification gate 只验证部分命令或旧结果：视为无效 evidence，不得宣称完成。
  - workflow eval 只验证静态文案而不验证 agent 行为：视为 coverage 不足，不得声称 adopted 项已经落地。
  - planning surface 只有“多写了一个段落”，但没有把验证命令、预期结果或命名漂移写清：视为借到了形式，没借到执行纪律。
- Non-Functional Requirements:
  - NFR-AWB-1: 借鉴矩阵中的每一条 adopted / rejected / deferred 结论都必须可通过正式文档回放。
  - NFR-AWB-2: adopted 项不得引入新的在线依赖、外部真值或多 harness bootstrap 作为当前默认前提。
  - NFR-AWB-3: workflow behavior eval 的首批覆盖必须至少命中 task-worktree、closeout、PR preflight、review-thread closeout 四段主链。
  - NFR-AWB-4: visual companion pilot 不得增加 world-simulator Viewer 默认 required gate 的在线依赖。
  - NFR-AWB-5: planning surface tightening 不得要求额外在线依赖、外部 bootstrap 或第二套 plan storage；所有新增约束必须落在现有 repo-owned 文档和模板里。
- Security & Privacy:
  - 外部 workflow 借鉴只保留结构化治理结论与公开来源链接，不导入第三方服务或隐式权限提升。
  - adopted 的 completion gate 必须继续遵守当前仓库的显式 owner、review、task traceability 规则。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 建立本专题三件套，冻结 `superpowers` 的 adopted / rejected / deferred 边界，并回写 engineering / Viewer 参考口径。
  - v1.1: 实施 workflow behavior eval harness，优先覆盖主链 workflow helpers。
  - v1.2: 实施 completion-claim verification gate，建立 repo-owned helper/checklist/smoke。
  - v1.3: 在下一轮 Viewer Web 结构/视觉专题中试点 visual companion，验证其作为 optional ideation layer 的收益。
  - v1.4: 将 `writing-plans` 的结构化拆分纪律翻译成 repo-owned planning surface，补齐 `project.md` affected-paths、handoff atomic steps 和 lightweight self-check。
  - v2.0: 在 repo-owned behavior/eval 稳定后，再决定是否重开 multi-harness workflow packaging 评估。
- Technical Risks:
  - 风险-1: 若只冻结 adopted 项、不补 repo-owned eval，最终会退化成“又一份 workflow 口号”。
  - 风险-2: 若不明确 rejected 项，外部 repo 的强制 ceremony 容易被误当成当前默认流程。
  - 风险-3: 若 visual companion 没有严格限域，可能把 Viewer 设计题和一般实现题混成统一前置门禁。
  - 风险-4: 若 packaging 先于 repo-owned truth 稳定，会造成“可分发但不可审计”的反向漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-AWB-001 | `agent-workflow-borrowing-governance`、`workflow-behavior-eval-harness-followup` | `test_tier_required` + `test_tier_full` | adopted/rejected/deferred 矩阵、repo-owned agent behavior eval、主链 workflow helper 回放 | `engineering` workflow 主链、agent 行为一致性 |
| PRD-ENGINEERING-AWB-002 | `completion-claim-verification-followup` | `test_tier_required` | fresh verification claim checklist/helper/smoke、失败签名与阻断文案 | task closeout、PR preflight、QA 报告口径 |
| PRD-ENGINEERING-AWB-003 | `viewer-visual-companion-pilot-followup` | `test_tier_required` | Viewer Web 前置 mockup/IA 对比样例、实现 task handoff、后续 `agent-browser`/repo-owned regression 不回退 | `world-simulator/viewer` 设计前置链路 |
| PRD-ENGINEERING-AWB-004 | `multi-harness-workflow-packaging-deferred` | `test_tier_required` | 仅验证 deferred 口径与 reopen 条件是否写清 | pluginization / harness distribution 边界 |
| PRD-ENGINEERING-AWB-005 | `workflow-planning-surface-tightening` | `test_tier_required` | `AGENTS.md` 规则、handoff 模板、planning self-checklist、topic/root project 回写与文档治理校验 | `engineering` planning / handoff / review 准备链路 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-AWB-001 | 只借鉴结构化 workflow patterns，并把 adopted 项转成 repo-owned follow-up | 直接把 `superpowers` 当默认 bootstrap 或当前流程真值 | oasis7 当前真值已经是 `AGENTS.md + .pm + GitHub PR review`，不能再平行引入第二条主链。 |
| DEC-AWB-002 | 采用 repo-owned workflow behavior eval 作为首批落地点 | 只更新文档口径，不验证 agent 实际是否遵守 | 这类借鉴的真正风险在于“文档和真实 agent 行为脱钩”。 |
| DEC-AWB-003 | 采用 fresh verification before completion claim 的治理方向 | 继续允许 owner 用旧结果、局部结果或 agent 自报成功来宣称完成 | 当前仓库已经高度依赖 evidence-first 收口，这项借鉴与现有方向一致且补短板明显。 |
| DEC-AWB-004 | visual companion 只作为 Viewer 等 UI-heavy 设计题的 optional ideation layer | 把 browser ideation 升级成所有实现题的 universal gate | oasis7 用户指令风格偏直接执行，强制设计门禁会与现有节奏冲突。 |
| DEC-AWB-005 | 将 multi-harness pluginization 保持 deferred | 在 repo-owned eval 稳定前立即推进 Codex/OpenCode packaging | 分发形态不应跑在治理真值前面。 |
| DEC-AWB-006 | 明确拒绝 universal brainstorming / fresh subagent-per-task / universal TDD 三类默认规则 | 以“外部方法论更完整”为理由整体照搬 | 这些规则与 oasis7 当前的 owner 授权、spawn 语义、测试分层和用户操作节奏存在直接冲突。 |
| DEC-AWB-007 | 将 `writing-plans` 的可 salvage 部分限域翻译为 `project.md` affected paths、handoff atomic steps 和 lightweight self-check | 继续把 `writing-plans` 整体维持在“只有理论价值”的 rejected 状态，或反向把它升成新的默认计划入口 | 这样既保留执行纪律的增益，又不引入第二套计划真值。 |

## PRD 自审（按 `.agents/skills/prd/check.md`）
- 目标与背景（Why 层）:
  - ✔ 是否明确说明本期解决什么问题：已明确“外部 workflow 借鉴边界不清”这一治理缺口。
  - ✔ 是否定义成功指标：SC-1~SC-5 已量化 adopted/rejected/deferred、repo-owned mapping 和边界约束。
- 用户与场景（Who / When）:
  - ✔ 是否明确目标用户与场景：producer、agent、QA、viewer 均已定义。
- 范围定义（Scope Control）:
  - ✔ 是否列出本期功能清单：借鉴矩阵、workflow eval、completion gate、visual companion、deferred packaging 已覆盖。
  - ✔ 是否明确 Out of Scope：未把外部 bootstrap、universal gate、pluginization 实施纳入本期。
- 功能规格（What）:
  - ✔ 是否定义动作、状态、权限和 follow-up 映射：规格矩阵已覆盖。
- 异常与边界（Edge Cases）:
  - ✔ 是否覆盖 adopted 无落地、visual companion 越界、partial verification 等关键风险：已覆盖。
- 非功能需求（NFR）:
  - ✔ 是否定义可审计、无新增真值、主链覆盖等约束：NFR-AWB-1~4 已覆盖。
- 可测试性（Testability）:
  - ✔ 是否给出 traceability、验证方法与回归范围：第 6 节已覆盖。
- 结论:
  - 🟢 Ready
