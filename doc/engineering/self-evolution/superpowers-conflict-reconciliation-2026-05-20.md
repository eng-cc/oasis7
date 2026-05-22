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

`new-task-worktree -> workflow-report start -> producer orchestrate / role subagent dispatch -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR review/approval -> review-thread closeout`

因此外部 workflow 只有四种合法落点：

1. 变成 repo-owned helper / skill / eval。
2. 变成 root workflow 的 repo-owned default orchestration rule。
3. 变成某个模块专题里的 optional technique。
4. 保持 `deferred`，直到当前真值链已经能稳定承接。

凡是会把外部 repo 升级成默认 bootstrap、默认计划系统、平行于 owner/task/worktree/PR 真值的 subagent 协作系统、或默认测试体系的，都与当前真值链冲突。

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

已完成的 reconcile：

- `brainstorming` 已改为 adopted（bounded）：只把“scope 是否需要拆分、是否需要 2-3 方案对比、是否需要 optional visual companion、以及推荐方向必须回写正式文档”的 ideation discipline 接回 root workflow 与本地 `bounded-brainstorming` skill。
- `test-driven-development` 已改为 adopted（bounded）：只把 behavior-first / regression-first contract 接回“行为变更且存在稳定自动化 harness”的实现任务。

与 oasis7 仍然冲突的点：

- `brainstorming` 的 universal mandatory pre-step、逐段审批和强制转入 `writing-plans` 仍与当前直接执行节奏和正式计划真值冲突。
- `test-driven-development` 的 universal mandatory pre-step 语义仍与当前 `test_tier_required/full` 分层、文档/治理任务粒度和无稳定 harness 场景冲突。

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
- upstream `using-superpowers` 里“先决定当前应进入哪段 process skill”的路由顺序

- 已完成的 reconcile：

- 已将 `using-superpowers` 中唯一值得保留的 process-skill routing order 翻译成 repo-owned workflow router：新增 `.agents/skills/repo-owned-workflow-router`，并把 `bounded-brainstorming -> tdd-test-writer -> executing-project-tasks -> verification-before-completion -> finishing-a-development-branch` 的默认 phase order 接回 root `AGENTS.md` 与 `.agents/skills/README.md`。

未来何时可重开：

- 当前 `writing-skills` 的 authoring / trigger surface，以及 `using-superpowers` 的 process-skill routing order 都已经被翻译成 repo-owned 入口；若要重开这里的剩余部分，只能针对 upstream 仍未采纳的 bootstrap、分发部署或其他会竞争 root truth 的内容，而不是回退去重新依赖外部 bootstrap 本身。

### 3.3 默认 subagent 协作的已吸收部分与剩余冲突

这类 skill 讨论的是 agent dispatch 如何成为默认开发方式；现在 `dispatching-parallel-agents` 和 `subagent-driven-development` 都已有 bounded adoption，剩下冲突的是仍然越界的 ceremony 与真值扩张部分。

受影响 skill：

- `subagent-driven-development`
- `dispatching-parallel-agents` 的未限域部分

已完成的 reconcile：

- 默认协作模式已改为 `producer_system_designer` orchestrator + 标准角色 subagents。
- 默认实施模式已改为 bounded subagent-driven execution：分析、实现、验证与补充 review 可交给角色 subagent，但都必须回收到同一 owner / `.pm` task / canonical worktree / GitHub PR 主链。
- handoff / planning surface 已补 `slice type`、`write scope`、`return contract` 与 `integration order` 字段，避免默认 subagent-driven 流程停留在口头约定。
- 角色 subagent 的输出必须回收到单 owner role、单 `.pm` task、单 canonical worktree 与 GitHub PR review 主链。

与 oasis7 仍然冲突的点：

- `subagent-driven-development` 剩余的 fresh subagent-per-task + local two-stage review ritual 仍与当前正式 PR review 边界冲突。
- 多 agent 并行若没有 task / file ownership 边界，仍然容易和 `.pm` task 及 worktree 隔离原则冲突。
- 任何把 subagent 放大成独立真值持有者的做法，仍会冲击 owner / task / review 责任链。

已 salvage 的部分：

- 默认角色 subagent 编排
- 默认 subagent-driven execution
- 把可并行子任务拆成 disjoint write scope 的原则
- reviewer / implementer 分离时的上下文最小化习惯

未来何时可重开剩余部分：

- 仅当某个 repo-owned eval 已能证明 agent 在多任务并行下仍遵守 worktree、task、review 边界，且仓库正式 review 边界本身发生变化，才应重新评估 local two-stage review ritual 这类剩余 rejected 部分。

### 3.4 分发先于治理冲突

这类内容不一定和当前仓库“语义冲突”，但它们会把分发形态跑在治理真值前面。

受影响 skill / 模式：

- `using-superpowers`
- `writing-skills`
- `dispatching-parallel-agents` 的 harness 化分发

与 oasis7 的冲突点：

- 当前 repo-owned truth 虽然已经补齐了 planning / execution / skill-authoring surface，但分发与部署边界仍未稳定；若先做 bootstrap / packaging，仍然容易出现“看起来可用，但无法审计谁是正式规则”的反向漂移。

可 salvage 的部分：

- skill 目录组织方式
- authoring checklist
- trigger wording / local-surface entrypoint 约束

未来何时可重开：

- `writing-skills` 的 authoring surface 已落到本地后，剩余可重开的只应是 packaging / distribution 类问题；且只有当 adopted skill / helper / eval 已经稳定，并且相应 owner 愿意维护 authoring / packaging 契约时，才应继续推进。

## 4. Skill-by-skill 冲突与互借表

说明：

- `当前状态` 以当前 oasis7 正式裁决为准；若某个 upstream skill 已被明确翻译成 repo-owned 默认规则，应直接写 `adopted`
- `可借鉴部分` 优先写已经收口到 repo-owned surface 的内容，再补 still-open 的 bounded borrowing
- `重开条件` 只描述 remaining deferred / rejected 部分何时允许继续推进

| skill | 当前状态 | 直接冲突 | 可借鉴部分 | 重开条件 |
| --- | --- | --- | --- | --- |
| `brainstorming` | adopted（bounded） | universal gate、逐段审批、或强制转入 `writing-plans` | 已完成 bounded borrowing：scope decomposition、2-3 方案对比、推荐方向、optional visual companion 与本地 `bounded-brainstorming` skill | 仅剩 universal gate / ceremony 扩张部分保持 rejected；只有在局部专题被证明稳定后才应继续扩张 |
| `subagent-driven-development` | adopted（bounded） | fresh subagent-per-task + local review ritual、或把 subagent 扩成独立真值持有者 | 已完成 bounded borrowing：默认 subagent-driven execution、任务拆分、上下文最小化、实现/验证/补充 review 切片，以及 `slice type / write scope / return contract / integration order` contract | 仅剩 local review ritual 等 rejected 部分；只有在正式 PR review 边界变化且 repo-owned multi-agent eval 稳定后才应重开 |
| `test-driven-development` | adopted（bounded） | universal TDD mandatory gate、或对无稳定 harness 的任务强套 RED-phase | 已完成 bounded borrowing：behavior-first / regression-first contract、`tdd-test-writer` skill、RED command or skip reason 约束 | 仅剩 universal gate 等 rejected 部分；只有在更细的局部领域验证稳定后才应继续扩张 |
| `writing-plans` | rejected（整体 skill） | 与 `prd.md` / `project.md` / `.pm` 形成第二套计划真值 | 已完成 bounded borrowing：`project.md` 的 `File Structure / Affected Paths`、handoff 原子步骤模板和 planning self-checklist | 剩余 skill 本体只有在不再竞争正式计划真值时，才允许继续局部 salvage；不得回退为默认前置计划系统 |
| `using-superpowers` | rejected（overall bootstrap） | 外部 bootstrap 与当前 root workflow 真值冲突 | 已完成 bounded borrowing：repo-owned workflow router、触发说明、skill 发现习惯与本地 phase-order 入口 | 剩余 bootstrap / packaging / 第二套 workflow truth 语义继续 rejected；不得回退为外部默认入口 |
| `dispatching-parallel-agents` | adopted（bounded） | 无 owner / 无 write-scope 的自由并行仍冲突 | 已完成 bounded borrowing：默认 `producer_system_designer` orchestrator + role subagents、parallel task decomposition、disjoint write scope 约束 | 剩余 harness packaging 或无边界 swarm 语义仍需 repo-owned multi-agent eval 后才允许继续扩张 |
| `executing-plans` | deferred（整体 skill） | 若整包引入，仍会和正式 project/task 执行链重复 | 已完成 bounded borrowing：`.agents/skills/executing-project-tasks`、execution gap review、逐步验证、明确 blocker handling | 剩余 upstream 单独执行会话包装与默认收尾假设继续保持 deferred，不得升级为第二套计划真值 |
| `writing-skills` | deferred（整体 skill） | 分发/作者规范容易先于治理真值 | 已完成 bounded borrowing：`.agents/skills/README.md`、`writing-repo-owned-skills`、template、checklist 与 trigger-entry 说明 | 剩余 upstream TDD/subagent gate 与分发部署部分仍 deferred；只有在本地 skill / helper / eval 真值稳定后才允许重开 |

## 5. 后续互借的优先级

如果后续真的要继续解决冲突、互相借鉴，推荐顺序不是从 `rejected` 开始，而是先把“还没吸收的剩余部分”和“已经完成 bounded borrowing 的部分”分开看：

1. 已完成 bounded borrowing、当前不需要再重开的部分
   - `writing-plans` 的 planning surface
   - `executing-plans` 的 execution surface
   - `writing-skills` 的 authoring surface
   - `using-superpowers` 的 repo-owned workflow router
   - `dispatching-parallel-agents` 与 `subagent-driven-development` 的 bounded 默认角色编排 / subagent-driven execution
2. 先做 `deferred` 中仍未吸收、且最接近当前主链的可控部分
   - `executing-plans` / `writing-skills` 剩余未吸收的 packaging、distribution 或 session-contract 部分，但前提仍是它们不形成第二套真值
   - `dispatching-parallel-agents` 剩余的 harness packaging / swarm 扩张部分，但前提仍是 bounded 默认编排已被 eval 证明稳定
3. 再做 `rejected` 里最容易局部 salvage 的子模式
   - 当前已不再需要把 `brainstorming` 当 reopen 起点；其 bounded ideation discipline 已吸收，剩余只是不应扩张的 rejected ceremony
4. 最后才碰第二套真值风险最高的内容
   - `writing-plans`
   - `using-superpowers` 剩余的 bootstrap / packaging 本体

原因很简单：第一类已经有 repo-owned 落点，不应再被误写成“未来可能借”；第二、三类最多是“局部 technique 还没完全落成 repo-owned”；最后一类则直接碰 root workflow 真值。

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

- 不为缺少 owner / write-scope / review 边界的自由 swarm 背书。
- 不单凭这份冲突说明文档自行裁决 skill；正式状态仍以 borrowing PRD / project 为准。
- 不为尚未启动的 reopen 项伪造新的实现任务。
- 不把“冲突存在”误写成“永远不能互借”。

## 8. 使用方式

后续如果有人再问“这个冲突项是不是可以借一点”，优先按下面顺序读：

1. 本文档：先看冲突发生在哪个真值层。
2. `agent-workflow-borrowing-governance-2026-05-19.prd.md`：看当前正式裁决。
3. `agent-workflow-borrowing-governance-2026-05-19.project.md`：看是否已有 follow-up 或 task。
4. 若仍值得推进，再新开独立 worktree 和 `.pm` task。
