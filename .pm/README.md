# oasis7 GitHub Project-backed PM 层

`doc/` 保存正式规格、项目追踪、证据与历史归档。项目管理正在从文件化 `.pm` task
迁移到 GitHub Issues + GitHub Project；Step 3 已删除仓库内 `.pm/tasks/*` task 文件。
GitHub Project 是 active work queue / external state gate，`.pm/` 只保留生成缓存、历史归档
与尚未迁出的 PM 对象：

- role memory / backlog
- historical task archive
- task-scoped working_memory
- shared memory
- GitHub-backed reflection intake
- stage / gate
- 模板与脚本输入输出契约

约束：
- `.pm/` 不得重写正式 `prd.md` / `project.md` 真值。
- GitHub Project 是 active task queue / status cockpit 的 authoritative 入口；`.pm/github-project-sync/tasks.json` 只是可重新生成的 `task_uid -> issue/project item` 本地 mapping cache，普通 PR 不应为了 task-local 状态变更提交它。
- `.pm/github-project-sync/task-archive.jsonl` 保存 Step 3 删除前的 task yaml 与 execution log 全量归档；它是审计桥，不是新的本地 planning queue。
- `.pm/tasks/task_<32hex>.yaml` 与 `.pm/tasks/task_<32hex>.execution.md` 已退休；正常工作流不得重新创建这些 task 文件。
- 本 README 后续仍提到旧 `.pm/tasks` 的脚本，均为 legacy 文件化 PM 入口或历史示例；除非 `source-of-truth` 重新批准，不再作为新工作流真值。
- task 的唯一身份是 `task_uid`；GitHub issue number / Project item id 只是外部对象句柄，不替代 `task_uid`。
- `.pm/registry/tasks.yaml` 与 role backlog 只保留可扫描重建视图，并作为 git-ignored 的本地生成文件存在，不再承担 planning queue 真值。
- `./scripts/pm/github-project-workflow.sh ... audit` 必须能以 selected-task / mapping-targeted 低成本路径验证 GitHub Project、mapping 与 archive/mirror 一致；普通 closeout / PR readiness 不得全量拉取 Project items。`step3-gate` 是全历史覆盖检查，可全量拉取 Project。
- 同一 owner / 同一工作流下若出现仅承担 truth refresh、doc sync 或中段 burn-down 留痕的已关闭微任务，必须先把 `project.md` / topic project 的 Trace 收口到 survivor task，再通过 `./scripts/pm/compact-task-group.sh` 并档；不允许在正式文档仍引用 dropped task UID 时直接删除 canonical task 文件。
- stage/gate、signal、task `source_refs` 与 memory `source_refs` 不得再把 `doc/devlog/*.md` 当运行态 source_ref；历史 `doc/devlog/*.md` 仅作归档参考，运行态证据统一来自 GitHub task issue evidence comments、正式文档或其他显式 evidence。
- 首批角色以 `.agents/roles/*.md` 为单一事实源。

首批标准角色：
- `producer_system_designer`
- `gameplay_designer`
- `game_visual_interaction_designer`
- `runtime_engineer`
- `blockchain_ops_engineer`
- `wasm_platform_engineer`
- `agent_engineer`
- `viewer_engineer`
- `qa_engineer`
- `repository_health_engineer`
- `liveops_community`

角色扩容规则：
- 先在 `.agents/roles/<role>.md` 建立正式职责卡，再进入 `.pm/`。
- 再将角色登记到 `.pm/registry/roles.yaml`，保持按 `role_name` 排序。
- 再执行 `./scripts/pm/scaffold.sh <role_name>` 生成 role memory/backlog 容器。
- 最后执行 `./scripts/pm/lint.sh`，确认 registry、模板与路径全部可枚举。

当前 GitHub-backed active 基础链路：
- `./scripts/pm/github-project-task.py`：GitHub Project-backed active lifecycle adapter；创建 task issue/project item、写 issue evidence comment、更新 Project 状态、执行 closeout evidence。
- `./scripts/pm/github-project-workflow.sh`：GitHub Project-backed PM adapter；`sync` 将 `.pm` mirror 或 Step 3 archive 推到 GitHub Issue/Project，`audit` 以 selected-task / mapping-targeted 低成本路径校验 Project/mapping/archive 漂移，`step3-gate` 是全历史硬 gate。
- `./scripts/pm/github-project-sync.sh`：底层 `.pm/archive -> GitHub Issues/Project` 幂等同步器，由 `github-project-workflow sync` 调用。
- `./scripts/pm/github-project-retire-tasks.sh`：Step 3 归档/删除工具；先导出 `.pm/github-project-sync/task-archive.jsonl`，再删除 `.pm/tasks/*`。
- `./scripts/pm/capture-todo.sh`：把还没决定创建 task 的顺手 TODO / discovery 记录为 GitHub-backed `source_type=reflection` intake issue；显式 `--create-task` 时才提升成 candidate task。
- `./scripts/pm/promote-signal.sh`：创建 GitHub-backed reflection intake issue，并刷新 ignored generated mirror `.pm/github-project-sync/intake-signals.json` 供 `reflection-report` / `workflow-report` 本地查看；带 `--create-task` 时再创建 GitHub-backed candidate task 并保留 `source_signal` / intake issue 回链。
- `.pm/github-project-sync/signal-archive.jsonl`：`.pm/inbox/signals.jsonl` 退休时的只读历史归档；active runtime 不再读写 `.pm/inbox/signals.jsonl`。
- `./scripts/pm/new-task.sh`：创建 GitHub Issue + GitHub Project item，可刷新本地 `.pm/github-project-sync/tasks.json` mapping cache，不创建 `.pm/tasks` 文件。
- `./scripts/new-task-worktree.sh --pm-owner-role ... --pm-title ... --pm-source-ref ...`：在创建 task worktree 的同时，切到目标 worktree 内原子完成 `new-task -> move-task committed -> workflow-report start`，证据写入 GitHub issue comment。
- `./scripts/pm/move-task.sh`：在 `candidate/committed/blocked/ready/pr_watch/done/deferred` 之间同步更新 GitHub Project Status / PM Status / Workflow Phase 与 mapping。
- `./scripts/pm/task-closeout.sh`：默认 ready-for-PR close-phase helper；在 fresh verification 通过后执行 `workflow-report close -> move-task ready`，证据进入 GitHub issue comment，再进入 commit / `prepare-task-pr.sh`；只有 post-PR merge/cleanup 或显式非 PR 任务才用 `--to-status done`。
- `./scripts/pm/fallback-evidence.sh`：GitHub issue comment 暂不可用时的 fallback packet create/audit/replay helper；回放前不算正式 task truth。
- `./scripts/pm/claim-ready.sh`：在宣称“完成 / 测试通过 / 可提 PR / 可合并”前立即执行 fresh verification command；有 task_uid 时把 verification 记录到 GitHub issue comment 和 mapping。
- `./scripts/pm/append-execution-log.sh`：结构化追加 GitHub issue evidence comment，要求显式 task_uid、role、完成内容、遗留事项、动作与验证结果。
- `./scripts/pm/task-execution-log-lint.sh`：legacy/local task-file lint，仅用于退役前数据或专门 fixture，不是 active GitHub-backed 证据入口。
- `./scripts/pm/promote-memory.sh`：从 signal 提升 active memory，或显式将噪声 signal 标记为 rejected / deferred。
- `./scripts/pm/supersede-memory.sh`：将 active memory 迁移到 superseded 文件，并补 `superseded_by` / `superseded_at` / `supersede_reason`。
- `./scripts/pm/memory-lint.sh`：校验 role/shared memory 的字段完整性、source refs、active topic 冲突与 superseded 链。
- `./scripts/pm/memory-report.sh`：按 role 输出 active / needs_review / superseded 报表，默认以 7 天未 review 记为 `needs_review`。
- `./scripts/pm/working-memory-lint.sh`：校验 `.pm/working_memory/*.yaml` 的 task/role/header、entry kind、source refs 与时间字段。
- `./scripts/pm/working-memory-report.sh`：按 task/role 输出 task-scoped `working_memory` 报表。
- `./scripts/pm/codex-transcript-report.sh`：优先从 `~/.codex/session_index.jsonl` / `history.jsonl` 读取单个 `session_id`；若 `history.jsonl` 无该会话消息，则 fallback 到 `~/.codex/sessions/**/rollout-*.jsonl`，只做排序与脱敏预处理。
- `./scripts/pm/codex-working-memory.sh`：先跑 `codex-transcript-report`，再调用 `codex exec --ephemeral` 把脱敏 transcript 提炼成 `working_memory` 条目；默认要求显式 `--session-id`，避免隐式读取当前 live Codex session。
- `./scripts/pm/working-memory-to-signal.sh`：旧本地 apply 路径已禁用；先用 `working-memory-autoflow.sh --dry-run` 规划，再用 `promote-signal.sh` 创建 GitHub-backed intake。
- `./scripts/pm/working-memory-autoflow.sh`：当前只保留 `--dry-run` 规划；apply 模式已禁用，避免复活 retired local signal/task 写入路径。
- `./scripts/pm/reflection-report.sh`：按角色查看 reflection signal 队列，以及每条 signal 已挂出的 candidate task。
- `./scripts/pm/role-report.sh`：按角色汇总本地生成视图、active / needs_review / superseded memory；带 `--task-uid` 时追加 task-centric collaboration view，辅助 owner 合流 GitHub issue evidence 与收口缺口。
- `./scripts/pm/set-stage.sh`：统一更新 `.pm/stage/current.yaml` 与 `.pm/stage/gate.yaml`，作为 producer 修改阶段当前态的 canonical 入口。
- `./scripts/pm/stage-lint.sh`：校验 stage/gate 文件完整性、blocking task 可达性，以及 active memory 与 stage 当前态是否漂移。
- `./scripts/pm/stage-report.sh`：汇总 `.pm/stage/*.yaml`、blocked tasks、role backlog 计数，以及 producer/shared active memory，供阶段评审读取。
- `./scripts/pm/workflow-report.sh`：按 `start / close / review` 三种 phase 汇总 role backlog、memory、GitHub-backed intake signal 视图与 stage/gate 摘要，并给出固定 checklist；`start/close + --task-uid` 会把执行证据写入 GitHub issue comment，并在 mapping 中更新 phase 时间戳。
- `./scripts/pm/sync-views.sh`：legacy/local-view helper；Step 3 后不再从 `.pm/tasks/*.yaml` 生成 active truth。
- `./scripts/pm/compact-task-group.sh`：legacy/local task-file consolidation helper；Step 3 后不得作为 active task 合并入口。
- `./scripts/pm/rebase-conflict-helper.sh`：在 active rebase 期间只读盘点 `.pm/**` 未合并路径；`.pm/inbox/signals.jsonl` 已退休，历史冲突只提示删除/人工归档，不再自动修复；若冲突命中 `.pm/registry/tasks.yaml` 或 `.pm/roles/*/backlog/*.yaml` 这类本地生成视图，helper 只提示“保留 `main` 删除，再执行 `./scripts/pm/sync-views.sh`”，不自动替用户覆盖 canonical task/memory/stage 真值。
- `./scripts/pm/migrate-task-identity.sh`：legacy migration helper；用于旧 `TASK-PM-xxxx` 数据迁移，不是 active task 创建入口。
- `./scripts/pm/required-tier-smoke.sh`：在临时 PM 根目录里跑一条 PM governance required-tier 验证链；fixture 中的 task-file 片段只验证 legacy compatibility，不代表 active evidence sink。
- `./scripts/pm/memory-regression-smoke.sh`：在临时 PM 根目录里跑 `needs_review` / active 冲突 / superseded 链 / 新角色扩容的 full-tier 回归。

工作流接入基础用法：
- GitHub Project active queue 同步：`./scripts/pm/github-project-workflow.sh --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 sync --json`
- GitHub Project 漂移审计（普通任务默认低成本 selected-task 路径）：`./scripts/pm/github-project-workflow.sh --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 audit --json`
- Step 3 全历史 gate（迁移/人工/定时全量审计，不进普通 PR 热路径）：`./scripts/pm/github-project-workflow.sh --repo eng-cc/oasis7 --project-owner eng-cc --project-number 1 step3-gate --json`
- Step 3 task 文件退休：`./scripts/pm/github-project-retire-tasks.sh --mapping .pm/github-project-sync/tasks.json --delete --json`
- 记录 pre-task TODO：`./scripts/pm/capture-todo.sh --source-ref <path> --summary "发现的问题/想法"`；默认 `role_hint=tpm`、`severity=low`、只创建 GitHub-backed reflection intake issue。若已经决定推进，再加 `--create-task --title ... --owner-role <role> --acceptance ...`。
- 创建任务：`./scripts/pm/new-task.sh --owner-role <owner_role> --title "<title>" --module <module> --source-ref <path> --json`
- 开始任务：`./scripts/pm/workflow-report.sh --phase start --role <owner_role> --task-uid <TASK-UID>`
- 收口任务：优先 `./scripts/pm/task-closeout.sh --role <owner_role> --task-uid <TASK-UID> --verify-command "<fresh verification command>"`，默认进入 `ready`；若需要手工拆步，再执行“fresh verification” + `./scripts/pm/workflow-report.sh --phase close --role <owner_role> --task-uid <TASK-UID>` + `./scripts/pm/move-task.sh --task-uid <TASK-UID> --to-status ready|done|deferred`
- fresh verification claim：`./scripts/pm/claim-ready.sh --claim-type ready_for_pr --verify-command "<fresh verification command>"`
- PM active lint：`./scripts/pm/lint.sh`；校验 GitHub Project mapping/archive、active lifecycle scripts、role registry、memory 与 stage 基础结构。
- 结构化追加 execution log：`./scripts/pm/append-execution-log.sh --task-uid <TASK-UID> --role <owner_role> --completed "..." --pending "..." --action "..." --validation-command "..." --expected-result "..." --actual-result "..." --blocker-next-action "..."`
- 阶段评审：`./scripts/pm/workflow-report.sh --phase review --role producer_system_designer`
- GitHub PR preflight / 默认 watch-fix-merge 边界：`./scripts/prepare-task-pr.sh`
- fallback evidence 回放：`./scripts/pm/fallback-evidence.sh replay --task-uid <TASK-UID>`；PR readiness lint 会拒绝未回放 fallback packet。
- 不再读写 `.pm/tasks/<TASK-UID>.execution.md`；新的任务过程证据应进入 GitHub Issue/Project-backed task envelope，或进入 source-of-truth 明确批准的替代 sink。
- `producer_system_designer` 的 `review` 视图会汇总全部角色的 pending signals；其他角色的 `start/close/review` 仍默认只看本角色。
- `committed` 只表示任务已进入 owner backlog，不强制代表已经开工；`ready` 表示本地 closeout 完成且准备创建 PR，`pr_watch` 表示 PR 已创建且仍在 CI/comments/merge watch 主链；任务一旦进入 `blocked/ready/pr_watch/done/deferred`，必须已有 `workflow-report --phase start --task-uid` 留下的 `last_started_at`，而 `done/deferred` 还必须已有最终完成 closeout evidence。
- 建议把 `workflow-report` 作为 worktree 创建后的第一条 PM 命令；收口时按 `fresh verification -> pre-PR local role subagent review -> findings 处置 -> task-closeout.sh close-phase -> commit -> prepare-task-pr.sh` 顺序推进。普通 PR 创建后默认继续盯 GitHub required checks、mergeability、PR comments 与 unresolved review threads；`REVIEW_REQUIRED` 只作为状态信息回报，不是 block 项。若 `mergeStateStatus=BLOCKED` 仅因缺少 review approval，且用户/task policy 明确授权跳过 approval，则在复查 checks、mergeability、requested changes、comments/thread 后可作为正常流程使用 repo admin merge path。checks 失败、requested changes、不可合并、存在 actionable comments / unresolved blocking threads，或非 review-approval 原因的 GitHub merge API/branch protection 实际拒绝时，才修复/验证/推送或回复/resolve；通过且 comments/thread 已收口后合入并清理。只有明确用于 manual packaging/release CI 的 PR 才能停在人工打包 gate。`prepare-task-pr.sh` 还会基于当前 changed paths 给出一条本地 required 验证建议与 planner `reason_summary`，但这些输出只负责推荐/解释，不自动执行，也不改写 `./scripts/ci-tests.sh required/full` 的既有语义。
- `./scripts/pm/workflow-behavior-eval.sh`：repo-owned workflow behavior eval 入口；把 task-worktree bootstrap、可选/必需 routing scenarios、subagent contract surface、PM closeout/claim gate、PR preflight 与 review-thread closeout 串成一条可重复的本地验证链。
- role subagent 产出的 patch、review card、summary、incident note 或 messaging brief，只有在被 owner 回写到 `project.md`、handoff、GitHub issue evidence comment、signal/memory，或 PR evidence 中至少一处后，才算进入 canonical 主链；孤立产物本身不构成正式收口证据。
- 若 slice 类型是 `liveops_feedback`、`supplemental_review` 或其他非代码反馈，收口前仍必须明确它的 formal sink：至少要么 `promote-signal` / `promote-memory`，要么在 GitHub issue evidence comment / PR evidence 中留下可追溯引用。
- 若 owner / title / source refs 已明确，优先直接用 `./scripts/new-task-worktree.sh <module> <task> --pm-owner-role <owner_role> --pm-title <title> --pm-source-ref <ref>` 一次性进入目标 worktree 并留下 `last_started_at`；只有在需要手工拆步时，才分开执行 `new-task.sh` / `workflow-report.sh` / `move-task.sh`，或显式跳过 `task-closeout.sh`。
- 默认最终合流路径是 GitHub PR；本地 `land-task-worktree.sh` 仅保留给显式 local-only / fallback 场景，不再是 `.pm` 默认收口路径。
- `.pm/registry/tasks.yaml` 与 `.pm/roles/*/backlog/*.yaml` 已降级为本地生成视图；它们会被 PM 命令自动刷新，但不应再作为 Git 冲突解决对象或人工真值手改。
- 若 rebase 命中 `.pm/**` 冲突，先运行 `./scripts/pm/rebase-conflict-helper.sh` 盘点类别；`.pm/inbox/signals.jsonl` 已退休，不再有 `.pm/**` 自动修复路径，canonical task/memory/stage 冲突仍需人工判断。

QA / liveops 基础用法：
- `./scripts/pm/promote-signal.sh --source-type issue_comment --source-ref https://github.com/eng-cc/oasis7/issues/<N>#issuecomment-<ID> --role-hint qa_engineer --severity high --summary "viewer smoke blocked on startup" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md --acceptance "candidate task exists in GitHub Project"`
- `./scripts/pm/promote-signal.sh --source-type incident --source-ref https://github.com/eng-cc/oasis7/issues/<N>#issuecomment-<ID> --role-hint liveops_community --severity medium --summary "community feedback needs follow-up" --create-task --related-prd doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`

状态迁移基础用法：
- `./scripts/pm/move-task.sh --task-uid task_<32hex> --to-status committed`
- `./scripts/pm/move-task.sh --task-uid task_<32hex> --to-status deferred`
- `./scripts/pm/set-stage.sh --current-stage internal_playable_alpha_late --claim-envelope internal_only --decision-date 2026-03-31 --gate-status blocked --lane-status qa=blocked --blocking-task task_<32hex> --source-ref https://github.com/eng-cc/oasis7/issues/<N>#issuecomment-<ID>`
- `./scripts/pm/promote-memory.sh --signal-id SIG-GH-<id> --role producer_system_designer --topic stage.current --promotion-reason stage_decision --tag stage --tag claim_envelope`
- `./scripts/pm/promote-memory.sh --signal-id SIG-GH-<id> --scope shared --role producer_system_designer --topic gate.claim_envelope --promotion-reason stage_decision`
- `./scripts/pm/promote-memory.sh --signal-id SIG-GH-<id> --role qa_engineer --reject-reason one_off_operation`
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
- `./scripts/pm/working-memory-autoflow.sh --task-uid task_<32hex> --severity medium --priority P2 --dry-run --json`
- `./scripts/pm/reflection-report.sh --role producer_system_designer --json`
- phase 1 的 transcript 预处理只负责排序与脱敏；结构化提炼统一交给 `codex exec --ephemeral`。
- `codex-working-memory.sh` 默认不会仅凭 task/worktree 自动解析 `.codex` session；若确实要走 registry / worktree pattern 自动解析，必须显式传 `--allow-auto-session`。
- `codex-working-memory.sh` 首次成功导入后会把 `task_uid -> session_id` 记到 `.pm/registry/codex-sessions.yaml`；后续若要继续复用该 registry 映射，也必须显式传 `--allow-auto-session`，或直接给出新的 `--session-id`。
- 同一 `task_uid + session_id` 默认按 `working_memory` header 里的 `last_extracted_ts` 做增量抽取；这只在 owner 显式选择该 session 后生效，避免把当前 live session 的隐式自读当作默认收口路径。需要重扫整段 transcript 时显式传 `--full-scan`。
- `working_memory` header 会记录 `source_session_id`、`source_thread_name`、`transcript_source`、`last_extracted_ts` 与 `captured_until_ts`，用于回放抽取来源与当前水位。
- `working-memory-autoflow.sh` 的 apply 模式已禁用，避免复活 retired local signal/task 写入路径；先用 `--dry-run` 规划，再用 `promote-signal.sh` 创建 GitHub-backed intake / candidate task。
- `working-memory-autoflow.sh --dry-run` 是严格只读的 plan 模式：它只返回“会创建/复用哪些 reflection signal 与 candidate task”，不会改 `.pm/working_memory/*.yaml`、task registry 或 task files。
- dry-run 结果里只有已存在对象才会带真实 `signal_id` / `task_uid`；若对象尚未创建，apply 之前不会预留 ID，也不会留下任何半完成状态。

role report 基础用法：
- `./scripts/pm/role-report.sh`
- `./scripts/pm/role-report.sh --role qa_engineer`
- `./scripts/pm/role-report.sh --role qa_engineer --json`
- `./scripts/pm/role-report.sh --role qa_engineer --task-uid task_<32hex>`
- 输出会同时带该角色 backlog 计数、任务列表，以及 active / needs_review / superseded memory；带 `--task-uid` 时额外输出 task collaboration 摘要，方便 owner 合流多个角色 slice 的 evidence 与缺口。

workflow report 基础用法：
- `./scripts/pm/workflow-report.sh --phase start --role qa_engineer --task-uid task_<32hex>`
- `./scripts/pm/workflow-report.sh --phase close --role liveops_community --task-uid task_<32hex>`
- `./scripts/pm/workflow-report.sh --phase review --role producer_system_designer --json`
- `./scripts/prepare-task-pr.sh --json`
- 输出会记录或读取 GitHub-backed task evidence；`start/close` 带 `--task-uid` 时会把 `last_started_at` / `last_closed_at` 写入 mapping，并在 issue comment 留证据。

阶段汇总基础用法：
- `./scripts/pm/stage-lint.sh`
- `./scripts/pm/stage-report.sh`
- `./scripts/pm/stage-report.sh --json`

required-tier 验证入口：
- `./scripts/pm/required-tier-smoke.sh`
- `./scripts/pm/required-tier-smoke.sh --json`
- `./scripts/pm/new-task-worktree-bootstrap-smoke.sh`
- `./scripts/pm/new-task-worktree-bootstrap-smoke.sh --json`
- `./scripts/pm/workflow-behavior-eval.sh`
- `./scripts/pm/workflow-behavior-eval.sh --json`
- `./scripts/pm/task-compaction-smoke.sh`
- `./scripts/pm/task-compaction-smoke.sh --json`

full-tier 验证入口：
- `./scripts/pm/memory-regression-smoke.sh`
- `./scripts/pm/memory-regression-smoke.sh --json`
- `./scripts/pm/codex-working-memory-smoke.sh`
- `./scripts/pm/codex-working-memory-smoke.sh --json`
