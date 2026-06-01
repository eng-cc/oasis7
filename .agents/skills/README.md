# Local Skill Surface

`.agents/skills/` 只收两类本地 surface：

- repo-owned workflow / helper / governance skill
- 明确场景专属、且与 oasis7 当前仓库有稳定绑定的 skill

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

## Authoring Entry Points

- 触发 skill：`.agents/skills/writing-repo-owned-skills/SKILL.md`
- 模板：`.agents/skills/templates/SKILL.template.md`
- 自检清单：`.agents/skills/checklists/skill-authoring-checklist.md`

## Domain-Specific Entry Points

- 中文长篇背景叙事与世界观资产编排：`.agents/skills/epic-story-orchestrator-zh/SKILL.md`

## Workflow Execution Entry Points

Canonical phase mapping lives in `doc/engineering/workflow/source-of-truth.md#11-skill-map-by-phase`; this README is an index, not the workflow authority.

- 启动会改变仓库状态的新 task、需要先确认标准 task worktree / `.pm` task / owner role 真值，并把后续阶段接回 repo-owned 主链时：`.agents/skills/default-workflow-bootstrap/SKILL.md`
- 启动已具备 task 真值的仓库变更 task、或不确定下一步该走哪条 repo-owned workflow surface 时：`.agents/skills/repo-owned-workflow-router/SKILL.md`
- 需求仍偏模糊、需要 scope 拆分、方案对比或判断是否需要 visual companion 时：`.agents/skills/bounded-brainstorming/SKILL.md`
- 行为变更类实现任务、且存在稳定自动化测试面时：`.agents/skills/tdd-test-writer/SKILL.md`
- 已有正式 `project.md` / handoff / `.pm` task 后进入实施：`.agents/skills/executing-project-tasks/SKILL.md`
- 遇到 bug、失败测试、脚本异常、意外 diff 或回归时：`.agents/skills/systematic-debugging/SKILL.md`
- 当前 diff 已形成 major feature、高风险收敛切片，或 commit 前 claim risk 明显偏高时：`.agents/skills/requesting-repo-owned-review/SKILL.md`
- 接近完成、准备宣称“通过 / 完成 / 可提 PR”时：`.agents/skills/verification-before-completion/SKILL.md`
- 已完成实现、准备 closeout / commit / PR 收口时：`.agents/skills/finishing-a-development-branch/SKILL.md`
- PR 收到 review comments / requested changes，需要核实、修复、回证据并保持 thread resolution 与 merge readiness 分离时：`.agents/skills/receiving-code-review/SKILL.md`
- 新增或修改本地 repo-owned skill、替换上游 skill 或调整 skill governance 时：`.agents/skills/writing-repo-owned-skills/SKILL.md`

Specialist skills are domain-triggered through TPM routing or professional subagent slice planning. They are intentionally not mandatory phases in the default workflow chain.

## Bounded Borrowing From `writing-skills`

当前只借以下部分：

- `SKILL.md` 的结构化 frontmatter / body 约束
- 更强调 trigger wording 的 description 写法
- supporting files 只在 heavy reference / reusable tools 时引入
- 发布前至少做一次 repo-owned 验证，而不是只凭自我判断

当前不直接引入：

- upstream 的 `failing test first with subagents` 作为硬性门禁
- 与 oasis7 无关的 agent-specific 安装 / 发布说明
- 任何会替代 `AGENTS.md + .pm + GitHub PR review` 主链的第二套流程

## Notes

- 外部来源同步到本地的 skill 仍按 `skills-lock.json` 追踪。
- repo-owned skill 不写入 `skills-lock.json`，其真值在仓库本身与对应治理文档里。
