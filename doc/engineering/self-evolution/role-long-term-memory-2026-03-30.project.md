# oasis7：角色长期记忆自建（2026-03-30）项目管理

- 对应设计文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- 对应需求文档: `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-080 (PRD-ENGINEERING-MEM-001/004/005) [test_tier_required]: 建立长期 memory 专题 `prd/design/project`，并同步回写 `self-evolution` 主专题、engineering 索引与 devlog。
- [ ] TASK-ENGINEERING-081 (PRD-ENGINEERING-MEM-001/002/003/005) [test_tier_required]: 建立首批 role/shared memory 文件模板与样例，优先覆盖 `producer_system_designer`、`qa_engineer`、`liveops_community`。
- [ ] TASK-ENGINEERING-082 (PRD-ENGINEERING-MEM-001/004) [test_tier_required]: 落地 `promote-memory` / `supersede-memory` 脚本契约与 promotion_reason 白名单。
- [ ] TASK-ENGINEERING-083 (PRD-ENGINEERING-MEM-002/003/004/005) [test_tier_required] + [test_tier_full]: 落地 `memory-lint` / `memory-report`，并验证 active 冲突、superseded 链、stale review 与新角色扩容回归。

## 依赖
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `AGENTS.md`
- `.agents/roles/*.md`
- `.pm/inbox/signals.jsonl`（待建）
- `.pm/roles/*/memory/*.yaml`

## 状态
- 更新日期: 2026-03-30
- 当前阶段: planned
- 当前任务: `TASK-ENGINEERING-081`
- 阻塞项:
  - `signal inbox` 正式文件与 promotion 链路仍未落地，memory 还无法从 signal 做真实提升。
  - memory 样例记录、promotion/supersede 脚本与 lint/report 仍未建档。
- 最新完成:
  - `TASK-ENGINEERING-075`：已建立 `.pm` 基础骨架，并为 7 个标准角色生成 `memory/{active,superseded}.yaml` 容器，长期 memory 后续任务可直接在仓库内演进。
  - `TASK-ENGINEERING-080`：已建立长期 memory 专题三件套，并把长期记忆自建方案正式挂入 `self-evolution` 总专题、engineering 索引与 devlog。
- 下一步:
  - 先跟随 `TASK-ENGINEERING-075` 建立 `.pm` 基础骨架；
  - 再执行 `TASK-ENGINEERING-081/082`，收口 memory 模板与 promotion/supersede 规则；
  - 最后执行 `TASK-ENGINEERING-083`，跑通 lint/report 和扩容验证。
