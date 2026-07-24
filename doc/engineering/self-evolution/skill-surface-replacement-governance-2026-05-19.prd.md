# oasis7：Skill Surface 替换治理（2026-05-19）

- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`

审计轮次: 1

> Current boundary: this PRD records the 2026-05/06 skill inventory
> rationalization decision. Current default skill reachability is governed by
> `doc/engineering/workflow/source-of-truth.md#12-specialist-skill-reachability`:
> `.agents/skills/` contains default-loadable workflow entrypoints, while root
> `skills/` contains non-default professional library material unless promoted
> through source-of-truth first.

## 目标
- 冻结 2026-05/06 当时本地 skill inventory 的 `keep / replace / retire / defer` 边界。
- 退役与 repo-native 指令或当前默认工作流冲突的低耦合 skill surface。
- 确保角色卡与 engineering 根入口只推荐当前仍应保留的 skill。

## 范围
- 覆盖 2026-05/06 本地 skill inventory 的治理分桶；当前默认入口与
  非默认专业库分层以后续 source-of-truth 为准。
- 覆盖首批低耦合 skill surface 的文件级退役与角色卡回写。
- 首轮不覆盖全部 generic game-skill mirror 的一次性清理；2026-06-20 follow-up 已完成该 watch bucket 的治理裁定与局部收口。

## 接口 / 数据
- default workflow skill 入口: `.agents/skills/*/SKILL.md`
- non-default specialist library 入口: `skills/*/SKILL.md`
- 角色职责入口: `.agents/roles/*.md`
- 工程治理入口: `doc/engineering/prd.md`
- 项目执行入口: `doc/engineering/project.md`

## 里程碑
- M1 (2026-05-19): 建立 skill rationalization 专题三件套并冻结首批 keep/replace/retire/defer 矩阵。
- M2: 退役首批低耦合 skill surface，并清理角色卡与活跃文档引用。
- M3 (2026-06-20): 完成 generic game-skill mirror watch bucket follow-up；`asset-optimization`、`audio-systems`、`monetization-systems` 退成本地上游跟踪决策，`game-design-theory`、`synchronization-algorithms` 收窄触发，`level-design`、`particle-systems` 保留为 domain-triggered non-default surface。

## 风险
- 若角色卡未同步回写，删除 skill 后会留下悬空推荐。
- 若一次性删除过多 generic skill，容易扩大角色卡和文档回写范围。

## 1. Executive Summary
- Problem Statement: 2026-05/06 当时 `.agents/skills/` 同时混有 repo-native 基础设施 skill、通用方法论 skill、以及从外部来源直接镜像的游戏通用 skill。若不先冻结哪些保留、哪些替换、哪些退役，角色卡与仓库真值会继续引用低耦合甚至与当时流程冲突的 skill surface。
- Proposed Solution: 在 `engineering/self-evolution` 下建立正式专题，先按 `keep / replace / retire / defer` 四态冻结 2026-05/06 当时 skill inventory，再优先退役一批与 repo-native 指令、当时文档组织或默认工作流冲突的低耦合 skill，并同步回写角色卡与工程入口。
- Success Criteria:
  - SC-1: 2026-05/06 当时本地 skill inventory 中的 skill 至少完成一轮 `keep / replace / retire / defer` 归类，并为每项给出 repo-specific 理由。
  - SC-2: 至少一批低耦合、纯通用、与 2026-05/06 当时仓库默认流程冲突的本地 skill surface 被正式退役，且角色卡/活跃文档不再残留悬空引用。
  - SC-3: `agent-browser`、`prd` 等专业方法 skill，以及 `xiaohongshu-note-analyzer` 这类渠道内容参考，均以 root `skills/` 非默认 library 口径写清保留边界；只有 workflow gate 继续作为 `.agents/skills/` default entrypoint，不与通用 skill 混为同类。
  - SC-4: 外部/上游 skill 的借鉴必须继续服从 `worktree -> .pm -> PRD/project -> tests -> GitHub PR` 单一主链，不得因为“skill 更完整”而引入第二套默认流程。
  - SC-5: `writing-skills` 的可 salvage 部分必须被翻译成 repo-owned skill authoring surface，包括本地 authoring entrypoint、template、checklist 与 bounded borrowing 说明，而不是继续停留在泛化 deferred。
  - SC-6: 若某个 upstream workflow skill 的可借部分已经被正式收口为 repo-owned local skill，则 `.agents/skills` inventory、README 入口与治理文档必须同步把它记录为已保留的 repo-owned surface，而不是继续停留在“仅 borrowing 文档存在”的悬空状态。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要判断 2026-05/06 当时 skill surface 哪些是真正的仓库资产，哪些只是历史镜像或低价值 persona。
  - `viewer_engineer` / `qa_engineer` / `liveops_community`: 需要角色卡里的推荐技能仍然可用、且与当时实际工作流一致。
  - 仓库维护者: 需要减少“看似存在但其实不该默认使用”的 skill 噪音。
- User Stories:
  - PRD-ENGINEERING-032: As a repo workflow owner, I want the 2026-05/06 local skill inventory frozen into keep/replace/retire/defer buckets, so that role cards and workflow docs only recommend skills that still match oasis7 truth.
  - PRD-ENGINEERING-032A: As a role owner, I want low-coupling generic skills retired when repo-native instructions already cover the same ground, so that recommended skills no longer point at stale or conflicting surfaces.
  - PRD-ENGINEERING-032B: As a maintainer, I want generic upstream skills either replaced by repository-owned guidance or explicitly deferred, so that local maintenance cost does not grow faster than repo-specific value.
  - PRD-ENGINEERING-032C: As a maintainer, I want a repo-owned skill authoring surface for `.agents/skills`, so that future local skills follow consistent trigger wording, template structure, and verification rules without importing upstream workflow wholesale.
  - PRD-ENGINEERING-032D: As a workflow owner, I want bounded borrowed workflow patterns that survive governance to land as repo-owned local skills, so that roles can trigger them directly without re-reading the full borrowing topic.
  - PRD-ENGINEERING-032E: As a workflow owner, I want the repo-owned workflow-router skill kept in the 2026-05/06 local inventory as the default entrypoint for non-trivial workflow chaining, so that upstream `using-superpowers` routing value is preserved without importing its external bootstrap.
- Critical User Flows:
  1. `盘点 2026-05/06 当时 .agents/skills inventory -> 读取角色卡/工程入口/活跃文档引用 -> 判断 skill 是否 repo-owned、generic-but-compatible、generic-and-conflicting`
  2. `对每个 skill 冻结 keep / replace / retire / defer -> 只对 low-coupling retire/replacement 执行首轮文件面收口 -> generic game-skill mirror follow-up 在 2026-06-20 收口为 retired-to-upstream-tracking / trigger-narrowing / domain-triggered non-default`
  3. `若 skill 被 retire -> 同步更新角色卡、活跃文档与工程入口 -> 复跑文档/PM 门禁，确保没有残留悬空引用`
  4. `若 upstream skill 只适合局部借鉴 -> 先抽出 repo-owned authoring/template/checklist surface -> 明确哪些内容 adopted、哪些仍 deferred/rejected -> 再把当时本地 skill inventory 和角色卡接回真值`
  5. `若 upstream workflow routing 被收口为 repo-owned local skill -> 将该 skill 接回 README / AGENTS phase order / topic project trace -> 明确“保留的是本地入口，不是上游 bootstrap”`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Skill inventory matrix | `skill_name`、`bucket=keep|replace|retire|defer`、`rationale`、`replacement_surface`、`followup_ref` | 对 2026-05/06 当时本地 skill 逐项归类，只有 `retire`/`replace` 才允许进入文件改动 | `unreviewed -> keep|replace|retire|defer` | 先按是否 repo-owned，再按与当时 workflow 冲突程度排序 | `producer_system_designer` 冻结结论，相关 owner 参与联审 |
| Low-coupling retirement | `local_skill_path`、`role_refs`、`active_doc_refs`、`replacement_surface` | 删除或退役低耦合 skill surface，并同步角色卡/文档引用 | `planned -> retired` | 优先处理无脚本依赖、无代码耦合、引用面最小者 | engineering owner 执行，相关 role 卡同步 |
| Deferred upstream mirrors | `skill_name`、`upstream_source`、`reason_deferred` | 首轮暂不删除，只记录其 generic mirror 身份与未来替换条件；2026-06-20 follow-up 已把原 generic game-skill mirror watch bucket 裁定为 retired-to-upstream-tracking / trigger-narrowing / domain-triggered non-default | `unreviewed -> deferred -> retired|maintain|preserve-non-default` | 优先保留高耦合、批量删除成本大的 generic mirrors；follow-up 以真实引用面和 repo-specific 增量重新裁定 | `producer_system_designer` 决定 reopen 时机，相关 domain role 参与复核 |
| Skill authoring surface | `authoring_entrypoint`、`template_path`、`checklist_path`、`bounded_borrowing_note` | 将 upstream `writing-skills` 中可复用的 authoring discipline 翻译成 repo-owned 入口、模板、自检清单与本地 skill | `deferred idea -> repo-owned surface` | 先保 trigger wording 和结构纪律，再决定是否需要更强验证机制 | `producer_system_designer` 冻结边界，skill maintainer 执行 |
- Acceptance Criteria:
  - AC-1: 2026-05/06 当时 skill inventory 中必须明确写出至少一批 `retire` 项，并给出对应 replacement surface。
  - AC-2: 本轮至少完成 1 组以上低耦合 skill surface 的正式退役，并清理角色卡中的直接引用。
  - AC-3: `verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch`、`tdd-test-writer`、`bounded-brainstorming` 等 workflow gate 的 default-surface 保留理由都必须显式记录为 repo-owned；`agent-browser`、`prd`、`gpt-image-2`、`humanizer-zh`、`xiaohongshu-note-analyzer` 等专业/渠道方法材料保留为 root `skills/` 非默认 library reference。
  - AC-4: 对 generic game-skill 镜像簇若未首轮删除，必须标记为 `defer` 并说明“为何先不动”；2026-06-20 follow-up 完成后不得继续把已裁定项保留为悬空 defer。
  - AC-5: 本轮必须为 `.agents/skills` 增加 repo-owned authoring surface，至少包含本地 skill、template、checklist 与入口说明，并明确 upstream `writing-skills` 哪些部分仍未采纳。
  - AC-6: `repo-owned-workflow-router` 必须被显式记录为保留的 repo-owned workflow surface，并在 `.agents/skills/README.md`、root `AGENTS.md` 和相关治理文档中保持同一默认 phase-order 口径。
- Non-Goals:
  - 不在首轮重写全部 generic game-skill 内容；后续 watch bucket 收口必须通过单独 task / role review / PR gate 完成。
  - 不把所有 skill 能力迁回系统提示词。
  - 不改变 `agent-browser`、`.pm`、`prepare-task-pr` 等现有 repo-owned workflow 主链。

## 3. Technical Specifications
- Architecture Overview:
  - 本专题只治理本地 skill surface 的保留/退役/替换边界，不引入新的技能运行时。
  - repo-owned skill 与 generic mirror skill 必须分开处理：前者强调工作流/脚本/平台依赖，后者强调是否仍值得本地维护。
- Integration Points:
  - `.agents/skills/*/SKILL.md`
  - `.agents/skills/README.md`
  - `.agents/skills/templates/SKILL.template.md`
  - `.agents/skills/checklists/skill-authoring-checklist.md`
  - `.agents/roles/*.md`
  - `AGENTS.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/project.md`
  - `doc/site/prd.md`
- Edge Cases & Error Handling:
  - 若 skill 已被删，但角色卡仍引用：视为未完成。
  - 若活跃文档仍把被退役 skill 当正式方法入口：视为未完成。
  - 若 generic skill 虽然通用，但承载了 repo-specific 脚本/路径/平台约束：不得直接按低耦合删除；需要先判断保留为 root `skills/` 非默认 library，还是经 source-of-truth 显式提升为 `.agents/skills/` wrapper。
  - 若把 upstream `writing-skills` 的 subagent / TDD 部分整体抬进来，导致 skill authoring 与当前主链竞争：视为越界。
- Non-Functional Requirements:
  - NFR-1: 本轮 skill rationalization 不得引入新的 repo root 文档平铺或第二套 workflow 真值。
  - NFR-2: `retire` 决策必须优先落到低耦合 skill surface，避免大规模删除导致角色卡与文档同步失配。
  - NFR-3: 角色卡推荐 skill 列表必须只引用当前仓库中仍存在且推荐的 skill。
  - NFR-4: repo-owned skill authoring surface 不得要求外部安装步骤、额外在线依赖或与 oasis7 无关的 deployment 说明。

## 4. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-032 | `skill-replacement-rationalization` | `test_tier_required` | inventory matrix、角色卡/活跃文档引用清理、`doc-governance-check`、`pm-lint`、`git diff --check` | `.agents/skills`、`.agents/roles`、engineering 根入口 |
| PRD-ENGINEERING-032C | `skill-authoring-surface-tightening` | `test_tier_required` | 本地 skill authoring skill、template、checklist、README、topic/root project 回写与治理校验 | `.agents/skills`、角色卡、skill 治理专题 |
| PRD-ENGINEERING-032D | `brainstorming-skill-boundary-reconciliation`、`tdd-skill-boundary-reconciliation` | `test_tier_required` | bounded borrowed workflow skill、本地 trigger/README、topic/root project 与 borrowing design 文档对齐 | `.agents/skills`、workflow skill surface、engineering 治理专题 |
| PRD-ENGINEERING-032E | `workflow-router-skill-reconciliation` | `test_tier_required` | `repo-owned-workflow-router`、`.agents/skills/README.md`、root `AGENTS.md` phase order、borrowing design / skill-surface 文档与 `.pm` trace 对齐 | `.agents/skills`、workflow entrypoint surface、engineering 治理专题 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SKILL-001 | 先退役低耦合通用 skill，再以 follow-up 处理 generic game-skill 镜像簇 | 一次性批量删除全部 generic skill | 引用面和角色卡同步风险太高，应先做低风险收缩；2026-06-20 follow-up 已完成原 watch bucket 裁定。 |
| DEC-SKILL-002 | repo-owned workflow gate 继续保留为 `.agents/skills/` default entrypoint；专业/场景方法 skill 转入 root `skills/` 非默认 library | 统一要求所有 skill 都改成上游安装，或让所有场景材料都默认触发 | `agent-browser`、`prd`、`gpt-image-2`、`humanizer-zh` 等仍有 repo-specific 方法价值，但不应作为默认 workflow trigger；`xiaohongshu-note-analyzer` 更适合作为显式 opt-in 渠道内容参考。 |
| DEC-SKILL-003 | 对与当前默认流程冲突的通用 skill 直接 retire | 保留 skill 但继续在角色卡中推荐 | 会继续制造“存在即推荐”的误导。 |
| DEC-SKILL-004 | 对已在 borrowing 专题中裁定为可借鉴的 skill，默认落为 root `skills/` 非默认 library；只有 repo-owned workflow gate 或 source-of-truth-promoted wrapper 才进入 `.agents/skills/` | 只在 borrowing 文档里记录 adopted 结论，不把 skill surface 真正落盘 | 若 adopted 项始终没有仓库内可引用 surface，角色层就无法稳定复用；若专业方法默认进入 `.agents/skills/`，又会扩大默认触发面。 |
| DEC-SKILL-005 | 将 `writing-skills` 只收敛为 repo-owned authoring surface，不引入其完整 TDD/subagent gate | 要么完全不借，要么整套照搬 upstream skill | 当前真正缺的是本地 skill 作者入口与结构纪律，不是再造一条与主链竞争的 skill deployment 流程。 |
| DEC-SKILL-006 | 将 `using-superpowers` 里唯一值得保留的 process-skill routing order 收口为本地 `repo-owned-workflow-router` skill | 完全不落本地 skill，或把 `using-superpowers` 整体 bootstrap 直接保留 | 当前角色真正需要的是本地 workflow 总入口；若不落本地 skill，路由只会继续散落在 borrowing 说明里；若整体保留 upstream bootstrap，又会重新制造第二套默认流程真值。 |
| DEC-SKILL-007 | `content-creation` 经 2026-06-20 维护项转为 oasis7 liveops/community/channel copy aid 后，从 `skills-lock.json` 外部镜像追踪中移除 | 继续把已本地化 entrypoint 作为 `anthropics/knowledge-work-plugins` mirror 锁定 | `.agents/skills/README.md` 规定 repo-owned skill 不写入 `skills-lock.json`；继续锁定会让未来同步误判或覆盖本地角色边界。 |

## 结论
- 🟢 Ready
