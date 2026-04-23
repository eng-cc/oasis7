# oasis7 文件化项目管理运行层

`doc/` 保存正式规格、项目追踪、证据与历史归档；`.pm/` 保存运行态项目管理对象与 task-local execution log：

- role memory / backlog
- task execution log
- task-scoped working_memory
- shared memory
- signal inbox
- task registry
- stage / gate
- 模板与脚本输入输出契约

约束：
- `.pm/` 不得重写正式 `prd.md` / `project.md` 真值。
- `.pm/tasks/task_<32hex>.execution.md` 是任务过程日志的 canonical 位置；长期 memory/backlog 通过对应 promote/move 脚本从 signal 或 task registry 视图提升。
- task 的唯一身份是 `task_uid`；`.pm/registry/tasks.yaml` 与 role backlog 只保留可扫描重建视图，并作为 git-ignored 的本地生成文件存在，不再承担仓库提交真值。
- stage/gate、signal、task `source_refs` 与 memory `source_refs` 不得再把 `doc/devlog/*.md` 当运行态 source_ref；历史 `doc/devlog/*.md` 仅作归档参考，运行态证据统一来自 task execution log、正式文档或其他显式 evidence。
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
- `./scripts/pm/new-task.sh`：从 signal 或手工输入创建 `.pm/tasks/task_<32hex>.yaml` 与对应 `.pm/tasks/task_<32hex>.execution.md`，并重建 task registry 与 owner 的 `backlog/candidate.yaml` 视图。
- `./scripts/new-task-worktree.sh --pm-owner-role ... --pm-title ... --pm-source-ref ...`：在创建 task worktree 的同时，切到目标 worktree 内原子完成 `new-task -> move-task committed -> workflow-report start`，避免 `.pm` task 误写回 source worktree。
- `./scripts/pm/move-task.sh`：在 `candidate/committed/blocked/done(deferred)` 之间同步迁移 task file、task registry 与 owner backlog 条目。
- `./scripts/pm/task-closeout.sh`：默认 close-phase helper；在 task 已 start 且 execution log 已回写后，统一执行 `workflow-report close -> move-task done|deferred -> pm lint`，再进入 commit / `prepare-task-pr.sh`。
- `./scripts/pm/task-execution-log-lint.sh`：校验 task execution log 的路径、标题格式、角色名和条目完整性。
- `./scripts/pm/promote-memory.sh`：从 signal 提升 active memory，或显式将噪声 signal 标记为 rejected / deferred。
- `./scripts/pm/supersede-memory.sh`：将 active memory 迁移到 superseded 文件，并补 `superseded_by` / `superseded_at` / `supersede_reason`。
- `./scripts/pm/memory-lint.sh`：校验 role/shared memory 的字段完整性、source refs、active topic 冲突与 superseded 链。
- `./scripts/pm/memory-report.sh`：按 role 输出 active / needs_review / superseded 报表，默认以 7 天未 review 记为 `needs_review`。
- `./scripts/pm/working-memory-lint.sh`：校验 `.pm/working_memory/*.yaml` 的 task/role/header、entry kind、source refs 与时间字段。
- `./scripts/pm/working-memory-report.sh`：按 task/role 输出 task-scoped `working_memory` 报表。
- `./scripts/pm/codex-transcript-report.sh`：优先从 `~/.codex/session_index.jsonl` / `history.jsonl` 读取单个 `session_id`；若 `history.jsonl` 无该会话消息，则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，只做排序与脱敏预处理。
- `./scripts/pm/codex-working-memory.sh`：先跑 `codex-transcript-report`，再调用 `codex exec --ephemeral` 把脱敏 transcript 提炼成 `working_memory` 条目；默认要求显式 `--session-id`，避免隐式读取当前 live Codex session。
- `./scripts/pm/working-memory-to-signal.sh`：把选中的 `working_memory` 条目提升成 `source_type=reflection` signal，并回写 `promoted_to`。
- `./scripts/pm/working-memory-autoflow.sh`：按安全默认值把 `working_memory` 自动提升成 reflection signal，并将 `next_step/open_question` 自动落成 candidate task。
- `./scripts/pm/reflection-report.sh`：按角色查看 reflection signal 队列，以及每条 signal 已挂出的 candidate task。
- `./scripts/pm/role-report.sh`：按角色汇总 backlog 状态、任务列表，以及该角色的 active / needs_review / superseded memory。
- `./scripts/pm/set-stage.sh`：统一更新 `.pm/stage/current.yaml` 与 `.pm/stage/gate.yaml`，作为 producer 修改阶段当前态的 canonical 入口。
- `./scripts/pm/stage-lint.sh`：校验 stage/gate 文件完整性、blocking task 可达性，以及 active memory 与 stage 当前态是否漂移。
- `./scripts/pm/stage-report.sh`：汇总 `.pm/stage/*.yaml`、blocked tasks、role backlog 计数，以及 producer/shared active memory，供阶段评审读取。
- `./scripts/pm/workflow-report.sh`：按 `start / close / review` 三种 phase 汇总 role backlog、memory、signal inbox 与 stage/gate 摘要，并给出固定 checklist；`start/close + --task-uid` 会把执行证据写回 task file，并在输出里带出 `execution_log_path`。
- `./scripts/pm/sync-views.sh`：从 `.pm/tasks/*.yaml` 扫描重建本地 task registry 与 role backlog 视图；lint/report/read-path 会在需要时自动刷新这些 git-ignored 视图。
- `./scripts/pm/migrate-task-identity.sh`：将旧的 `TASK-PM-xxxx` task/working_memory/source_ref 一次性迁到 `task_uid` canonical 模型，并重建 registry/backlog 视图。
- `./scripts/pm/required-tier-smoke.sh`：在临时 PM 根目录里跑一条 `seed evidence -> task execution log -> signal -> task -> memory -> stage report` required-tier 验证链。
- `./scripts/pm/memory-regression-smoke.sh`：在临时 PM 根目录里跑 `needs_review` / active 冲突 / superseded 链 / 新角色扩容的 full-tier 回归。

工作流接入基础用法：
- 开始任务：`./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>`
- 收口任务：优先 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID>`；若需要手工拆步，再执行 `./scripts/pm/workflow-report.sh --phase close --role <owner_role> --task-uid <TASK-UID>` + `./scripts/pm/move-task.sh --task-uid <TASK-UID> --to-status done|deferred`
- 阶段评审：`./scripts/pm/workflow-report.sh --phase review --role producer_system_designer`
- GitHub PR preflight / 默认评审边界：`./scripts/prepare-task-pr.sh`
- 开工前后都直接读写 `.pm/tasks/<TASK-UID>.execution.md`，不要再追加新的集中式 `doc/devlog/*.md`
- `producer_system_designer` 的 `review` 视图会汇总全部角色的 pending signals；其他角色的 `start/close/review` 仍默认只看本角色。
- `committed` 只表示任务已进入 owner backlog，不强制代表已经开工；但任务一旦进入 `blocked/done/deferred`，必须已有 `workflow-report --phase start --task-uid` 留下的 `last_started_at`，而 `done/deferred` 还必须已有 `last_closed_at`。
- 建议把 `workflow-report` 作为 worktree 创建后的第一条 PM 命令；收口时优先使用 `task-closeout.sh` 完成 close-phase，再在 commit 后立即进入 `prepare-task-pr.sh`，由 GitHub PR 的 required checks + review/approval 承担默认评审边界。`prepare-task-pr.sh` 还会基于当前 changed paths 给出一条本地 required 验证建议与 planner `reason_summary`，但这些输出只负责推荐/解释，不自动执行，也不改写 `./scripts/ci-tests.sh required/full` 的既有语义。
- 若 owner / title / source refs 已明确，优先直接用 `./scripts/new-task-worktree.sh <module> <task> --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref>` 一次性进入目标 worktree 并留下 `last_started_at`；只有在需要手工拆步时，才分开执行 `new-task.sh` / `workflow-report.sh` / `move-task.sh`，或显式跳过 `task-closeout.sh`。
- 默认最终合流路径是 GitHub PR；本地 `land-task-worktree.sh` 仅保留给显式 local-only / fallback 场景，不再是 `.pm` 默认收口路径。
- `.pm/registry/tasks.yaml` 与 `.pm/roles/*/backlog/*.yaml` 已降级为本地生成视图；它们会被 PM 命令自动刷新，但不应再作为 Git 冲突解决对象或人工真值手改。

QA / liveops 基础用法：
- `./scripts/pm/promote-signal.sh --source-type task_execution_log --source-ref .pm/tasks/task_<32hex>.execution.md --role-hint qa_engineer --severity high --summary "viewer smoke blocked on startup" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md --acceptance "candidate task exists in qa backlog"`
- `./scripts/pm/promote-signal.sh --source-type incident --source-ref .pm/tasks/task_<32hex>.execution.md --role-hint liveops_community --severity medium --summary "community feedback needs follow-up" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`

状态迁移基础用法：
- `./scripts/pm/move-task.sh --task-uid task_<32hex> --to-status committed`
- `./scripts/pm/move-task.sh --task-uid task_<32hex> --to-status deferred`
- `./scripts/pm/set-stage.sh --current-stage internal_playable_alpha_late --claim-envelope internal_only --decision-date 2026-03-31 --gate-status blocked --lane-status qa=blocked --blocking-task task_<32hex> --source-ref .pm/tasks/task_<32hex>.execution.md`
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

working_memory 基础用法：
- `./scripts/pm/working-memory-report.sh`
- `./scripts/pm/working-memory-report.sh --task-uid task_<32hex> --json`
- `./scripts/pm/codex-transcript-report.sh --session-id <session_id> --json`
- `./scripts/pm/codex-working-memory.sh --task-uid task_<32hex> --role producer_system_designer --session-id <session_id> --worktree-hint <hint>`
- `./scripts/pm/codex-working-memory.sh --task-uid task_<32hex> --role producer_system_designer --allow-auto-session --worktree-hint <hint>`
- `./scripts/pm/codex-transcript-report.sh --task-uid task_<32hex> --json`
- `./scripts/pm/codex-working-memory.sh --task-uid task_<32hex> --role producer_system_designer --session-id <session_id> --full-scan`
- `./scripts/pm/working-memory-to-signal.sh --task-uid task_<32hex> --entry-id WM-0001 --severity medium`
- `./scripts/pm/working-memory-autoflow.sh --task-uid task_<32hex> --severity medium --priority P2`
- `./scripts/pm/working-memory-autoflow.sh --task-uid task_<32hex> --dry-run --json`
- `./scripts/pm/reflection-report.sh --role producer_system_designer --json`
- phase 1 的 transcript 预处理只负责排序与脱敏；结构化提炼统一交给 `codex exec --ephemeral`。
- `codex-working-memory.sh` 默认不会仅凭 task/worktree 自动解析 `.codex` session；若确实要走 registry / worktree pattern 自动解析，必须显式传 `--allow-auto-session`。
- `codex-working-memory.sh` 首次成功导入后会把 `task_uid -> session_id` 记到 `.pm/registry/codex-sessions.yaml`；后续若要继续复用该 registry 映射，也必须显式传 `--allow-auto-session`，或直接给出新的 `--session-id`。
- 同一 `task_uid + session_id` 默认按 `working_memory` header 里的 `last_extracted_ts` 做增量抽取；这只在 owner 显式选择该 session 后生效，避免把当前 live session 的隐式自读当作默认收口路径。需要重扫整段 transcript 时显式传 `--full-scan`。
- `working_memory` header 会记录 `source_session_id`、`source_thread_name`、`transcript_source`、`last_extracted_ts` 与 `captured_until_ts`，用于回放抽取来源与当前水位。
- `working-memory-autoflow.sh` 只自动做安全动作：reflection signal + candidate task；不会自动升长期 memory，也不会自动改 stage / 正式文档。
- `working-memory-autoflow.sh --dry-run` 是严格只读的 plan 模式：它只返回“会创建/复用哪些 reflection signal 与 candidate task”，不会改 `.pm/inbox/signals.jsonl`、`.pm/working_memory/*.yaml`、task registry 或 task files。
- dry-run 结果里只有已存在对象才会带真实 `signal_id` / `task_uid`；若对象尚未创建，apply 之前不会预留 ID，也不会留下任何半完成状态。

role report 基础用法：
- `./scripts/pm/role-report.sh`
- `./scripts/pm/role-report.sh --role qa_engineer`
- `./scripts/pm/role-report.sh --role qa_engineer --json`
- 输出会同时带该角色 backlog 计数、任务列表，以及 active / needs_review / superseded memory。

workflow report 基础用法：
- `./scripts/pm/workflow-report.sh --phase start --role qa_engineer --task-uid task_<32hex>`
- `./scripts/pm/workflow-report.sh --phase close --role liveops_community --task-uid task_<32hex>`
- `./scripts/pm/workflow-report.sh --phase review --role producer_system_designer --json`
- `./scripts/prepare-task-pr.sh --json`
- 输出会同时带 backlog/memory 摘要、pending signals、stage/gate 摘要与推荐动作清单；其中 producer 的 `review` 会跨角色汇总 pending signals，`start/close` 若带 `--task-uid` 还会把 `last_started_at` / `last_closed_at` 写回 task file。

阶段汇总基础用法：
- `./scripts/pm/stage-lint.sh`
- `./scripts/pm/stage-report.sh`
- `./scripts/pm/stage-report.sh --json`

required-tier 验证入口：
- `./scripts/pm/required-tier-smoke.sh`
- `./scripts/pm/required-tier-smoke.sh --json`
- `./scripts/pm/new-task-worktree-bootstrap-smoke.sh`
- `./scripts/pm/new-task-worktree-bootstrap-smoke.sh --json`

full-tier 验证入口：
- `./scripts/pm/memory-regression-smoke.sh`
- `./scripts/pm/memory-regression-smoke.sh --json`
- `./scripts/pm/codex-working-memory-smoke.sh`
- `./scripts/pm/codex-working-memory-smoke.sh --json`
