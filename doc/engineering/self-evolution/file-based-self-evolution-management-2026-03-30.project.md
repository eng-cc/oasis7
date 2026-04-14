# oasis7：自我进化文件化项目管理（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`

审计轮次: 7

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-074 (PRD-ENGINEERING-SE-001/004/006) [test_tier_required]: 建立 `self-evolution` 专题 `prd/design/project`，并同步回写 engineering 根入口、主项目、索引与 task execution log 规则。
- [x] TASK-ENGINEERING-075 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 建立 `.pm/` 目录骨架、role registry、task registry 与模板文件，明确首批 7 角色及扩容规则。
- [x] TASK-ENGINEERING-076 (PRD-ENGINEERING-SE-002/003/004) [test_tier_required]: 落地 signal inbox 与 `promote-signal` 基础链路，优先覆盖 `qa_engineer` 与 `liveops_community`。
- [x] TASK-ENGINEERING-077 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 落地 role memory / backlog 文件格式、`superseded` 生命周期与 lint 规则。
- [x] TASK-ENGINEERING-078 (PRD-ENGINEERING-SE-001/004) [test_tier_required]: 落地 `stage/gate` 文件与 `stage-report` 汇总入口，为制作人阶段评审提供 canonical 输入。
- [x] TASK-ENGINEERING-079 (PRD-ENGINEERING-SE-002/003/006) [test_tier_required] + [test_tier_full]: 建立 `task execution log / evidence -> signal -> memory/task -> stage report` required-tier 验证链路，并评估角色扩容与多 worktree 并发回归。
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-SE-005/006) [test_tier_required]: 建立长期 memory 自建专题 `prd/design/project`，将 role memory schema、promotion 与 superseded 规则从总专题中单独冻结。
- [x] TASK-ENGINEERING-084 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required] + [test_tier_full]: 落地 `role-report` 入口，汇总 role backlog + role memory，并补按角色查询与扩容回归。
- [x] TASK-ENGINEERING-085 (PRD-ENGINEERING-SE-001/002/003/004/007) [test_tier_required] + [test_tier_full]: 把 `.pm` 接入现有开发工作流，补齐 `set-stage` / `stage-lint` 当前态治理、`workflow-report --task-uid` 的 start/close 留痕，并同步接入 `AGENTS.md`、角色职责卡、`new-task-worktree` 提示、commit 前快照式 `codex exec review --uncommitted` 规则与 smoke。
- [x] TASK-ENGINEERING-092 (PRD-ENGINEERING-SE-007) [test_tier_required]: 收紧 commit 前 review 的默认流程口径，并同步回写 `AGENTS.md`、engineering 主 PRD、self-evolution 专题与 task execution log。
- [x] TASK-ENGINEERING-093 (PRD-ENGINEERING-SE-007) [test_tier_required]: 补齐 commit 前 review 的正式追踪互链，并同步回写 `AGENTS.md`、engineering 主 PRD、self-evolution 专题与 task execution log。
- [x] TASK-ENGINEERING-094 (PRD-ENGINEERING-SE-007) [test_tier_required]: 将根 `AGENTS.md`、engineering 主 PRD 与本专题中的 commit 前 review 规则统一收口为默认流程口径。
- [x] TASK-ENGINEERING-096 (PRD-ENGINEERING-SE-004/007) [test_tier_required] + [test_tier_full]: 将执行日志 canonical 路径切换到 `.pm/tasks/task_<32hex>.execution.md`，并同步回写 `AGENTS.md`、`.pm/README`、`workflow-report`、task lint 与 smoke。
- [x] TASK-ENGINEERING-097 (PRD-ENGINEERING-SE-007) [test_tier_required]: 收紧 commit 前 review 话术，补齐快照式 `codex exec review --uncommitted` 与旧口径之间的边界说明，并把运行环境阻断边界同步回写到正式文档与 `workflow-report` checklist。
- [x] TASK-ENGINEERING-098 (PRD-ENGINEERING-SE-007) [test_tier_required]: 将 `workflow-report --phase close --task-uid` 的 working_memory 提示改为按当前 task 计数，并在零条目时提示 `codex-working-memory` bootstrap 入口，同时补齐 smoke 断言。
- [x] TASK-ENGINEERING-099 (PRD-ENGINEERING-SE-004/006/007/008) [test_tier_required] + [test_tier_full]: 将 `.pm` task identity 重构为 `task_uid` 单一真值，移除 `TASK-PM-xxxx`、`next_sequence` 与强同步 task registry/backlog 主键依赖，并补齐 task/state/source_ref 迁移脚本与回归验证。
- [x] TASK-ENGINEERING-100 (PRD-ENGINEERING-SE-001/004/007) [test_tier_required]: 清理 `doc/devlog/*.md` 作为 `.pm` 运行态 `source_ref(s)` / `updated_from` 的残留口径，补齐 stage/signal/task/memory 门禁与正式文档回写。
- [x] TASK-ENGINEERING-102 (PRD-ENGINEERING-SE-007) [test_tier_required]: 清理正式流程中残留的旧 review 文案，并将 commit 前 review 固定为通过 `./scripts/pm/codex-review-snapshot.sh` 在临时隔离快照中执行 `codex exec review --uncommitted`，同时同步回写 self-evolution / engineering 正式追踪、`.pm` 运行态口径与 `workflow-report` smoke。
- [x] TASK-ENGINEERING-113 (PRD-ENGINEERING-SE-007) [test_tier_required]: 将默认最终合流从本地 `landing` 切到 GitHub PR，新增 `prepare-task-pr.sh` 标准入口，并同步回写 `AGENTS.md`、`.pm/README`、self-evolution / engineering 正式追踪、scripts 模块文档与旧 landing 兼容边界。
- [x] drop-local-review-script (PRD-ENGINEERING-SE-007) [test_tier_required]: 将默认评审边界完全切到 GitHub PR review，移除 `codex-review-snapshot.sh` 与相关 `workflow-report` / smoke / README / self-evolution 正式口径残留。 Trace: .pm/tasks/task_72972433a36f46d0b8e95c04e1303a42.yaml
- [x] TASK-ENGINEERING-PMVIEW-001 (PRD-ENGINEERING-SE-004/007/008) [test_tier_required] + [test_tier_full]: 将 `.pm` registry/backlog 降级为 git-ignored 本地生成视图，新增 `sync-views` 入口并让 lint/report/read-path 在缺失时自动重建；同时收口根 engineering 项目页的热点写法与 topic-scoped task id 口径。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `.pm/tasks/task_<32hex>.execution.md`
- `testing-manual.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.project.md`
- 仓库根目录 `.pm/`
- `scripts/pm/*.sh`

## 状态
- 更新日期: 2026-04-10
- 当前阶段: active
- 当前任务: 回到 `TASK-ENGINEERING-089/090` 的 recall / reflection 契约与质量回归；`TASK-ENGINEERING-113` 已把默认最终合流切到 GitHub PR，`TASK-ENGINEERING-099/100` 分别完成 `task_uid` 单一真值与 `doc/devlog/*.md` 退出 `.pm` 运行态真值的收口。
- 阻塞项:
  - 无；该专题 workflow integration tranche 已闭环。
- 最新完成:
  - `drop-local-review-script`：已将默认评审边界完全切到 GitHub PR review，并把 `workflow-report` close checklist、required-tier smoke、`.pm/README`、`AGENTS.md` 与 engineering / self-evolution 正式追踪统一改成“commit -> prepare-task-pr -> GitHub PR review/approval”。
  - `TASK-ENGINEERING-113`：已将默认最终合流从本地 `landing` 切到 GitHub PR，新增 `prepare-task-pr.sh` 标准入口，并把 `AGENTS.md`、`.pm/README`、self-evolution / engineering 正式追踪与 scripts 模块文档统一改成“PR 是默认最终保护边界，本地 landing 仅保留给 compatibility / fallback”。
  - `TASK-ENGINEERING-PMVIEW-001`：已新增 `sync-views` 入口，并把 `.pm/registry/tasks.yaml` 与 `.pm/roles/*/backlog/*.yaml` 降级为 git-ignored 本地生成视图；PM lint/report/read-path 在缺失时可自动重建，根 engineering project 也已停止手工维护“最新完成”长列表，改为以 topic project 与 `.pm/tasks/*.yaml` 追溯近期收口。
  - `TASK-ENGINEERING-102`：已清理正式流程中残留的旧 review 文案，并将 commit 前 review 固定为通过 `./scripts/pm/codex-review-snapshot.sh` 在临时隔离快照中执行 `codex exec review --uncommitted`，同时同步回写 self-evolution / engineering 正式追踪、`.pm` 运行态口径与 `workflow-report` smoke。
  - `TASK-ENGINEERING-097`：已收紧 commit 前 review 话术，补齐快照式 `codex exec review --uncommitted` 与旧口径之间的边界说明，并补齐运行环境阻断边界。
  - `TASK-ENGINEERING-098`：已将 `workflow-report --phase close --task-uid` 的 working_memory 提示改为按当前 task 计数，并在零条目时提示 `codex-working-memory` bootstrap 入口，同时补齐 smoke 断言。
  - `TASK-ENGINEERING-099`：已将 `.pm` task identity 收敛为 `task_uid` 单一真值，移除顺序 `TASK-PM-xxxx`、`next_sequence` 与强同步 task registry/backlog 主键依赖，并完成 lint/smoke 与正式文档迁移收口。
  - `TASK-ENGINEERING-100`：已明确 `doc/devlog/*.md` 仅作历史归档，`.pm` 的 stage/gate、signal、task 与 memory `source_ref(s)` / `updated_from` 统一切到 task execution log、正式文档或显式 evidence，并补齐 lint / promote-signal / set-stage 阻断。
  - `TASK-ENGINEERING-096`：已将执行日志 canonical 路径切到 `.pm/tasks/task_<32hex>.execution.md`，并把 AGENTS / `.pm/README` / `workflow-report` / task lint / smoke 一并收口到按任务归档模型。
  - `TASK-ENGINEERING-094`：已将根 `AGENTS.md`、engineering 主 PRD 与本专题中的 commit 前 review 规则统一收口为默认流程口径。
  - `TASK-ENGINEERING-085`：已补齐 `set-stage` / `stage-lint` 当前态治理、`workflow-report --task-uid` 的 start/close 留痕，并把 AGENTS / 角色职责卡 / `new-task-worktree` / `.pm/README` / required-tier smoke 全部切到显式 task 绑定口径。
  - `TASK-ENGINEERING-086`：已建立“记忆启发式自我进化补强”专题三件套，冻结 `memoryOSS` / 《Hindsight》 的 adopted / rejected / deferred 边界，并把后续 recall/reflection 增量任务挂回 `self-evolution` 总专题依赖链。
  - `TASK-ENGINEERING-084`：已落地 `role-report.sh` 与 `pm_store.py role-report`，可按角色汇总 backlog 状态、blocked tasks 与 active/needs_review/superseded memory；required/full smoke 均已覆盖真实 backlog + stale memory + 扩容场景。
  - `TASK-ENGINEERING-083`：已落地 `memory-report.sh`、7 天 stale review 口径、`PM_ROOT_DIR` 兼容的 lint/scaffold，以及 `memory-regression-smoke.sh` full-tier 回归；长期 memory 现具备 active / needs_review / superseded 统一查询入口。
  - `TASK-ENGINEERING-082`：已落地 `promote-memory.sh` 与 signal `memory_promotion_state` 决策回写，required-tier smoke 现可覆盖 accepted/rejected memory promotion case。
  - `TASK-ENGINEERING-081`：已为 `producer_system_designer`、`qa_engineer`、`liveops_community` 与 `shared` 落地首批 active/superseded 样例，覆盖 stage current、QA failure signature、community messaging boundary 与 shared claim envelope 场景。
  - `TASK-ENGINEERING-079`：已新增 `required-tier-smoke.sh`，可在临时 PM 根目录内跑通 `task execution log / evidence -> signal -> blocked task -> stage report` 验证链；同时记录角色扩容仍受 `.agents/roles/*.md` 白名单约束，而 `.pm/registry/tasks.yaml` / `signals.jsonl` 仍是多 worktree 并发下的主要合流点。
  - `TASK-ENGINEERING-078`：已落地 `stage-report.sh`，可汇总 `.pm/stage/{current,gate}.yaml`、blocked tasks、按角色 backlog 计数，以及 producer/shared active memory，为阶段评审提供统一输入。
  - `TASK-ENGINEERING-077`：已落地 `move-task.sh`、`supersede-memory.sh`、`memory-lint.sh` 与内部 `pm_store.py`，补齐 backlog 全状态迁移、role/shared memory 的 superseded 生命周期，以及 task/backlog/memory 一致性 lint。
  - `TASK-ENGINEERING-076`：已落地 `.pm/inbox/signals.jsonl`、`scripts/pm/promote-signal.sh` 与 `scripts/pm/new-task.sh`，可将 `qa_engineer` / `liveops_community` 高价值信号写入 inbox，并直接生成 candidate task、task registry 与 owner backlog 条目。
  - `TASK-ENGINEERING-075`：已建立 `.pm/` 目录骨架、role registry、task registry、stage/shared 容器、首批 7 角色 memory/backlog 文件，以及 `scripts/pm/{scaffold,lint,new-task,promote-signal,stage-report,role-report}.sh` Phase 1 入口。
  - `TASK-ENGINEERING-074`：已建立 `self-evolution` 专题三件套，并将文件化项目管理目标态正式挂入 engineering 根入口、主项目、索引与 task execution log 规则。
  - `TASK-ENGINEERING-080`：已将长期 memory 从总专题里拆成独立子专题，单独冻结 active/superseded schema、promotion 规则和 memory 脚本契约。
- 下一步:
- 转回 `TASK-ENGINEERING-087~090` 的 recall / reflection 增量补强；
  - 之后若要继续推进 overdue SLA、跨角色 review board、shared dashboard 或新的 `.pm` 低热点治理，再单列后续治理任务。
