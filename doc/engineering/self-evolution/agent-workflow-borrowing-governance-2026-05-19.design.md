# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）设计

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`

审计轮次: 1

## 1. 设计定位
本专题负责把外部 agent workflow 方法论转成 oasis7 可审计的治理输入，而不是把第三方 repo 直接接进当前默认流程。首批样本是 `obra/superpowers`，但产出必须是 repo-owned 的 adopted / rejected / deferred 决策和后续任务，不是对外部 skill 文案的镜像复制。

## 2. 结构分层
### 2.1 决策层
- `prd.md`：冻结哪些模式 adopted、哪些 rejected、哪些 deferred。
- `project.md`：把 adopted 项拆成 repo-owned follow-up，明确 owner、测试层级与依赖。
- 本设计文档：说明为什么这样分层，以及 adopted 项如何接回当前仓库主链。

### 2.2 当前主链不变
外部借鉴不得替换当前默认执行链：
`new-task-worktree -> workflow-report start -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR review/approval -> review-thread closeout`

因此本专题只允许两种输出：
1. 作为 repo-owned helper/eval/smoke 的 follow-up。
2. 作为某个模块专题的 optional design technique reference。

## 3. 决策矩阵
### 3.0 `superpowers` 当前 skill inventory 快照
当前 `main` 分支可见 skill 为：
`brainstorming`、`dispatching-parallel-agents`、`executing-plans`、`finishing-a-development-branch`、`receiving-code-review`、`requesting-code-review`、`subagent-driven-development`、`systematic-debugging`、`test-driven-development`、`using-git-worktrees`、`using-superpowers`、`verification-before-completion`、`writing-plans`、`writing-skills`。

### 3.1 Adopted
- workflow behavior evals:
  - 理由：oasis7 已有大量 workflow 文档与 helper，但还缺少“agent 是否真按规则做”的 repo-owned 行为验证层。
  - 目标：把主链 workflow helpers 转成 eval fixture 与 pressure-case smoke。
- verification before completion:
  - 理由：与当前 evidence-first 收口方向一致，而且能直接减少“没 fresh 验证就宣称完成”的漂移。
  - 目标：形成 claim type -> required command -> allowed evidence 的 repo-owned helper/checklist/smoke。
- visual companion:
  - 理由：Viewer Web 等 UI-heavy 题在结构和信息层级比对上确实适合浏览器侧 mockup。
  - 限域：只用于设计前置，不进入默认实现门禁。

### 3.2 Rejected
- universal brainstorming gate:
  - 与 oasis7 用户的直接执行节奏冲突。
- fresh subagent per task + two-stage review as default:
  - 与当前显式 `spawn_agent` 语义和 GitHub PR review 默认边界冲突。
- universal TDD:
  - 与当前 `test_tier_required/full`、文档/脚本/治理任务的现实粒度不匹配。
- external bootstrap as current truth:
  - 不能让 plugin bootstrap 取代 `AGENTS.md`、`.pm` 和 repo-owned docs。

### 3.3 Deferred
- multi-harness workflow packaging
- auto-trigger/bootstrap distribution
- contributor anti-slop contract 的正式 PR 模板化

这些都必须等 adopted 的 repo-owned truth 稳定后再重开。

### 3.4 Skill-to-decision mapping
| superpowers skill | decision | current handling in oasis7 |
| --- | --- | --- |
| `verification-before-completion` | adopted | 已落地为 `scripts/pm/claim-ready.sh`、claim 前 fresh verification 契约，以及同名 repo-owned skill。 |
| `using-git-worktrees` | adopted | 已由 `./scripts/new-task-worktree.sh` 和 root `AGENTS.md` 的 worktree 规则覆盖。 |
| `requesting-code-review` | adopted | 已映射到 `prepare-task-pr` + GitHub PR review 默认边界。 |
| `receiving-code-review` | adopted | 已映射到 `pr-review-thread-closeout.sh`、review fix/verify loop，以及同名 repo-owned skill。 |
| `finishing-a-development-branch` | adopted | 已映射到 `task-closeout -> prepare-task-pr -> merge/cleanup` 收口链，以及同名 repo-owned skill。 |
| `systematic-debugging` | adopted | 已本地化为 repo-owned debugging skill，不再停留在 deferred playbook。 |
| `dispatching-parallel-agents` | deferred | 只保留为显式授权下的 bounded `spawn_agent` 参考，不升为默认流程。 |
| `executing-plans` | deferred | 当前先依赖 `project.md`/`.pm` 的正式计划真值，暂不再引入单独执行会话契约。 |
| `writing-skills` | deferred | 等本地 skill surface 稳定后再决定是否需要 repo-owned 作者手册。 |
| `brainstorming` | rejected | 仅 salvage 其 visual-companion 子模式；其 universal pre-step 语义不进入默认流程。 |
| `subagent-driven-development` | rejected | fresh subagent-per-task + two-stage review 不进入当前默认主链。 |
| `test-driven-development` | rejected | universal TDD 不作为仓库默认门禁。 |
| `writing-plans` | rejected | 不允许在 `prd.md`/`project.md`/`.pm` 之外再引入第二套默认计划真值。 |
| `using-superpowers` | rejected | 外部 bootstrap 不能取代 `AGENTS.md + .pm + GitHub PR review`。 |

## 4. Follow-up 映射
### 4.1 Workflow behavior eval harness
- owner 倾向：`agent_engineer` + `qa_engineer`
- 覆盖面：
  - `scripts/new-task-worktree.sh`
  - `scripts/pm/workflow-report.sh`
  - `scripts/pm/task-closeout.sh`
  - `scripts/prepare-task-pr.sh`
  - `scripts/pr-review-thread-closeout.sh`
- 目标：验证 agent 在真实对话/fixture 下是否走对主链，而不是只打印“建议”。

### 4.2 Completion-claim verification gate
- owner 倾向：`producer_system_designer` + `qa_engineer`
- 覆盖面：
  - 完成宣称
  - 测试通过宣称
  - 可提 PR / 可合并宣称
- 目标：把“claim 之前必须 fresh verify”固定成 repo-owned 可执行契约。

### 4.2A Localized workflow skills
- 当前已本地化：
  - `.agents/skills/verification-before-completion`
  - `.agents/skills/systematic-debugging`
  - `.agents/skills/receiving-code-review`
  - `.agents/skills/finishing-a-development-branch`
- 约束：
  - 这些 skill 必须继续绑定 repo-owned helper 和正式 workflow 文档。
  - 不能把 skill 文案提升为新的独立真值系统。

### 4.3 Viewer visual companion pilot
- owner 倾向：`viewer_engineer`
- 接入点：
  - `world-simulator/viewer` 的下一轮结构/视觉专题
  - 仅在 IA、wireframe、layout compare、state diagram 等内容上使用
- 非目标：
  - 不替代 `agent-browser`
  - 不替代 repo-owned Solid/browser regression
  - 不替代实现 task

## 5. 模块回链策略
- `engineering/prd.md`：增加“外部 agent workflow 借鉴治理”顶层规则。
- `engineering/project.md`：增加当前专题建档任务。
- `engineering/prd.index.md` / `README.md`：把专题接入 `self-evolution` 可达入口。
- `world-simulator/project.md` 与对应 Viewer topic project：只补“下一轮可参考”的边界说明，不提前伪造新的实现 task。

## 6. 风险控制
- 任何 adopted 项如果没有 repo-owned follow-up，就仍视为未落地。
- 任何 rejected 项如果重新出现在 root workflow 文档中，应视为治理回弹。
- 任何 visual companion 使用如果绕过实现 task / regression / PR review，应视为越界。
- 任何已本地化 skill 如果与 `AGENTS.md`、`.pm` helper 或 engineering 专题口径漂移，应视为 inventory truth drift。
