# oasis7：自我进化文件化项目管理设计（2026-03-30）

- 历史需求锚点: `PRD-ENGINEERING-021`；本文件仅保留 2026-03 `.pm` object-model 背景，不定义当前开发与交付规则。
- 当前 workflow 真值: `doc/engineering/workflow/source-of-truth.md`

审计轮次: 7

## 1. 历史设计定位

本文件保留 2026-03 `.pm` 对象模型的历史背景。其任务身份、状态、memory 自动转换和准入实现已经退出；现有用户资料原地保留为普通可读材料，不承担开发许可或完成证明。当前开发与交付仅由 [开发流程规范](../workflow/source-of-truth.md) 定义。


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

- 设计原则:
  - retired `.pm/inbox/signals.jsonl` 不得重建
  - `capture-todo.sh` / `promote-signal.sh` 创建或更新 GitHub-backed reflection intake
  - promotion 决定是否进入长期 memory 或候选 task
- 最小字段:
  - `signal_id`
  - `source_type`
  - `source_ref`
  - `role_hint`
  - `severity`
  - `summary`
  - `promotion_state`

### 2.5 Task Registry

- current task truth: GitHub Issue + GitHub Project item + `task_uid` mapping
- generated/archive paths: `.pm/github-project-sync/tasks.json`、`.pm/github-project-sync/task-archive.jsonl`
- 约束:
  - `task_uid` 仍是稳定内部身份，GitHub issue number / Project item id 是外部对象 handle
  - `.pm/github-project-sync/tasks.json` 是 generated mapping cache，可缺失或从 GitHub/task evidence 刷新
  - `.pm/github-project-sync/task-archive.jsonl` 是历史 task metadata / evidence audit bridge，不是 planning queue
  - 旧 `.pm/tasks/<task_uid>.yaml` / `.execution.md` 不再作为新任务真值或 execution evidence sink
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

1. 角色把 execution evidence 写入 PR、实际 CI 与评审记录（Issue 按需）。
2. `capture-todo.sh` / `promote-signal.sh` 将高价值条目创建为 GitHub-backed reflection intake。
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
   - 先聚合 工作说明或按需 Issue state、memory stale、reflection intake 与 stage/gate 摘要
   - workflow start evidence 写入 PR、实际 CI 与评审记录（Issue 按需）；旧 `last_started_at` task-file 写法只作迁移前背景
2. `workflow-report.sh --phase close --role <owner> --task-uid <task_uid>`
   - 回写 PR、实际 CI 与评审记录（Issue 按需），并按需更新 memory、GitHub-backed reflection intake 与 generated views
   - working memory 为空时暴露 bootstrap 入口，而不是静默跳过
4. `workflow-report.sh --phase review --role <owner>`
   - producer 额外聚合全部角色 pending signals
5. `sync-views.sh` 在需要时重建 registry/backlog 本地视图

## 4. Script Surface

- `scaffold.sh`: 建 `.pm/` 骨架与模板
- `new-task.sh`: 通过 工作说明或按需 Issue lifecycle 创建或绑定 task truth
- `promote-signal.sh`: 将高价值条目送入 GitHub-backed reflection intake
- `sync-views.sh`: 从 GitHub-backed mapping / archive / intake mirror 重建本地视图
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

- 看当前 task truth / evidence / PR readiness 规则: `doc/engineering/workflow/source-of-truth.md`
- 看 `.pm/` 仍然需要遵守的对象与流程约束: 本文档
- 看历史背景与 PRD-ENGINEERING-021 追溯锚点: 本文档。
