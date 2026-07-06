# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）设计

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`
- 冲突 / 互借参考: 本文件“继续拒绝或 deferred 的边界”与“逐 skill 冲突边界速查”。

审计轮次: 1

## 目标

这份设计文档不再重复逐 skill 裁决和 rollout 历史，只解释 adopted 项如何接回当前 repo 主链，以及哪些冲突必须继续挡在设计层之外。

- PRD 冻结 adopted / rejected / deferred 与验收边界。
- project 追踪 repo-owned rollout 和后续 follow-up。
- design 只负责解释当前默认链路上的接线方式和防回弹约束。

## 范围

### 不可替换的主链

外部 workflow 借鉴不得替换当前默认执行链：

`new-task-worktree -> GitHub-backed task bootstrap -> tpm orchestrate / role subagent dispatch -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR watch/fix/review-thread closeout/merge`

因此 adopted 项只能以三种形态落地：

1. repo-owned helper / skill / eval / smoke
2. root workflow 的 repo-owned default orchestration rule
3. 模块专题里的 optional technique reference

### 已吸收的设计层

当前已吸收并接回主链的只有以下几层：

- workflow router：把 `brainstorming -> TDD -> execution -> verification -> closeout` 串成 repo-owned phase order，只路由本地 skills 和 root workflow，不引入外部 bootstrap。
- default role-subagent orchestration：固定为 `tpm` orchestrator + 标准角色 subagents，同时保留单 owner role、单 GitHub-backed task truth、单 canonical worktree 与 GitHub PR review 主链。
- bounded subagent-driven execution：允许把分析、实现、验证、补充 review 切成 subagent slices，但每个 slice 必须写清 `slice type / write scope / return contract / integration order`。
- bounded brainstorming：只在方向仍模糊、范围过大或问题本身偏产品 / 架构 / UI 取舍时启用，产出必须回写 `prd.md` / `project.md` / handoff / GitHub task issue evidence comments。
- bounded behavior-first testing：只约束“行为变更且存在稳定自动化 harness”的实现任务，要么给出 RED command，要么显式写 skip reason。
- completion-claim verification：任何“完成 / 通过 / 可提 PR”都必须先跑 fresh verification 并读取结果。

### 可保留但不是默认门禁的层

- Viewer visual companion：只作为 UI-heavy 设计题的 optional ideation / evidence layer，可包含 Image2 target image 与 native/browser screenshot comparison；target image 不替代 `agent-browser`、真实交互 smoke、repo-owned regression 或正式实现 task。
- workflow behavior eval harness：已落为 `scripts/pm/workflow-behavior-eval.sh`，通过 task-worktree bootstrap smoke、subagent contract surface 校验、PM closeout/claim gate smoke、PR preflight fixture 与 review-thread closeout fixture 证明主链仍可回放。

### 继续拒绝或 deferred 的边界

以下内容继续被挡在设计层之外：

- external bootstrap 作为默认入口
- `prd.md` / `project.md` / GitHub-backed task evidence 之外的第二套计划真值
- universal brainstorming gate
- universal TDD gate
- fresh subagent-per-task + local two-stage review ritual
- 任何让 subagent 成为独立真值持有者的编排方式

以下内容只有在 repo-owned truth 和 eval 稳定后才允许重开：

- multi-harness workflow packaging
- auto-trigger / bootstrap distribution
- 更重的 contributor anti-slop contract 模板化

## 接口 / 数据

正式写回面与治理接口如下：

- root workflow: `AGENTS.md`
- local skills: `.agents/skills/README.md` 与对应 repo-owned skills
- role collaboration: `.agents/roles/*.md` 与 handoff/planning templates
- topic truth: 本专题 `prd.md`、`design.md`、`project.md`

重开 deferred 项时必须同时提供：

1. 明确 owner
2. 明确验证面
3. 明确正式回写面

## 里程碑

- 第一阶段：冻结 adopted / rejected / deferred 边界，并把 planning / execution / skill-authoring 的可 salvage 部分翻译成 repo-owned surface。
- 第二阶段：把默认协作收口为 `tpm` orchestrator + role subagents，并把 bounded brainstorming / TDD / execution / verification / closeout 接回 root workflow。
- 第三阶段：补齐 `task-closeout` fresh verification gate 与 `workflow-behavior-eval.sh`，让默认链路可以通过 repo-owned eval 回放。

## 风险

- adopted 项若没有 repo-owned 落点，视为未真正 adopted。
- rejected 项若重新出现在 root workflow，视为治理回弹。
- subagent 协作若缺少 owner、write scope 或 return contract，视为越界。
- brainstorming / TDD 若被写成所有任务的 mandatory pre-step，视为越界。
- visual companion 若绕过实现 task / 真实截图或交互 smoke / regression / PR review，视为越界。

## 逐 skill 冲突边界速查

本节承接已退役的 standalone `superpowers` conflict reference；正式状态仍以本专题 PRD / project 为准。

| upstream skill / pattern | 当前状态 | 剩余冲突 | 已吸收的 repo-owned 部分 | 重开条件 |
| --- | --- | --- | --- | --- |
| `brainstorming` | adopted（bounded） | universal gate、逐段审批、强制转入 `writing-plans` | `bounded-brainstorming`、scope decomposition、2-3 方案对比、推荐方向、optional visual companion | 只允许在局部专题内扩张；不得回流成 root mandatory pre-step |
| `requesting-code-review` | adopted（bounded） | every-task reviewer ritual、把本地 review 提升为正式评审主链 | `requesting-repo-owned-review`、review packet、`findings / no_findings / residual_risk` formal sink | 只允许补强 high-risk local diff；不得替代 GitHub PR review |
| `subagent-driven-development` | adopted（bounded） | fresh subagent-per-task、local review ritual、subagent 独立真值化 | bounded subagent-driven execution、任务拆分、上下文最小化、write-scope / return-contract contract | 仅当 repo-owned multi-agent eval 稳定且 review 边界变化时重开 |
| `test-driven-development` | adopted（bounded） | universal TDD gate、对无稳定 harness 任务强套 RED | behavior-first / regression-first contract、`tdd-test-writer`、RED command or skip reason | 只允许在更细局部领域继续扩张；不得升成 universal gate |
| `writing-plans` | rejected（整体 skill） | 与 `prd.md` / `project.md` / GitHub-backed task evidence 形成第二套计划真值 | `File Structure / Affected Paths`、handoff 原子步骤、planning self-checklist | 只有在不竞争正式计划真值时才允许继续局部 salvage |
| `using-superpowers` | rejected（overall bootstrap） | 外部 bootstrap、packaging、第二套入口语义 | `default-workflow-bootstrap`、`repo-owned-workflow-router`、触发说明、skill 发现习惯 | bootstrap / packaging / 第二套入口语义继续 rejected |
| `dispatching-parallel-agents` | adopted（bounded） | 无 owner / 无 write-scope 的自由并行 | `tpm` orchestrator + role subagents、parallel task decomposition、disjoint write scope | 仅当 multi-agent eval 稳定后，才应继续评估 swarm / packaging 扩张 |
| `executing-plans` | deferred（整体 skill） | 整包引入会与正式 project/task 执行链重复 | `.agents/skills/executing-project-tasks`、execution gap review、逐步验证、blocker handling | 剩余会话包装与默认收尾假设继续 deferred |
| `writing-skills` | deferred（整体 skill） | 分发/作者规范容易先于治理真值 | `.agents/skills/README.md`、`writing-repo-owned-skills`、template、checklist | 剩余分发部署与上游 gate 部分继续 deferred |

## 使用方式

- 看正式裁决：读 `agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 看当前 rollout 和 follow-up：读 `agent-workflow-borrowing-governance-2026-05-19.project.md`
- 看逐 skill 冲突、已吸收部分和 reopen 条件：读本文件“逐 skill 冲突边界速查”。
