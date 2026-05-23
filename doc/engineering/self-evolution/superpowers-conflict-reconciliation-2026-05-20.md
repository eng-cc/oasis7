# oasis7：`superpowers` workflow 冲突与互借参考（2026-05-20）

- 文档类型: Explanation / Reference
- 目标读者: `producer_system_designer`、`agent_engineer`、`qa_engineer`
- 上游样本: `https://github.com/obra/superpowers/tree/main/skills`
- 对应治理专题:
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`

## 1. 这份文档保留什么

这份文档只保留四类信息：

1. 当前为什么冲突。
2. 冲突撞到 oasis7 的哪条真值链。
3. 哪一部分已经被翻译成 repo-owned surface。
4. 剩余部分在什么条件下才允许 reopen。

不再重复保留每一轮 truth refresh 的过程叙述。

## 2. 不可替换的真值链

后续任何互借都不能绕开当前默认执行链：

`new-task-worktree -> workflow-report start -> producer orchestrate / role subagent dispatch -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR review/approval -> review-thread closeout`

因此外部 workflow 只有四种合法落点：

1. 变成 repo-owned helper / skill / eval。
2. 变成 root workflow 的 repo-owned default orchestration rule。
3. 变成某个模块专题里的 optional technique。
4. 保持 `deferred`，直到当前真值链已经能稳定承接。

凡是会把外部 repo 升级成默认 bootstrap、第二套计划真值、平行于 owner/task/worktree/PR 的 subagent 协作系统，或 universal mandatory gate 的，都与当前真值链冲突。

## 3. 冲突类型

### 3.1 默认前置步骤冲突

受影响 skill：

- `brainstorming`
- `writing-plans`
- `test-driven-development`

冲突点：

- 用户经常直接要求“做 / 继续 / landing”，默认期望立即执行。
- 当前正式计划真值已经是 `prd.md` / `project.md` / `.pm` task。
- 当前测试真值是 `test_tier_required / test_tier_full` 分层，不是 universal TDD。

已吸收的部分：

- `brainstorming`：scope decomposition、2-3 方案对比、推荐方向、optional visual companion
- `writing-plans`：`File Structure / Affected Paths`、handoff 原子步骤、planning self-checklist
- `test-driven-development`：behavior-first / regression-first contract、`tdd-test-writer`、RED command or skip reason

仍保持冲突的部分：

- `brainstorming` 的 universal gate、逐段审批、强制转入 `writing-plans`
- `test-driven-development` 的 universal mandatory gate
- `writing-plans` 作为第二套默认计划系统

重开条件：

- 只能在局部专题内限域扩张，不能回流成 root 默认 ceremony。

### 3.2 第二套流程真值冲突

受影响 skill：

- `using-superpowers`
- `writing-plans`

冲突点：

- `using-superpowers` 把外部 bootstrap 当成对话默认前提。
- `writing-plans` 若升成默认前置，会与现有正式计划文档形成并行系统。

已吸收的部分：

- `using-superpowers` 中的 process-skill routing order 已翻译成 `.agents/skills/repo-owned-workflow-router`
- `writing-skills` 的 authoring / trigger surface 已翻译成本地 authoring surface

仍保持冲突的部分：

- 外部 bootstrap 本体
- packaging / distribution 先于 repo-owned truth
- 任何会竞争 `AGENTS.md + .pm + GitHub PR review` 的并行入口

重开条件：

- 只允许针对剩余 bootstrap / packaging / distribution 问题做局部评估，不得回退为依赖外部入口。

### 3.3 默认 subagent 协作的剩余冲突

受影响 skill：

- `subagent-driven-development`
- `dispatching-parallel-agents` 的未限域部分

已吸收的部分：

- `producer_system_designer` orchestrator + 标准角色 subagents
- bounded subagent-driven execution
- `slice type / write scope / return contract / integration order` handoff/planning contract
- disjoint write scope 的并行原则

仍保持冲突的部分：

- fresh subagent-per-task + local two-stage review ritual
- 无 owner / 无 write-scope 的自由并行
- 把 subagent 放大成独立真值持有者

重开条件：

- 只有当 repo-owned multi-agent eval 能证明 agent 在并行场景下仍遵守 worktree、task、review 边界，且正式 review 边界本身发生变化，才应继续评估。

### 3.4 分发先于治理冲突

受影响模式：

- `using-superpowers`
- `writing-skills`
- `dispatching-parallel-agents` 的 harness 化分发

冲突点：

- 当前 repo-owned planning / execution / skill-authoring surface 已落地，但分发与部署边界仍未稳定；若先做 packaging，很容易出现“可用但不可审计”的反向漂移。

已吸收的部分：

- skill 目录组织方式
- authoring checklist
- trigger wording / local-surface entrypoint

重开条件：

- 只有当 adopted skill / helper / eval 已稳定，且相应 owner 愿意维护 packaging 契约，才应继续推进。

## 4. Skill-by-skill 表

| skill | 当前状态 | 直接冲突 | 已吸收的部分 | 剩余重开条件 |
| --- | --- | --- | --- | --- |
| `brainstorming` | adopted（bounded） | universal gate、逐段审批、强制转入 `writing-plans` | `bounded-brainstorming`、scope decomposition、2-3 方案对比、推荐方向、optional visual companion | 只允许在局部专题内扩张；不得回流成 root mandatory pre-step |
| `requesting-code-review` | adopted（bounded） | every-task reviewer ritual、把本地 review 提升为正式评审主链 | `requesting-repo-owned-review`、review packet、`findings / no_findings / residual_risk` formal sink | 只允许补强 high-risk local diff；不得替代 GitHub PR review |
| `subagent-driven-development` | adopted（bounded） | fresh subagent-per-task、local review ritual、subagent 独立真值化 | bounded subagent-driven execution、任务拆分、上下文最小化、write-scope / return-contract contract | 仅当 repo-owned multi-agent eval 稳定且 review 边界变化时重开 |
| `test-driven-development` | adopted（bounded） | universal TDD gate、对无稳定 harness 任务强套 RED | behavior-first / regression-first contract、`tdd-test-writer`、RED command or skip reason | 只允许在更细局部领域继续扩张；不得升成 universal gate |
| `writing-plans` | rejected（整体 skill） | 与 `prd.md` / `project.md` / `.pm` 形成第二套计划真值 | `File Structure / Affected Paths`、handoff 原子步骤、planning self-checklist | 只有在不竞争正式计划真值时才允许继续局部 salvage |
| `using-superpowers` | rejected（overall bootstrap） | 外部 bootstrap 与当前 root workflow 真值冲突 | `repo-owned-workflow-router`、触发说明、skill 发现习惯 | bootstrap / packaging / 第二套入口语义继续 rejected |
| `dispatching-parallel-agents` | adopted（bounded） | 无 owner / 无 write-scope 的自由并行 | `producer_system_designer` orchestrator + role subagents、parallel task decomposition、disjoint write scope | 仅当 multi-agent eval 稳定后，才应继续评估 swarm / packaging 扩张 |
| `executing-plans` | deferred（整体 skill） | 整包引入会与正式 project/task 执行链重复 | `.agents/skills/executing-project-tasks`、execution gap review、逐步验证、blocker handling | 剩余会话包装与默认收尾假设继续 deferred |
| `writing-skills` | deferred（整体 skill） | 分发/作者规范容易先于治理真值 | `.agents/skills/README.md`、`writing-repo-owned-skills`、template、checklist | 剩余分发部署与上游 gate 部分继续 deferred |

## 5. 后续优先级

后续若继续互借，推荐顺序是：

1. 先看仍未吸收但最接近当前主链的 `deferred` 剩余部分。
2. 再看 `rejected` 里是否存在可被局部专题限域承接的子模式。
3. 最后才碰 bootstrap / 第二套计划真值 / packaging 这类高风险内容。

不要回退去重复讨论已经完成 bounded borrowing 的部分。

## 6. 允许重开的最低标准

后续若要重开某个冲突项，至少要同时满足：

1. 不替代 `AGENTS.md`、`.pm` task、task execution log、GitHub PR review 四条正式真值。
2. 能说明落点是 helper、skill、eval，还是模块专题内的 optional technique。
3. 有明确 owner。
4. 有明确验证面。
5. 不把局部技巧偷渡成所有任务的默认 mandatory pre-step。

只要其中任一条答不上来，就不该 reopen。

## 7. 非目标

- 不为缺少 owner / write-scope / review 边界的自由 swarm 背书。
- 不单凭这份文档自行裁决 skill；正式状态仍以 borrowing PRD / project 为准。
- 不为尚未启动的 reopen 项伪造新的实现任务。

## 8. 使用方式

后续如果再问“这个冲突项是不是还能借一点”，优先按下面顺序读：

1. 本文档：先看冲突发生在哪个真值层。
2. `agent-workflow-borrowing-governance-2026-05-19.prd.md`：看当前正式裁决。
3. `agent-workflow-borrowing-governance-2026-05-19.project.md`：看当前正式 follow-up。
4. 若仍值得推进，再新开独立 worktree 和 `.pm` task。
