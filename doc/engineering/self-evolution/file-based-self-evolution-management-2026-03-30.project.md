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
- 更新日期: 2026-03-30
- 当前阶段: active
- 当前任务: `TASK-ENGINEERING-081`
- 阻塞项:
  - `role-report.sh` 仍仅完成入口占位，尚未具备跨角色记忆与 backlog 汇总能力。
  - `promote-memory.sh` 与 `memory-report.sh` 尚未落地，长期 memory 仍缺少从 signal 提升和 stale 汇总入口。
- 最新完成:
  - `TASK-ENGINEERING-079`：已新增 `required-tier-smoke.sh`，可在临时 PM 根目录内跑通 `devlog -> signal -> blocked task -> stage report` 验证链；同时记录角色扩容仍受 `.agents/roles/*.md` 白名单约束，而 `.pm/registry/tasks.yaml` / `signals.jsonl` 仍是多 worktree 并发下的主要合流点。
  - `TASK-ENGINEERING-078`：已落地 `stage-report.sh`，可汇总 `.pm/stage/{current,gate}.yaml`、blocked tasks、按角色 backlog 计数，以及 producer/shared active memory，为阶段评审提供统一输入。
  - `TASK-ENGINEERING-077`：已落地 `move-task.sh`、`supersede-memory.sh`、`memory-lint.sh` 与内部 `pm_store.py`，补齐 backlog 全状态迁移、role/shared memory 的 superseded 生命周期，以及 task/backlog/memory 一致性 lint。
  - `TASK-ENGINEERING-076`：已落地 `.pm/inbox/signals.jsonl`、`scripts/pm/promote-signal.sh` 与 `scripts/pm/new-task.sh`，可将 `qa_engineer` / `liveops_community` 高价值信号写入 inbox，并直接生成 candidate task、task registry 与 owner backlog 条目。
  - `TASK-ENGINEERING-075`：已建立 `.pm/` 目录骨架、role registry、task registry、stage/shared 容器、首批 7 角色 memory/backlog 文件，以及 `scripts/pm/{scaffold,lint,new-task,promote-signal,stage-report,role-report}.sh` Phase 1 入口。
  - `TASK-ENGINEERING-074`：已建立 `self-evolution` 专题三件套，并将文件化项目管理目标态正式挂入 engineering 根入口、主项目、索引与 devlog。
  - `TASK-ENGINEERING-080`：已将长期 memory 从总专题里拆成独立子专题，单独冻结 active/superseded schema、promotion 规则和 memory 脚本契约。
- 下一步:
  - 先执行 `TASK-ENGINEERING-081`，补齐首批 role/shared memory 样例；
  - 再执行 `TASK-ENGINEERING-082/083`，补 `promote-memory`、`memory-report` 与长期 memory 扩容验证；
  - 后续再评估 `role-report` 是否需要单列治理任务。
