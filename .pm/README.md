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
- `devlog` 继续保存原始事件流；长期 memory/backlog 通过对应 promote/move 脚本从 signal 或 task registry 提升。
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
- `./scripts/pm/move-task.sh`：在 `candidate/committed/blocked/done(deferred)` 之间同步迁移 task file、task registry 与 owner backlog 条目。
- `./scripts/pm/promote-memory.sh`：从 signal 提升 active memory，或显式将噪声 signal 标记为 rejected / deferred。
- `./scripts/pm/supersede-memory.sh`：将 active memory 迁移到 superseded 文件，并补 `superseded_by` / `superseded_at` / `supersede_reason`。
- `./scripts/pm/memory-lint.sh`：校验 role/shared memory 的字段完整性、source refs、active topic 冲突与 superseded 链。
- `./scripts/pm/memory-report.sh`：按 role 输出 active / needs_review / superseded 报表，默认以 7 天未 review 记为 `needs_review`。
- `./scripts/pm/stage-report.sh`：汇总 `.pm/stage/*.yaml`、blocked tasks、role backlog 计数，以及 producer/shared active memory，供阶段评审读取。
- `./scripts/pm/required-tier-smoke.sh`：在临时 PM 根目录里跑一条 `devlog -> signal -> task -> memory -> stage report` required-tier 验证链。
- `./scripts/pm/memory-regression-smoke.sh`：在临时 PM 根目录里跑 `needs_review` / active 冲突 / superseded 链 / 新角色扩容的 full-tier 回归。

QA / liveops 基础用法：
- `./scripts/pm/promote-signal.sh --source-type devlog --source-ref doc/devlog/2026-03-30.md --role-hint qa_engineer --severity high --summary "viewer smoke blocked on startup" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md --acceptance "candidate task exists in qa backlog"`
- `./scripts/pm/promote-signal.sh --source-type incident --source-ref doc/devlog/2026-03-30.md --role-hint liveops_community --severity medium --summary "community feedback needs follow-up" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`

状态迁移基础用法：
- `./scripts/pm/move-task.sh --task-id TASK-PM-0001 --to-status committed`
- `./scripts/pm/move-task.sh --task-id TASK-PM-0001 --to-status deferred`
- `./scripts/pm/promote-memory.sh --signal-id SIG-PM-0002 --role producer_system_designer --topic stage.current --promotion-reason stage_decision --tag stage --tag claim_envelope`
- `./scripts/pm/promote-memory.sh --signal-id SIG-PM-0003 --scope shared --role producer_system_designer --topic gate.claim_envelope --promotion-reason stage_decision`
- `./scripts/pm/promote-memory.sh --signal-id SIG-PM-0004 --role qa_engineer --reject-reason one_off_operation`
- `./scripts/pm/supersede-memory.sh --role qa_engineer --memory-id MEM-QA-0001 --superseded-by MEM-QA-0002 --supersede-reason signature_refined`

长期 memory promotion 约束：
- `promotion_reason` 白名单：`stage_decision`、`failure_signature`、`policy_boundary`、`stable_pattern`、`engineering_constraint`
- `reject_reason` 白名单：`one_off_operation`、`unverified_hypothesis`、`short_lived_execution_detail`、`task_status_update`
- `--scope shared` 仅允许 `producer_system_designer` 执行；shared 正式 memory 不接受其他角色直写

memory report 基础用法：
- `./scripts/pm/memory-report.sh`
- `./scripts/pm/memory-report.sh --role qa_engineer --no-shared`
- `./scripts/pm/memory-report.sh --stale-after-days 14 --json`
- 默认 stale 阈值为 7 天，对应长期 memory 每周至少 review 1 次的治理口径。

阶段汇总基础用法：
- `./scripts/pm/stage-report.sh`
- `./scripts/pm/stage-report.sh --json`

required-tier 验证入口：
- `./scripts/pm/required-tier-smoke.sh`
- `./scripts/pm/required-tier-smoke.sh --json`

full-tier 验证入口：
- `./scripts/pm/memory-regression-smoke.sh`
- `./scripts/pm/memory-regression-smoke.sh --json`
