# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）设计

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`
- 冲突 / 互借参考: `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`

审计轮次: 1

## 1. 设计定位

这份设计文档不再重复逐 skill 裁决和 rollout 历史。它只说明一件事：

- PRD 冻结 adopted / rejected / deferred 与验收边界。
- project 追踪 repo-owned rollout 和后续 follow-up。
- 本设计文档只解释 adopted 项如何接回当前 repo 主链，以及哪些冲突必须继续挡在设计层之外。

## 2. 设计落点

### 2.1 不可替换的主链

外部 workflow 借鉴不得替换当前默认执行链：

`new-task-worktree -> workflow-report start -> producer orchestrate / role subagent dispatch -> implementation/docs/tests -> task-closeout -> commit -> prepare-task-pr -> GitHub PR review/approval -> review-thread closeout`

因此 adopted 项只能以三种形态落地：

1. repo-owned helper / skill / eval / smoke
2. root workflow 的 repo-owned default orchestration rule
3. 模块专题里的 optional technique reference

### 2.2 已吸收的设计层

当前已吸收并接回主链的只有以下几层：

- workflow router:
  - 负责把 `brainstorming -> TDD -> execution -> verification -> closeout` 串成 repo-owned phase order
  - 只路由本地 skills 和 root workflow，不引入外部 bootstrap
- default role-subagent orchestration:
  - 固定为 `producer_system_designer` orchestrator + 标准角色 subagents
  - 保留单 owner role、单 `.pm` task、单 canonical worktree 与 GitHub PR review 主链
- bounded subagent-driven execution:
  - 允许把分析、实现、验证、补充 review 切成 subagent slices
  - 每个 slice 必须写清 `slice type / write scope / return contract / integration order`
- bounded brainstorming:
  - 只在方向仍模糊、范围过大或问题本身偏产品 / 架构 / UI 取舍时启用
  - 产出必须回写 `prd.md` / `project.md` / handoff / execution log
- bounded behavior-first testing:
  - 只约束“行为变更且存在稳定自动化 harness”的实现任务
  - 要么给出 RED command，要么显式写 skip reason
- completion-claim verification:
  - 任何“完成 / 通过 / 可提 PR”都必须先跑 fresh verification 并读取结果

### 2.3 可保留但不是默认门禁的层

- Viewer visual companion:
  - 只作为 UI-heavy 设计题的 optional ideation layer
  - 不替代 `agent-browser`、repo-owned regression 或正式实现 task
- workflow behavior eval harness:
  - 是 adopted follow-up，但仍属于后续验证层
  - 作用是证明 agent 真正遵守主链，而不是再造新的对话流程

## 3. 继续拒绝或 deferred 的设计边界

### 3.1 继续拒绝

以下内容继续被挡在设计层之外：

- external bootstrap 作为默认入口
- `prd.md` / `project.md` / `.pm` 之外的第二套计划真值
- universal brainstorming gate
- universal TDD gate
- fresh subagent-per-task + local two-stage review ritual
- 任何让 subagent 成为独立真值持有者的编排方式

### 3.2 继续 deferred

以下内容只有在 repo-owned truth 和 eval 稳定后才允许重开：

- multi-harness workflow packaging
- auto-trigger / bootstrap distribution
- 更重的 contributor anti-slop contract 模板化

重开时仍必须满足：

1. 不替代 `AGENTS.md`、`.pm`、task execution log、GitHub PR review
2. 有明确 owner、验证面和回写面
3. 落点必须是 helper / skill / eval / optional technique，而不是新的真值系统

## 4. 写回与风险控制

### 4.1 正式写回面

- root workflow: `AGENTS.md`
- local skills: `.agents/skills/README.md` 与对应 repo-owned skills
- role collaboration: `.agents/roles/*.md` 与 handoff/planning templates
- topic truth: 本专题 `prd.md`、`project.md`、conflict reference

### 4.2 风险控制

- adopted 项若没有 repo-owned 落点，视为未真正 adopted
- rejected 项若重新出现在 root workflow，视为治理回弹
- subagent 协作若缺少 owner、write scope 或 return contract，视为越界
- brainstorming / TDD 若被写成所有任务的 mandatory pre-step，视为越界
- visual companion 若绕过实现 task / regression / PR review，视为越界

## 5. 使用方式

- 看正式裁决：读 `agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 看当前 rollout 和 follow-up：读 `agent-workflow-borrowing-governance-2026-05-19.project.md`
- 看逐 skill 冲突、已吸收部分和 reopen 条件：读 `superpowers-conflict-reconciliation-2026-05-20.md`
