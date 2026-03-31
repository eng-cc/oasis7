# oasis7：记忆启发式自我进化补强（2026-03-31）项目管理

- 对应设计文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.design.md`
- 对应需求文档: `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-086 (PRD-ENGINEERING-MIR-001/004/005) [test_tier_required]: 建立记忆启发式补强专题 `prd/design/project`，并同步回写 engineering 根入口、主项目、索引与 devlog。
- [ ] TASK-ENGINEERING-087 (PRD-ENGINEERING-MIR-001/002/005) [test_tier_required]: 扩展 role/shared memory schema，增加 `memory_kind`、`review_due_at`、`recall_priority` 与兼容 lint/report。
- [ ] TASK-ENGINEERING-088 (PRD-ENGINEERING-MIR-002/005) [test_tier_required] + [test_tier_full]: 建立 recall profile registry，并让 `workflow-report` / `memory-report` 支持 budgeted recall 视图与截断说明。
- [ ] TASK-ENGINEERING-089 (PRD-ENGINEERING-MIR-002/003/004) [test_tier_required] + [test_tier_full]: 为 `source_type=reflection` 建立 canonical signal 契约、去重规则和 owner review 入口。
- [ ] TASK-ENGINEERING-090 (PRD-ENGINEERING-MIR-003/004/005) [test_tier_required] + [test_tier_full]: 建立 recall/reflection 质量评估与回归 smoke，覆盖 stale belief、重复 reflection 与新角色扩容场景。

## 依赖
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.project.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.prd.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.design.md`
- `doc/engineering/self-evolution/role-long-term-memory-2026-03-30.project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/project.md`
- `.pm/roles/*/memory/{active,superseded}.yaml`
- `.pm/inbox/signals.jsonl`
- `scripts/pm/promote-signal.sh`
- `scripts/pm/promote-memory.sh`
- `scripts/pm/memory-report.sh`
- `scripts/pm/workflow-report.sh`
- `https://github.com/memoryOSScom/memoryOSS`
- `https://arxiv.org/abs/2512.12818`

## 状态
- 更新日期: 2026-03-31
- 当前阶段: planned
- 当前任务: `TASK-ENGINEERING-086` 已完成；后续先推进 schema 补强与 recall profile，再进入 reflection 契约与质量回归。
- 阻塞项:
  - `TASK-ENGINEERING-087~090` 依赖 owner 先确认增量字段命名与 recall profile 的落位方式，再开始实现。
- 最新完成:
  - `TASK-ENGINEERING-086`：已建立“记忆启发式自我进化补强”专题三件套，冻结对 `memoryOSS` / 《Hindsight》 的 adopted / rejected / deferred 边界，并同步回写 engineering 根入口、主项目、索引与 devlog。
- 下一步:
  - 先完成 `TASK-ENGINEERING-087`，为现有 role/shared memory 加入 `memory_kind`、`review_due_at`、`recall_priority` 三类增量字段与 lint/report 兼容；
  - 然后推进 `TASK-ENGINEERING-088`，让 `workflow-report` 进入预算化 recall 视图，而不是继续输出无上限 memory 汇总；
  - 在 recall budget 稳定后，再推进 `TASK-ENGINEERING-089/090` 的 reflection 契约与质量评估。
