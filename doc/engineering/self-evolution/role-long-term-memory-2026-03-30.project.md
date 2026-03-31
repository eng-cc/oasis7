# oasis7：角色长期记忆自建（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-MEM-001/004/005) [test_tier_required]: 建立长期 memory 专题 `prd/design/project`，并同步回写 `self-evolution` 主专题、engineering 索引与 devlog。
- [x] TASK-ENGINEERING-081 (PRD-ENGINEERING-MEM-001/002/003/005) [test_tier_required]: 建立首批 role/shared memory 文件模板与样例，优先覆盖 `producer_system_designer`、`qa_engineer`、`liveops_community`。
- [x] TASK-ENGINEERING-082 (PRD-ENGINEERING-MEM-001/004) [test_tier_required]: 落地 `promote-memory` / `supersede-memory` 脚本契约与 promotion_reason 白名单。
- [x] TASK-ENGINEERING-083 (PRD-ENGINEERING-MEM-002/003/004/005) [test_tier_required] + [test_tier_full]: 落地 `memory-lint` / `memory-report`，并验证 active 冲突、superseded 链、stale review 与新角色扩容回归。

## 依赖
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `.pm/inbox/signals.jsonl`
- `.pm/roles/*/memory/*.yaml`

## 状态
- 更新日期: 2026-03-31
- 当前阶段: active
- 当前任务: `TASK-ENGINEERING-083` 已完成；长期 memory 子专题 `TASK-ENGINEERING-080~083` 已全部闭环。
- 阻塞项:
  - 暂无新增阻塞；若要继续推进 `role-report.sh`，需先冻结新的治理任务。
- 最新完成:
  - `TASK-ENGINEERING-083`：已落地 `scripts/pm/memory-report.sh` 与 `pm_store.py memory-report`，默认按 7 天 stale 阈值输出 active / needs_review / superseded 视图；同时补齐 `PM_ROOT_DIR` 兼容的 `lint.sh` / `scaffold.sh`，并新增 `memory-regression-smoke.sh` 覆盖 stale review、active topic 冲突、superseded 缺链和新角色扩容回归。
  - `TASK-ENGINEERING-082`：已落地 `scripts/pm/promote-memory.sh` 与 `pm_store.py promote-memory` 子命令，冻结 `promotion_reason` / `reject_reason` 白名单、shared memory 写权限，以及 signal 的 `memory_promotion_state` 决策回写；`required-tier-smoke.sh` 现覆盖 accepted/rejected promotion case。
  - `TASK-ENGINEERING-081`：已为 `producer_system_designer`、`qa_engineer`、`liveops_community` 与 `shared` 落地首批 active/superseded 样例，覆盖 stage current、QA failure signature、community messaging boundary 与 shared claim envelope 场景。
  - `TASK-ENGINEERING-077`：已在 `self-evolution` 主专题先行落地 `supersede-memory.sh`、`memory-lint.sh` 与 role/shared memory 基础 lint 规则，后续长期 memory 子专题可在此基础上继续补样例、promotion 和 report。
  - `TASK-ENGINEERING-076`：已落地 `.pm/inbox/signals.jsonl` 与 `signal -> candidate task` 基础链路，后续 memory promotion 可直接复用 signal inbox 作为输入层。
  - `TASK-ENGINEERING-075`：已建立 `.pm` 基础骨架，并为 7 个标准角色生成 `memory/{active,superseded}.yaml` 容器，长期 memory 后续任务可直接在仓库内演进。
  - `TASK-ENGINEERING-080`：已建立长期 memory 专题三件套，并把长期记忆自建方案正式挂入 `self-evolution` 总专题、engineering 索引与 devlog。
- 下一步:
  - 后续若要推进 `role-report.sh`，先新增正式治理任务，再决定是否把 role/backlog 汇总单独挂题；
  - 现阶段长期 memory 子专题已满足 active/superseded/report/lint/扩容验证的冻结范围。
