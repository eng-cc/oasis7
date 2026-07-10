# oasis7：角色长期记忆自建（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`

审计轮次: 7

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-MEM-001/004/005) [test_tier_required]: 建立长期 memory 专题 `prd/design/project`，并同步回写 `self-evolution` 主专题、engineering 索引与 task execution log 规则。
- [x] TASK-ENGINEERING-081 (PRD-ENGINEERING-MEM-001/002/003/005) [test_tier_required]: 建立首批 role/shared memory 文件模板与样例，优先覆盖 `producer_system_designer`、`qa_engineer`、`liveops_community`。
- [x] TASK-ENGINEERING-082 (PRD-ENGINEERING-MEM-001/004) [test_tier_required]: 落地 `promote-memory` / `supersede-memory` 脚本契约与 promotion_reason 白名单。
- [x] TASK-ENGINEERING-083 (PRD-ENGINEERING-MEM-002/003/004/005) [test_tier_required] + [test_tier_full]: 落地 `memory-lint` / `memory-report`，并验证 active 冲突、superseded 链、stale review 与新角色扩容回归。
- [x] TASK-ENGINEERING-092 (PRD-ENGINEERING-MEM-001/004/005) [test_tier_required]: 起草 7 个标准角色 `topic` allowlist、扩展 `promotion_reason` 白名单与 close-phase 记忆抽取 checklist，并同步接入职责卡、`workflow-report` 与 `.pm/templates/role-memory-policy.yaml` 草案。

## 依赖
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- `doc/engineering/workflow/source-of-truth.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `.pm/templates/role-memory-policy.yaml`
- `scripts/pm/pm_store.py`
- GitHub-backed reflection intake / `.pm/github-project-sync/intake-signals.json`
- `.pm/roles/*/memory/*.yaml`

## 状态
- 更新日期: 2026-04-03
- 当前阶段: active
- 当前任务: `TASK-ENGINEERING-092` 已在当前 worktree 起草完成；长期 memory 子专题现已补齐角色级 memory policy draft、扩展 promotion 白名单与 close-phase 抽取口径。
- 当前 evidence sink 说明: 上方 `TASK-*` 任务拆解行是受 doc-governance 保护的历史顺序号表面，行内旧 `task execution log` 口径仅作当期完成态追溯；新任务证据与可复用结论沉淀必须走 GitHub task issue evidence comments、task-scoped `working_memory` 或 GitHub-backed reflection intake。
- 阻塞项:
  - 暂无新增阻塞；若要继续推进 `role-report.sh`，需先冻结新的治理任务。
- 下一步:
  - 若要把 `topic` allowlist 从草案提升为 lint 强约束，再单列任务把 `.pm/templates/role-memory-policy.yaml` 接入 `promote-memory` / `memory-lint`；
  - 若要把 close-phase 三问进一步结构化到 `working_memory -> reflection signal`，优先与 `TASK-ENGINEERING-089/090` 合并推进；
  - 现阶段长期 memory 子专题已补齐 role policy / promotion / checklist 的首版草案，但尚未将 role-specific topic policy 作为硬门禁执行。
