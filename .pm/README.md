# oasis7 文件化项目管理运行层

`doc/` 保存正式规格、项目追踪、证据和 devlog。`.pm/` 只保存运行态项目管理对象：

- role memory / backlog
- shared memory
- signal inbox
- task registry
- stage / gate
- 模板与脚本输入输出契约

约束：
- `.pm/` 不得重写正式 `prd.md` / `project.md` 真值。
- `devlog` 继续保存原始事件流；长期 memory/backlog 由后续任务负责从 signal 提升。
- 首批角色以 `.agents/roles/*.md` 为单一事实源。

首批标准角色：
- `producer_system_designer`
- `runtime_engineer`
- `wasm_platform_engineer`
- `agent_engineer`
- `viewer_engineer`
- `qa_engineer`
- `liveops_community`

角色扩容规则：
- 先在 `.agents/roles/<role>.md` 建立正式职责卡，再进入 `.pm/`。
- 再将角色登记到 `.pm/registry/roles.yaml`，保持按 `role_name` 排序。
- 再执行 `./scripts/pm/scaffold.sh <role_name>` 生成 role memory/backlog 容器。
- 最后执行 `./scripts/pm/lint.sh`，确认 registry、模板与路径全部可枚举。

当前已落地的 Phase 2 基础链路：
- `./scripts/pm/promote-signal.sh`：把高价值信号写入 `.pm/inbox/signals.jsonl`。
- `./scripts/pm/new-task.sh`：从 signal 或手工输入创建 `.pm/tasks/TASK-PM-*.yaml`，并同步更新 task registry 与 owner 的 `backlog/candidate.yaml`。

QA / liveops 基础用法：
- `./scripts/pm/promote-signal.sh --source-type devlog --source-ref doc/devlog/2026-03-30.md --role-hint qa_engineer --severity high --summary "viewer smoke blocked on startup" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md --acceptance "candidate task exists in qa backlog"`
- `./scripts/pm/promote-signal.sh --source-type incident --source-ref doc/devlog/2026-03-30.md --role-hint liveops_community --severity medium --summary "community feedback needs follow-up" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
