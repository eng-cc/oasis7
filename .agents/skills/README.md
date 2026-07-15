# Local Skill Surface

`.agents/skills/` is the default-loadable local skill surface. It only keeps
entrypoints that should be discoverable as active repo-owned workflow skills:

- repo-owned workflow / helper / governance skill

Root `skills/` is the non-default specialist library surface for professional
method skills. Role cards may reference these skills as methods for bounded
professional slices, but they should not become automatic default triggers. A
`skills/<name>/SKILL.md` file is library material unless source-of-truth and a
new `.agents/skills/<name>/SKILL.md` wrapper promote it back into the default
surface.

不要把下面这些内容直接塞进本地 skill：

- 一次性任务复盘
- 只适用于某个专题 task 的临时约定
- 已经更适合写进 `AGENTS.md`、模块 `prd.md` / `project.md`、handoff 模板或脚本校验的内容

## 什么时候应该新建 skill

适合新建：

- 会在多个 session / 多个任务里重复复用
- 需要 repo-specific 路径、命令、helper 或 review 边界
- 仅靠系统提示或角色卡不够稳定，单独 skill 更容易被触发

不适合新建：

- 只是当前模块的一次性约定
- 完全 generic，且仓库没有 repo-specific 增量
- 可以直接靠脚本或 lint 自动强制的机械规则

## Default vs Library Surfaces

- Keep default workflow gates under `.agents/skills/<name>/SKILL.md`.
- Keep professional method skills under `skills/<name>/` and reference them
  from role cards or slice contracts.
- Do not rely on root `skills/` material for automatic workflow routing. TPM
  must explicitly opt in through task evidence or promote it back through a
  source-of-truth-first change.
- When moving a default skill into the library, update this README,
  `doc/engineering/workflow/source-of-truth.md`, role cards, and any scripts
  that referenced the old `.agents/skills` path.

## Authoring Entry Points

- 触发 skill：`.agents/skills/writing-repo-owned-skills/SKILL.md`
- 模板：`.agents/skills/templates/SKILL.template.md`
- 自检清单：`.agents/skills/checklists/skill-authoring-checklist.md`
- 静态校验：`./scripts/lint-skills.sh`

## Professional Skill Library

- 中文长篇背景叙事与世界观资产编排：`skills/epic-story-orchestrator-zh/SKILL.md`
- 产品/规划文档：`skills/prd/SKILL.md`
- 游戏架构规划：`skills/game-architect/SKILL.md`
- 游戏设计理论：`skills/game-design-theory/SKILL.md`
- 玩法机制：`skills/gameplay-mechanics/SKILL.md`
- 关卡设计：`skills/level-design/SKILL.md`
- 游戏玩家交互流程、状态反馈、输入/控制、错误恢复和可访问性审查：`skills/game-interaction-design/SKILL.md`
- 游戏 UI 视觉层级、可读性、截图审查和 visual reference workflow：`skills/game-visual-design/SKILL.md`
- 浏览器闭环：`skills/agent-browser/SKILL.md`
- GPT Image 2 / visual companion：`skills/gpt-image-2/SKILL.md`
- 中文文本自然化：`skills/humanizer-zh/SKILL.md`
- LiveOps / channel copy：`skills/content-creation/SKILL.md`
- 性能优化：`skills/optimization-performance/SKILL.md`
- 内存治理：`skills/memory-management/SKILL.md`
- 粒子 / VFX：`skills/particle-systems/SKILL.md`
- 同步算法：`skills/synchronization-algorithms/SKILL.md`
- 非默认渠道内容参考：`skills/xiaohongshu-note-analyzer/SKILL.md`

## Workflow Execution Entry Points

Canonical phase mapping lives in `doc/engineering/workflow/source-of-truth.md#11-skill-map-by-phase`; this README is an index, not the workflow authority.

GitHub Project-backed PM truth lives in `doc/engineering/workflow/source-of-truth.md#123-github-project-backed-pm-contract`: GitHub Issues + GitHub Project are active task truth, GitHub issue comments are the execution/audit sink, and `.pm/github-project-sync/` is the deterministic local mapping/archive surface for scripts.

- 启动任何用户请求、需要先确认标准 task worktree / GitHub Project-backed task truth / owner role 真值，并把后续阶段接回 repo-owned 主链时：`.agents/skills/default-workflow-bootstrap/SKILL.md`
- 只读/聊天请求也默认进入 task/worktree bootstrap；不要先用“只读/聊天/纯事实”分类决定是否跳过 bootstrap。如果要输出专业结论，进入 task truth 后仍由 TPM 派发对应 bounded 专业角色 slice。纯路径查找、命令输出复述等客观事实读取可由 TPM 在已绑定 task/worktree 内直接处理。
- 启动已具备 task 真值的仓库变更 task、或不确定下一步该走哪条 repo-owned workflow surface 时：`.agents/skills/repo-owned-workflow-router/SKILL.md`
- 评估或实现跨阶段 production supervisor target 时：`.agents/skills/tpm-production-supervisor/SKILL.md`；当前状态为 blocked，不得宣称自动恢复。
- 需求仍偏模糊、需要 scope 拆分、方案对比或判断是否需要 visual companion 时：`.agents/skills/bounded-brainstorming/SKILL.md`
- 行为变更类实现任务、且存在稳定自动化测试面时：`.agents/skills/tdd-test-writer/SKILL.md`
- 已有正式 `project.md` / handoff / GitHub-backed task truth 后进入实施：`.agents/skills/executing-project-tasks/SKILL.md`
- 遇到 bug、失败测试、脚本异常、意外 diff 或回归时：`.agents/skills/systematic-debugging/SKILL.md`
- 当前 diff 已形成 major feature、高风险收敛切片，或 commit 前 claim risk 明显偏高时：`.agents/skills/requesting-repo-owned-review/SKILL.md`
- 接近完成、准备宣称“通过 / 完成 / 可提 PR”时：`.agents/skills/verification-before-completion/SKILL.md`
- 已完成实现、准备 closeout / commit / PR 收口时：`.agents/skills/finishing-a-development-branch/SKILL.md`
- PR 收到 review comments / requested changes，需要核实、修复、回证据并保持 thread resolution 与 merge readiness 分离时：`.agents/skills/receiving-code-review/SKILL.md`
- 新增或修改本地 repo-owned skill、替换上游 skill 或调整 skill governance 时：`.agents/skills/writing-repo-owned-skills/SKILL.md`

Specialist skills are domain-triggered through TPM routing or professional subagent slice planning. They are intentionally not mandatory phases in the default workflow chain. TPM routing is coordination only; specialist conclusions must be owned by the matching professional role slice. Professional slice contracts record intended model, actual dispatched model/reasoning or `inherited/unverified`, context delivery mode, and mandatory context checklist; default context delivery is a minimal task packet bound to the task UID and current/frozen HEAD, while full-thread/full-history delivery requires a recorded escalation reason.

Non-default specialist library material under root `skills/` is opt-in
reference material. It can support a professional slice, but it is not a default
trigger and must not be treated as active task truth without task evidence.

## Bounded Borrowing From `writing-skills`

当前只借以下部分：

- `SKILL.md` 的结构化 frontmatter / body 约束
- 更强调 trigger wording 的 description 写法
- supporting files 只在 heavy reference / reusable tools 时引入
- 发布前至少做一次 repo-owned 验证，而不是只凭自我判断

## Skill Hygiene Gates

- `SKILL.md` 是入口，不是整本手册；超过 lint 阈值的详细说明、示例、命令矩阵应移入 `references/`、`scripts/`、`assets/`、`templates/` 等 supporting files。
- `description` 必须以 `Use when` 开头，只描述触发条件。
- entrypoint 中列出的 supporting files 必须真实存在。
- `scripts/` 下的 placeholder helper 必须被 `SKILL.md` 或直接引用的 reference 明确承接；未被承接的 placeholder helper 进入 skill-surface 退役候选，不作为“存在即保留”的默认资产。`assets/`、`templates/` 与重型 reference material 的治理仍按对应 skill entrypoint 和专题 follow-up 单独裁定。
- 核心 workflow skill 必须保留 `Known Failure Modes`，把反复踩过的流程坑写进入口。
- Root `skills/` library entries should keep valid frontmatter when they use
  `SKILL.md`, but they are checked as library material and are not required to
  satisfy default workflow-gate expectations.
- 修改本地 skill surface 后运行 `./scripts/lint-skills.sh`。

当前不直接引入：

- upstream 的 `failing test first with subagents` 作为硬性门禁
- 与 oasis7 无关的 agent-specific 安装 / 发布说明
- 任何会替代 `AGENTS.md + GitHub-backed task truth + GitHub PR review` 主链的第二套流程

## Notes

- 外部来源同步到本地的 skill 仍按 `skills-lock.json` 追踪。
- repo-owned skill 不写入 `skills-lock.json`，其真值在仓库本身与对应治理文档里。
