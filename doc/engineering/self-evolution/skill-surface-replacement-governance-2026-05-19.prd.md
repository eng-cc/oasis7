# oasis7：Skill Surface 替换治理（2026-05-19）

- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`
- 对应项目管理文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`

审计轮次: 1

## 目标
- 冻结当前 `.agents/skills/` inventory 的 `keep / replace / retire / defer` 边界。
- 退役与 repo-native 指令或当前默认工作流冲突的低耦合 skill surface。
- 确保角色卡与 engineering 根入口只推荐当前仍应保留的 skill。

## 范围
- 覆盖 `.agents/skills/` 当前本地 inventory 的治理分桶。
- 覆盖首批低耦合 skill surface 的文件级退役与角色卡回写。
- 不覆盖全部 generic game-skill mirror 的一次性清理。

## 接口 / 数据
- skill inventory 入口: `.agents/skills/*/SKILL.md`
- 角色职责入口: `.agents/roles/*.md`
- 工程治理入口: `doc/engineering/prd.md`
- 项目执行入口: `doc/engineering/project.md`

## 里程碑
- M1 (2026-05-19): 建立 skill rationalization 专题三件套并冻结首批 keep/replace/retire/defer 矩阵。
- M2: 退役首批低耦合 skill surface，并清理角色卡与活跃文档引用。
- M3: 视维护成本继续评估 generic game-skill mirror 簇是否转成上游跟踪清单。

## 风险
- 若角色卡未同步回写，删除 skill 后会留下悬空推荐。
- 若一次性删除过多 generic skill，容易扩大角色卡和文档回写范围。

## 1. Executive Summary
- Problem Statement: 当前 `.agents/skills/` 同时混有 repo-native 基础设施 skill、通用方法论 skill、以及从外部来源直接镜像的游戏通用 skill。若不先冻结哪些保留、哪些替换、哪些退役，角色卡与仓库真值会继续引用低耦合甚至与当前流程冲突的 skill surface。
- Proposed Solution: 在 `engineering/self-evolution` 下建立正式专题，先按 `keep / replace / retire / defer` 四态冻结当前 skill inventory，再优先退役一批与 repo-native 指令、当前文档组织或默认工作流冲突的低耦合 skill，并同步回写角色卡与工程入口。
- Success Criteria:
  - SC-1: 当前 `.agents/skills/` inventory 中的 skill 至少完成一轮 `keep / replace / retire / defer` 归类，并为每项给出 repo-specific 理由。
  - SC-2: 至少一批低耦合、纯通用、与当前仓库默认流程冲突的本地 skill surface 被正式退役，且角色卡/活跃文档不再残留悬空引用。
  - SC-3: `agent-browser`、`prd`、`xiaohongshu*` 等 repo-owned 或明确场景专属 skill 的保留边界被显式写清，不与通用 skill 混为同类。
  - SC-4: 外部/上游 skill 的借鉴必须继续服从 `worktree -> .pm -> PRD/project -> tests -> GitHub PR` 单一主链，不得因为“skill 更完整”而引入第二套默认流程。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要判断当前 skill surface 哪些是真正的仓库资产，哪些只是历史镜像或低价值 persona。
  - `viewer_engineer` / `qa_engineer` / `liveops_community`: 需要角色卡里的推荐技能仍然可用、且与当前实际工作流一致。
  - 仓库维护者: 需要减少“看似存在但其实不该默认使用”的 skill 噪音。
- User Stories:
  - PRD-ENGINEERING-032: As a repo workflow owner, I want the current local skill inventory frozen into keep/replace/retire/defer buckets, so that role cards and workflow docs only recommend skills that still match oasis7 truth.
  - PRD-ENGINEERING-032A: As a role owner, I want low-coupling generic skills retired when repo-native instructions already cover the same ground, so that recommended skills no longer point at stale or conflicting surfaces.
  - PRD-ENGINEERING-032B: As a maintainer, I want generic upstream skills either replaced by repository-owned guidance or explicitly deferred, so that local maintenance cost does not grow faster than repo-specific value.
- Critical User Flows:
  1. `盘点当前 .agents/skills inventory -> 读取角色卡/工程入口/活跃文档引用 -> 判断 skill 是否 repo-owned、generic-but-compatible、generic-and-conflicting`
  2. `对每个 skill 冻结 keep / replace / retire / defer -> 只对 low-coupling retire/replacement 执行本轮文件面收口 -> 其余高耦合 generic mirror 先保留并记录 deferred`
  3. `若 skill 被 retire -> 同步更新角色卡、活跃文档与工程入口 -> 复跑文档/PM 门禁，确保没有残留悬空引用`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Skill inventory matrix | `skill_name`、`bucket=keep|replace|retire|defer`、`rationale`、`replacement_surface`、`followup_ref` | 对当前本地 skill 逐项归类，只有 `retire`/`replace` 才允许进入文件改动 | `unreviewed -> keep|replace|retire|defer` | 先按是否 repo-owned，再按与当前 workflow 冲突程度排序 | `producer_system_designer` 冻结结论，相关 owner 参与联审 |
| Low-coupling retirement | `local_skill_path`、`role_refs`、`active_doc_refs`、`replacement_surface` | 删除或退役低耦合 skill surface，并同步角色卡/文档引用 | `planned -> retired` | 优先处理无脚本依赖、无代码耦合、引用面最小者 | engineering owner 执行，相关 role 卡同步 |
| Deferred upstream mirrors | `skill_name`、`upstream_source`、`reason_deferred` | 暂不删除，只记录其 generic mirror 身份与未来替换条件 | `unreviewed -> deferred` | 优先保留高耦合、批量删除成本大的 generic mirrors | `producer_system_designer` 决定 reopen 时机 |
- Acceptance Criteria:
  - AC-1: 当前 skill inventory 中必须明确写出至少一批 `retire` 项，并给出对应 replacement surface。
  - AC-2: 本轮至少完成 1 组以上低耦合 skill surface 的正式退役，并清理角色卡中的直接引用。
  - AC-3: `agent-browser`、`prd`、`xiaohongshu`、`xiaohongshu-note-analyzer`、`gpt-image-2`、`humanizer-zh` 的保留理由必须显式记录为 repo-owned 或明确场景专属。
  - AC-4: 对 generic game-skill 镜像簇若未本轮删除，必须标记为 `defer` 并说明“为何先不动”。
- Non-Goals:
  - 不在本轮重写全部 generic game-skill 内容。
  - 不把所有 skill 能力迁回系统提示词。
  - 不改变 `agent-browser`、`.pm`、`prepare-task-pr` 等现有 repo-owned workflow 主链。

## 3. Technical Specifications
- Architecture Overview:
  - 本专题只治理本地 skill surface 的保留/退役/替换边界，不引入新的技能运行时。
  - repo-owned skill 与 generic mirror skill 必须分开处理：前者强调工作流/脚本/平台依赖，后者强调是否仍值得本地维护。
- Integration Points:
  - `.agents/skills/*/SKILL.md`
  - `.agents/roles/*.md`
  - `AGENTS.md`
  - `doc/engineering/README.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/project.md`
  - `doc/site/github-pages/github-pages-game-engine-reposition-2026-02-25.prd.md`
- Edge Cases & Error Handling:
  - 若 skill 已被删，但角色卡仍引用：视为未完成。
  - 若活跃文档仍把被退役 skill 当正式方法入口：视为未完成。
  - 若 generic skill 虽然通用，但承载了 repo-specific 脚本/路径/平台约束：不得直接按低耦合删除。
- Non-Functional Requirements:
  - NFR-1: 本轮 skill rationalization 不得引入新的 repo root 文档平铺或第二套 workflow 真值。
  - NFR-2: `retire` 决策必须优先落到低耦合 skill surface，避免大规模删除导致角色卡与文档同步失配。
  - NFR-3: 角色卡推荐 skill 列表必须只引用当前仓库中仍存在且推荐的 skill。

## 4. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-032 | `skill-replacement-rationalization` | `test_tier_required` | inventory matrix、角色卡/活跃文档引用清理、`doc-governance-check`、`pm-lint`、`git diff --check` | `.agents/skills`、`.agents/roles`、engineering 根入口 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-SKILL-001 | 先退役低耦合通用 skill，再处理 generic game-skill 镜像簇 | 一次性批量删除全部 generic skill | 引用面和角色卡同步风险太高，应先做低风险收缩。 |
| DEC-SKILL-002 | repo-owned/场景专属 skill 保留，本轮不动 | 统一要求所有 skill 都改成上游安装 | `agent-browser`、`prd`、`xiaohongshu*` 等已与当前仓库工作流强绑定。 |
| DEC-SKILL-003 | 对与当前默认流程冲突的通用 skill 直接 retire | 保留 skill 但继续在角色卡中推荐 | 会继续制造“存在即推荐”的误导。 |

## 结论
- 🟢 Ready
