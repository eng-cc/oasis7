# oasis7：自我进化文件化项目管理设计（2026-03-30）

- 对应需求文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- 对应项目管理文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`

审计轮次: 7

## 1. 设计定位

这份设计文档只保留 `.pm/` 运行层仍需要的结构设计与流程约束，不再重复 PRD 的背景、project 的 rollout、或历史阶段流水账。

`doc/**` 继续负责规格与计划，`.pm/` 只负责运行态对象、视图重建和流程留痕。

## 2. Canonical Object Model

### 2.1 Role Registry

- 文件: `.pm/registry/roles.yaml`
- 作用: 枚举当前启用角色及其存储路径
- 最小字段:
  - `role_name`
  - `memory_active_path`
  - `memory_superseded_path`
  - `backlog_paths`
  - `is_active`
  - `introduced_at`

### 2.2 Role Memory

- 文件: `.pm/roles/<role>/memory/{active,superseded}.yaml`
- 约束:
  - active / superseded 分文件
  - 每条记录必须带 `source_refs`
  - 不做全文 RAG，只保留可审计结构化记录
- 最小字段:
  - `id`
  - `topic`
  - `summary`
  - `source_refs`
  - `effective_at`
  - `last_reviewed_at`
  - `status`
  - `superseded_by`

### 2.3 Role Backlog

- 文件: `.pm/roles/<role>/backlog/{candidate,committed,blocked,done}.yaml`
- 视图策略:
  - 这些 backlog 文件是 git-ignored 本地生成视图
  - 缺失时由 `sync-views.sh` 或任一 PM 读写命令自动重建
- 固定状态:
  - `candidate`
  - `committed`
  - `blocked`
  - `done`
  - `deferred` 通过条目字段表达，不单独拆文件
- 最小字段:
  - `task_uid`
  - `title`
  - `priority`
  - `source_signal`
  - `related_prd`
  - `acceptance`
  - `handoff_to`
  - `status`

### 2.4 Signal Inbox

- 文件: `.pm/inbox/signals.jsonl`
- 设计原则:
  - 只追加写入
  - promotion 决定是否进入长期 memory 或 task registry
- 最小字段:
  - `signal_id`
  - `source_type`
  - `source_ref`
  - `role_hint`
  - `severity`
  - `summary`
  - `promotion_state`

### 2.5 Task Registry

- canonical object: `.pm/tasks/<task_uid>.yaml`
- 重建视图: `.pm/registry/tasks.yaml`
- 约束:
  - 任务主键真值只存在于 canonical task file
  - `task_uid` 本地生成，不依赖顺序号
  - registry / backlog 只做扫描重建视图，不承担主键真值
  - `.pm/registry/tasks.yaml` 与 backlog 视图均为 git-ignored 本地缓存
- 最小字段:
  - `task_uid`
  - `owner_role`
  - `status`
  - `priority`
  - `source_refs`
  - `doc_refs`
  - `acceptance`
  - `updated_at`

### 2.6 Stage / Gate

- 文件: `.pm/stage/current.yaml`、`.pm/stage/gate.yaml`
- 作用:
  - 汇总当前阶段、claim envelope、lane 状态与 blocking tasks
  - 作为阶段评审与对外口径复核输入
- 约束:
  - `.pm/stage/*.yaml` 是阶段当前态唯一真值
  - producer/shared active memory 只保留裁决依据，不单独定义当前阶段
  - producer 修改阶段时统一通过 `set-stage.sh`
  - lint 必须阻断 stage 文件与 memory/claim 口径漂移

## 3. Workflow Integration

### 3.1 Signal -> Memory / Task

1. 角色写 `.pm/tasks/<task_uid>.execution.md`
2. `promote-signal.sh` 提炼高价值条目写入 `.pm/inbox/signals.jsonl`
3. owner 决定将 signal 提升为:
   - role memory
   - candidate task
   - discarded / deferred

### 3.2 QA / LiveOps Feedback

1. QA 或 liveops 写 signal
2. script / owner 归到对应 `role_hint`
3. 若影响阶段或对外口径，同步更新 `.pm/stage/gate.yaml`
4. producer 在 review 阶段读取汇总报告

### 3.3 Supersede Chain

1. 新结论进入 active memory
2. 旧结论转入 superseded
3. 记录 `superseded_by`
4. lint 校验链路和 `source_refs`

### 3.4 Workflow Report Hookup

1. `workflow-report.sh --phase start --role <owner> --task-uid <task_uid>`
   - 先聚合 backlog、memory stale、pending signals 与 stage/gate 摘要
   - 成功构建报告后再写 `last_started_at`
2. `workflow-report.sh --phase close --role <owner> --task-uid <task_uid>`
   - 回写 task execution log、signal、memory 与 backlog
   - working memory 为空时暴露 bootstrap 入口，而不是静默跳过
3. commit 后通过 `./scripts/prepare-task-pr.sh` 进入 GitHub PR review
4. `workflow-report.sh --phase review --role <owner>`
   - producer 额外聚合全部角色 pending signals
5. `sync-views.sh` 在需要时重建 registry/backlog 本地视图

## 4. Script Surface

- `scaffold.sh`: 建 `.pm/` 骨架与模板
- `new-task.sh`: 创建 canonical task file
- `promote-signal.sh`: 将高价值条目送入 signal inbox
- `sync-views.sh`: 从 canonical task files 重建 backlog / registry 本地视图
- `lint.sh`: 校验字段、链路、source refs 与 stage drift
- `stage-report.sh`: 聚合阶段视图
- `role-report.sh`: 聚合角色 backlog / memory / stale
- `workflow-report.sh`: 统一 start / close / review 三段入口

## 5. 风险控制

- `.pm/` 若退化成自由文本日记层: 用结构化字段和 lint 阻断
- `.pm/` 若与 `doc/` 形成重复真值: 保持“运行态 vs 正式规格”分层
- 角色扩容导致 schema 失控: 通过 registry 驱动接入，不把角色数量编码进文件结构
- worktree 并发冲突回弹: 只允许 canonical task object 冲突，视图文件必须可删可重建

## 6. 使用方式

- 看正式规格与验收: `file-based-self-evolution-management-2026-03-30.prd.md`
- 看 rollout / follow-up: `file-based-self-evolution-management-2026-03-30.project.md`
- 看 `.pm/` 仍然需要遵守的对象与流程约束: 本文档
