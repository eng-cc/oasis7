# oasis7：自我进化文件化项目管理（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-074 (PRD-ENGINEERING-SE-001/004/006) [test_tier_required]: 建立 `self-evolution` 专题 `prd/design/project`，并同步回写 engineering 根入口、主项目、索引与 devlog。
- [x] TASK-ENGINEERING-075 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 建立 `.pm/` 目录骨架、role registry、task registry 与模板文件，明确首批 7 角色及扩容规则。
- [x] TASK-ENGINEERING-076 (PRD-ENGINEERING-SE-002/003/004) [test_tier_required]: 落地 signal inbox 与 `promote-signal` 基础链路，优先覆盖 `qa_engineer` 与 `liveops_community`。
- [x] TASK-ENGINEERING-077 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required]: 落地 role memory / backlog 文件格式、`superseded` 生命周期与 lint 规则。
- [x] TASK-ENGINEERING-078 (PRD-ENGINEERING-SE-001/004) [test_tier_required]: 落地 `stage/gate` 文件与 `stage-report` 汇总入口，为制作人阶段评审提供 canonical 输入。
- [x] TASK-ENGINEERING-079 (PRD-ENGINEERING-SE-002/003/006) [test_tier_required] + [test_tier_full]: 建立 `devlog -> signal -> memory/task -> stage report` required-tier 验证链路，并评估角色扩容与多 worktree 并发回归。
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-SE-005/006) [test_tier_required]: 建立长期 memory 自建专题 `prd/design/project`，将 role memory schema、promotion 与 superseded 规则从总专题中单独冻结。
- [x] TASK-ENGINEERING-084 (PRD-ENGINEERING-SE-004/005/006) [test_tier_required] + [test_tier_full]: 落地 `role-report` 入口，汇总 role backlog + role memory，并补按角色查询与扩容回归。
- [ ] TASK-ENGINEERING-085 (PRD-ENGINEERING-SE-001/002/003/004/007) [test_tier_required] + [test_tier_full]: 把 `.pm` 接入现有开发工作流，落地 `workflow-report` 统一入口，并同步接入 `AGENTS.md`、角色职责卡、`new-task-worktree` 提示与 smoke。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `doc/devlog/YYYY-MM-DD.md`
- `testing-manual.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- 仓库根目录 `.pm/`
- `scripts/pm/*.sh`

## 状态
- 更新日期: 2026-03-31
- 当前阶段: active
- 当前任务: `TASK-ENGINEERING-085` 进行中，目标是把 `.pm` 从“基础入口已齐”提升为“现有工作流默认使用”；`TASK-ENGINEERING-074~084` 已完成。
- 阻塞项:
  - 暂无实现阻塞；当前等待 owner 复核 `workflow-report`、`AGENTS.md` 接入口径与 smoke 结果后再提交。
- 最新完成:
  - `TASK-ENGINEERING-084`：已落地 `role-report.sh` 与 `pm_store.py role-report`，可按角色汇总 backlog 状态、blocked tasks 与 active/needs_review/superseded memory；required/full smoke 均已覆盖真实 backlog + stale memory + 扩容场景。
  - `TASK-ENGINEERING-083`：已落地 `memory-report.sh`、7 天 stale review 口径、`PM_ROOT_DIR` 兼容的 lint/scaffold，以及 `memory-regression-smoke.sh` full-tier 回归；长期 memory 现具备 active / needs_review / superseded 统一查询入口。
  - `TASK-ENGINEERING-082`：已落地 `promote-memory.sh` 与 signal `memory_promotion_state` 决策回写，required-tier smoke 现可覆盖 accepted/rejected memory promotion case。
  - `TASK-ENGINEERING-081`：已为 `producer_system_designer`、`qa_engineer`、`liveops_community` 与 `shared` 落地首批 active/superseded 样例，覆盖 stage current、QA failure signature、community messaging boundary 与 shared claim envelope 场景。
  - `TASK-ENGINEERING-079`：已新增 `required-tier-smoke.sh`，可在临时 PM 根目录内跑通 `devlog -> signal -> blocked task -> stage report` 验证链；同时记录角色扩容仍受 `.agents/roles/*.md` 白名单约束，而 `.pm/registry/tasks.yaml` / `signals.jsonl` 仍是多 worktree 并发下的主要合流点。
  - `TASK-ENGINEERING-078`：已落地 `stage-report.sh`，可汇总 `.pm/stage/{current,gate}.yaml`、blocked tasks、按角色 backlog 计数，以及 producer/shared active memory，为阶段评审提供统一输入。
  - `TASK-ENGINEERING-077`：已落地 `move-task.sh`、`supersede-memory.sh`、`memory-lint.sh` 与内部 `pm_store.py`，补齐 backlog 全状态迁移、role/shared memory 的 superseded 生命周期，以及 task/backlog/memory 一致性 lint。
  - `TASK-ENGINEERING-076`：已落地 `.pm/inbox/signals.jsonl`、`scripts/pm/promote-signal.sh` 与 `scripts/pm/new-task.sh`，可将 `qa_engineer` / `liveops_community` 高价值信号写入 inbox，并直接生成 candidate task、task registry 与 owner backlog 条目。
  - `TASK-ENGINEERING-075`：已建立 `.pm/` 目录骨架、role registry、task registry、stage/shared 容器、首批 7 角色 memory/backlog 文件，以及 `scripts/pm/{scaffold,lint,new-task,promote-signal,stage-report,role-report}.sh` Phase 1 入口。
  - `TASK-ENGINEERING-074`：已建立 `self-evolution` 专题三件套，并将文件化项目管理目标态正式挂入 engineering 根入口、主项目、索引与 devlog。
  - `TASK-ENGINEERING-080`：已将长期 memory 从总专题里拆成独立子专题，单独冻结 active/superseded schema、promotion 规则和 memory 脚本契约。
- 下一步:
  - 完成 `TASK-ENGINEERING-085` 的 review、提交与 landing，把 `workflow-report` 正式并入本地 `main`；
  - 之后若要推进 overdue SLA、跨角色 review board 或 shared dashboard，再单列新治理任务。
