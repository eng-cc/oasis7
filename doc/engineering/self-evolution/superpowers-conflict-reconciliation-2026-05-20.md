# oasis7：`superpowers` workflow 冲突与互借参考（2026-05-20）

- 文档类型: Explanation / Reference
- 目标读者: `producer_system_designer`、`agent_engineer`、`qa_engineer`、后续要继续处理 `superpowers` 借鉴的人
- 上游样本: `https://github.com/obra/superpowers/tree/main/skills`
- 对应治理专题:
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`

## 1. 这份文档解决什么问题

`agent-workflow-borrowing-governance-2026-05-19` 已经给出 `superpowers` 各 skill 的 `adopted / deferred / rejected` 裁决，但后续如果要继续互相借鉴，仅靠“当前结论”还不够。

真正需要保留的是：

1. 当前为什么冲突。
2. 冲突发生在 oasis7 哪条真值链上。
3. 哪一部分是可 salvage 的。
4. 未来要满足什么条件，才允许从 `rejected` / `deferred` 重新进入 adopted 评估。

这份文档就是为后续 reopen 这些问题准备的。

## 2. 当前 oasis7 不可替换的真值链

后续任何互借都不能绕开当前默认执行链：

`new-task-worktree -> workflow-report start -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR review/approval -> review-thread closeout`

因此外部 workflow 只有三种合法落点：

1. 变成 repo-owned helper / skill / eval。
2. 变成某个模块专题里的 optional technique。
3. 保持 `deferred`，直到当前真值链已经能稳定承接。

凡是会把外部 repo 升级成默认 bootstrap、默认计划系统、默认 subagent 协作系统、或默认测试体系的，都与当前真值链冲突。

## 3. 冲突类型

### 3.1 默认前置步骤冲突

这类 skill 的问题不是“方法一定错”，而是它们试图把某一步变成所有任务的 mandatory first step。

受影响 skill：

- `brainstorming`
- `writing-plans`
- `test-driven-development`

与 oasis7 的冲突点：

- 用户经常直接要求“做 / 继续 / landing”，默认期望立即执行，而不是先进入固定 ceremony。
- 当前正式计划真值已经是 `prd.md` / `project.md` / `.pm` task。
- 当前测试真值是 `test_tier_required / test_tier_full` 分层，不是 universal TDD。

可 salvage 的部分：

- `brainstorming` 里的 visual companion 子模式
- `writing-plans` 里的结构化拆分习惯
- `test-driven-development` 里的“先证明行为再落实现”思路

未来何时可重开：

- 只有当某个子领域明确需要额外前置步骤，而且该步骤被限制在局部专题内，而不是回流为 root 默认规则。

### 3.2 第二套流程真值冲突

这类 skill 会把外部方法论包装成新的权威入口，直接与 `AGENTS.md + .pm + GitHub PR review` 竞争。

受影响 skill：

- `using-superpowers`
- `writing-plans`

与 oasis7 的冲突点：

- `using-superpowers` 把外部 skill bootstrap 当成对话默认前提。
- `writing-plans` 若升成默认前置，会与现有正式计划文档形成并行系统。

可 salvage 的部分：

- skill 发现和说明方式
- 对“何时触发某类 skill”做更明确约束

未来何时可重开：

- 仅当这些内容能被翻译成 repo-owned skill authoring / trigger governance，而不是继续引用外部 bootstrap 本身。

### 3.3 默认 subagent 协作冲突

这类 skill 试图把 agent dispatch 升成“默认开发方式”。

受影响 skill：

- `subagent-driven-development`
- `dispatching-parallel-agents`

与 oasis7 的冲突点：

- 当前 `spawn_agent` 仍是显式授权能力，而不是所有任务的默认第一反应。
- 当前正式评审边界是 GitHub PR review，不是 local two-stage review ritual。
- 多 agent 并行若没有 task / file ownership 边界，容易和 `.pm` task 及 worktree 隔离原则冲突。

可 salvage 的部分：

- 把可并行子任务拆成 disjoint write scope 的原则
- reviewer / implementer 分离时的上下文最小化习惯

未来何时可重开：

- 仅当某个 repo-owned eval 已能证明 agent 在多任务并行下仍遵守 worktree、task、review 边界。

### 3.4 分发先于治理冲突

这类内容不一定和当前仓库“语义冲突”，但它们会把分发形态跑在治理真值前面。

受影响 skill / 模式：

- `using-superpowers`
- `writing-skills`
- `dispatching-parallel-agents` 的 harness 化分发

与 oasis7 的冲突点：

- 当前 repo-owned truth 还在持续收紧；若先做 bootstrap / packaging，很容易出现“看起来可用，但无法审计谁是正式规则”的反向漂移。

可 salvage 的部分：

- skill 目录组织方式
- authoring checklist

未来何时可重开：

- 只有当 adopted skill / helper / eval 已经稳定，并且相应 owner 愿意维护 authoring / packaging 契约时。

## 4. Skill-by-skill 冲突与互借表

| skill | 当前状态 | 直接冲突 | 可借鉴部分 | 重开条件 |
| --- | --- | --- | --- | --- |
| `brainstorming` | rejected | 把设计前置变成 universal gate | visual companion、IA/wireframe 对比 | 仅在 Viewer 等 UI-heavy 专题内按需启用 |
| `subagent-driven-development` | rejected | 默认 fresh subagent-per-task + local review ritual | 任务拆分、上下文最小化 | 需先有 repo-owned multi-agent behavior eval |
| `test-driven-development` | rejected | universal TDD 与 `test_tier_required/full` 不匹配 | 行为先验、失败先行思维 | 仅在特定实现域作为按需 skill，而非 root 默认规则 |
| `writing-plans` | rejected | 与 `prd.md` / `project.md` / `.pm` 形成第二套计划真值 | 已限域翻译为 `project.md` 的 `File Structure / Affected Paths`、handoff 原子步骤模板和 planning self-checklist | 只能继续作为 repo-owned planning surface，不得替代现有文档链 |
| `using-superpowers` | rejected | 外部 bootstrap 与当前 root workflow 真值冲突 | 触发说明、skill 发现习惯 | 必须先转成 repo-owned trigger governance，再评估 |
| `dispatching-parallel-agents` | deferred | 若默认启用会冲击显式 `spawn_agent` 边界 | parallel task decomposition | 需先证明多 agent 仍遵守 worktree/task/review 边界 |
| `executing-plans` | deferred | 若整包引入，仍会和正式 project/task 执行链重复 | 已限域借鉴为 `.agents/skills/executing-project-tasks`、execution gap review、逐步验证、明确 blocker handling | upstream 的单独执行会话包装与默认收尾假设继续保持 deferred，不得升级为第二套计划真值 |
| `writing-skills` | deferred | 分发/作者规范容易先于治理真值 | 已限域翻译为本地 skill authoring skill、template、checklist 与入口说明 | 仅继续保留 authoring surface；upstream TDD/subagent gate 与分发部署部分仍 deferred |

## 5. 后续互借的优先级

如果后续真的要继续解决冲突、互相借鉴，推荐顺序不是从 `rejected` 开始，而是：

1. 先做 `deferred` 中最接近当前主链的可控部分
   - `dispatching-parallel-agents` 的 bounded decomposition 原则
   - `executing-plans` 的“已有正式计划后的执行 discipline”，这部分现已收口成 `.agents/skills/executing-project-tasks`；剩余 session packaging 仍 deferred
2. 再做 `rejected` 里最容易局部 salvage 的子模式
   - `brainstorming` 的 visual companion
   - `test-driven-development` 的 behavior-first 子集
3. 最后才碰第二套真值风险最高的内容
   - `writing-plans`
   - `using-superpowers`

原因很简单：前两类最多是“局部 technique 还没落成 repo-owned”，最后一类则直接碰 root workflow 真值。

## 6. 允许重开的判定标准

后续若要把某个当前冲突项重新拿出来评估，至少要同时满足：

1. 不替代 `AGENTS.md`、`.pm` task、task execution log、GitHub PR review 这四条正式真值。
2. 能说明落点是 helper、skill、eval，还是某个模块专题内的 optional technique。
3. 有明确 owner。
4. 有明确验证面。
5. 不把局部技巧偷渡成所有任务的默认 mandatory pre-step。

只要其中任一条答不上来，就不该 reopen。

## 7. 非目标

这份文档不做以下事情：

- 不重新裁决 `superpowers` skill 的 adopted / deferred / rejected 状态。
- 不直接把任何 `rejected` skill 改成 adopted。
- 不为尚未启动的 reopen 项伪造新的实现任务。
- 不把“冲突存在”误写成“永远不能互借”。

## 8. 使用方式

后续如果有人再问“这个冲突项是不是可以借一点”，优先按下面顺序读：

1. 本文档：先看冲突发生在哪个真值层。
2. `agent-workflow-borrowing-governance-2026-05-19.prd.md`：看当前正式裁决。
3. `agent-workflow-borrowing-governance-2026-05-19.project.md`：看是否已有 follow-up 或 task。
4. 若仍值得推进，再新开独立 worktree 和 `.pm` task。
