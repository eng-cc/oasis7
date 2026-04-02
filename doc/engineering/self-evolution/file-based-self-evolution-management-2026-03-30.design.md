# oasis7：自我进化文件化项目管理设计（2026-03-30）

- 对应需求文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`

审计轮次: 7

## 目标
- 在仓库内建立一套不依赖外部服务的文件化项目管理运行层，为 7 个标准角色及后续扩展角色提供长期 memory/backlog。
- 将 `.pm/` 与现有 `doc/` 正式文档体系分层，确保“运行态项目管理”和“正式规格文档”各自收口。

## 当前现状
- 当前仓库已有：
  - `doc/**/prd.md` 与专题 `*.prd.md`：负责 Why/What/Done。
  - `doc/**/project.md` 与专题 `*.project.md`：负责 How/When/Who。
  - `.pm/tasks/TASK-PM-*.execution.md`：负责 task-local 过程记录。
  - `.agents/roles/*.md`：负责 7 个标准角色职责边界。
  - `AGENTS.md` 与 worktree 脚本：负责 owner role、handoff 和隔离执行。
- 当前缺口：
  - 没有角色长期 memory namespace。
  - 没有 role backlog/source signal/stage gate 的统一运行态结构。
  - QA/liveops 信号仍主要依赖人工阅读日志或正式文档后再整理。
  - 阶段评审输入分散在多个文档与证据文件中，无法一键汇总。

## 目标完成态
- 在仓库根目录建立 `.pm/`：
```text
.pm/
  roles/
    producer_system_designer/
      memory/
        active.yaml
        superseded.yaml
      backlog/
        candidate.yaml
        committed.yaml
        blocked.yaml
        done.yaml
    runtime_engineer/
    wasm_platform_engineer/
    agent_engineer/
    viewer_engineer/
    qa_engineer/
    liveops_community/
  tasks/
    TASK-PM-0001.yaml
    TASK-PM-0001.execution.md
  inbox/
    signals.jsonl
  stage/
    current.yaml
    gate.yaml
  registry/
    roles.yaml
    tasks.yaml
  templates/
    task.yaml
    memory.yaml
    signal.json
```
- 在 `scripts/pm/` 建立脚本入口：
  - `scaffold.sh`
  - `new-task.sh`
  - `promote-signal.sh`
  - `lint.sh`
  - `stage-report.sh`
  - `role-report.sh`
  - `workflow-report.sh`

## 对象设计
### 1. Role Registry
- 文件：`.pm/registry/roles.yaml`
- 用途：统一枚举当前启用角色及其存储路径。
- 最小字段：
  - `role_name`
  - `memory_active_path`
  - `memory_superseded_path`
  - `backlog_paths`
  - `is_active`
  - `introduced_at`

### 2. Role Memory
- 文件：`.pm/roles/<role>/memory/active.yaml`、`.pm/roles/<role>/memory/superseded.yaml`
- 设计原则：
  - active 和 superseded 分文件，避免高频冲突。
  - 每条记录必须带 source refs 和时间范围。
  - 不做全文 RAG；首期只做可审计、可 lint 的结构化记录。
- 最小字段：
  - `id`
  - `topic`
  - `summary`
  - `source_refs`
  - `effective_at`
  - `last_reviewed_at`
  - `status`
  - `superseded_by`

### 3. Role Backlog
- 文件：`.pm/roles/<role>/backlog/{candidate,committed,blocked,done}.yaml`
- 状态固定：
  - `candidate`
  - `committed`
  - `blocked`
  - `done`
  - `deferred` 通过条目字段表达，不单独拆文件
- 最小字段：
  - `task_id`
  - `title`
  - `priority`
  - `source_signal`
  - `related_prd`
  - `acceptance`
  - `handoff_to`
  - `status`

### 4. Signal Inbox
- 文件：`.pm/inbox/signals.jsonl`
- 设计原则：
  - 追加写入，适合事件流。
  - 由 promotion 脚本决定是否进入长期 memory 或 task registry。
- 最小字段：
  - `signal_id`
  - `source_type`
  - `source_ref`
  - `role_hint`
  - `severity`
  - `summary`
  - `promotion_state`

### 5. Task Registry
- 文件：`.pm/tasks/TASK-PM-*.yaml` 与 `.pm/registry/tasks.yaml`
- 设计原则：
  - 一任务一文件，降低 worktree 并发冲突。
  - registry 只做索引，不重复完整任务正文。
- 最小字段：
  - `task_id`
  - `owner_role`
  - `status`
  - `priority`
  - `source_refs`
  - `doc_refs`
  - `acceptance`
  - `updated_at`

### 6. Stage / Gate
- 文件：`.pm/stage/current.yaml`、`.pm/stage/gate.yaml`
- 用途：
  - 汇总当前阶段、claim envelope、lane 状态、blocking tasks。
  - 作为制作人阶段评审和对外口径复核的输入层。
 - 约束：
   - `.pm/stage/*.yaml` 是阶段“当前态”唯一真值；producer/shared active memory 只保留裁决依据或快照，不再单独定义当前阶段。
   - producer 修改阶段结论时统一通过 `set-stage.sh` 写回 `current/gate` 两份文件，并同步 `updated_from`、`decision_date` 与 blocker 集。
   - lint 必须阻断“active memory 仍声称存在 `stage.current` / `gate.claim_envelope`，但 stage 文件为空或缺来源”的漂移状态。

## 流程设计
### Flow A: task execution log 提升
1. 角色完成任务并写 `.pm/tasks/TASK-PM-XXXX.execution.md`
2. `promote-signal.sh` 把高价值条目写入 `.pm/inbox/signals.jsonl`
3. owner 决定：
   - 提升为 role memory
   - 提升为 candidate task
   - 标记为 discarded/deferred

### Flow B: QA / LiveOps 反馈回流
1. QA 或 liveops 写入 signal
2. script / owner 把信号归到对应 role hint
3. 若影响阶段或对外口径，则同步更新 `.pm/stage/gate.yaml`
4. producer 在阶段评审时读取汇总报告

### Flow C: 结论 supersede
1. 新结论进入 active memory
2. 旧结论转入 superseded
3. 写入 `superseded_by`
4. lint 校验链路和 source refs 仍有效

### Flow D: 工作流接入
1. owner 在新 task worktree 中执行 `workflow-report.sh --phase start --role <owner> --task-id <TASK-ID>`
2. 脚本先聚合 role backlog、memory stale、pending signals 与 stage/gate 摘要，构建 report/checklist 成功后再把 `last_started_at` 回写到 task file，避免失败时留下假证据
3. owner 开发完成后执行 `workflow-report.sh --phase close --role <owner> --task-id <TASK-ID>`，按 checklist 回写 task execution log、signal、memory 与 backlog；其中 working_memory 提示按当前 task 统计，零条目时先暴露 `codex-working-memory` bootstrap 入口，再在 commit 前启动独立 subagent review 当前 diff
4. 该 review 属于仓库默认 close 流程，不需要仅因执行这一步再单独向用户申请
5. owner 先处理或记录 subagent review findings，再提交 commit
6. producer 或 owner 在阶段评审前执行 `workflow-report.sh --phase review --role <owner>`，作为统一评审入口；其中 producer 的 review 额外聚合全部角色 pending signals，而已 `promoted/rejected/deferred` 的 signal 不再计入 pending

## 分阶段实施
### Phase 1: 骨架
- 建 `.pm/` 目录与 registry/template
- 打通 lint 和 scaffold

### Phase 2: 信号与任务
- 打通 signal inbox 与 task registry
- 优先服务 `qa_engineer` / `liveops_community`

### Phase 3: 全角色 memory/backlog
- 为 7 个角色全部落位
- 建立 superseded 规则

### Phase 4: 阶段评审
- 建立 `stage-report.sh`
- 把阶段输入收敛成可审计文件

### Phase 5: 角色视图
- 建立 `role-report.sh`
- 让每个 owner 可以直接读取本角色 backlog、blocked tasks、active memory 与 `needs_review` 清单

### Phase 6: 工作流接入
- 建立 `workflow-report.sh`
- 建立 `set-stage.sh` / `stage-lint`，把阶段当前态与 drift 检查收敛到正式入口
- 将 `.pm` 默认操作序列接入 `AGENTS.md`、角色职责卡与 `new-task-worktree.sh`
- 在 close checklist 中强制加入 commit 前独立 subagent review 当前 diff 的动作
- required/full smoke 必须覆盖 `workflow-report --task-id` 留痕、stage drift 阻断与 signal pending 视图

## 验证策略
- 结构验证：
  - `scripts/pm/lint.sh`
  - `git diff --check`
  - `./scripts/doc-governance-check.sh`
- 功能验证：
  - 手工构造 signal -> promote -> task/memory -> workflow/role/stage report 样例链路
  - 验证新角色注册无需修改历史 schema
- 回归验证：
  - 确认 `.pm/` 不与 `doc/` 形成重复真值
  - 确认 worktree 间修改可通过一任务一文件模型减冲突

## 风险与缓解
- 风险：`.pm/` 退化成另一份 `devlog`
  - 缓解：所有记录必须结构化并带状态，不允许自由流水文本替代 task/memory 对象。
- 风险：`.pm/` 退化成另一份 `project.md`
  - 缓解：明确 `.pm/` 是运行态，正式规格和正式任务定义仍留在 `doc/`。
- 风险：角色扩容时 schema 失控
  - 缓解：registry 驱动角色接入，文件结构不编码固定 7 角色。

## 交付物
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/devlog/2026-03-30.md`
