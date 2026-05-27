# TASK-ENGINEERING-009 活跃老格式文档分批迁移收口记录（2026-03-11）

审计轮次: 4

## 目标
- 为 `TASK-ENGINEERING-009 (PRD-ENGINEERING-004)` 形成统一完成态结论，确认“按模块分批推进活跃老格式文档逐篇人工迁移并持续回写 PRD-ID / project / devlog”的目标已由 `TASK-ENGINEERING-010 ~ TASK-ENGINEERING-015` 全量兑现。
- 将 engineering 主项目中最后一个迁移 umbrella 任务正式挂账为完成，避免后续继续把已收口迁移专项误判为未完成主线工作。

## 任务定义对照
- 任务要求：按模块分批推进活跃老格式文档迁移，并对迁移过程持续回写 `PRD-ID / project / devlog`。
- 已满足要件：
  - 分批方案已冻结：`TASK-ENGINEERING-010` 产出协作主文档与 2026-03-03 冻结快照。
  - 模块批次已执行：`TASK-ENGINEERING-011/012/013/013B/013C/013D/014/014-D1/014-D2` 覆盖 world-simulator、p2p、world-runtime、headless-runtime、site、readme、scripts、game、engineering 与根入口 redirect。
  - 收口复核已完成：`TASK-ENGINEERING-015` 已确认冻结快照现存缺口为 0，命名一致性、引用可达与模块追踪同步均通过。
  - 文档追踪已回写：迁移协作子项目、engineering 主项目与 `doc/devlog/README.md` / `doc/devlog/README.md` 均有明确记录。

## 批次覆盖摘要
| 批次 | 范围 | 状态 | 关键结论 |
| --- | --- | --- | --- |
| `TASK-ENGINEERING-010` | 协作方案与冻结快照 | `completed` | 冻结 owner 边界、迁移原则与 303 条待迁移快照 |
| `TASK-ENGINEERING-011` | `doc/world-simulator/**` | `completed` | 146 篇迁移完成 |
| `TASK-ENGINEERING-012` | `doc/p2p/**` | `completed` | 70 篇迁移完成 |
| `TASK-ENGINEERING-013/013B/013C/013D` | `doc/world-runtime/**` + `doc/headless-runtime/**` | `completed` | 30 篇迁移完成并分批收口 |
| `TASK-ENGINEERING-014/014-D1/014-D2` | `doc/site/**`、`doc/readme/**`、`doc/scripts/**`、`doc/game/**`、`doc/engineering/**` 与根入口 redirect | `completed` | 57 篇迁移与 3 份根入口 redirect 收口完成 |
| `TASK-ENGINEERING-015` | 全量迁移收口复核 | `completed` | 现存迁移条目缺口为 0，迁移专项具备完成态证据链 |

## 证据文件
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.prd.md`
- `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`
- `doc/engineering/doc-migration/legacy-doc-migration-backlog-2026-03-03.md`
- `doc/engineering/doc-migration/task-engineering-015-migration-closure-review-2026-03-11.md`
- `doc/engineering/project.md`
- `doc/devlog/README.md`
- `doc/devlog/README.md`

## 验收判定
- `PRD-ENGINEERING-004` 对 `TASK-ENGINEERING-009` 的要求已满足，`doc/engineering/project.md` 可以将该任务标记为完成，并将 engineering 主项目切为 completed。
- 后续若新增 legacy 迁移需求，应新开专题任务，而不是重新打开 `TASK-ENGINEERING-009`。

## 对 QA 的交接点
- 本次只要求 `qa_engineer` 复核“umbrella 任务是否具备完成态证据链”，不要求重新审读所有迁移文档正文。
- 若 QA 需要抽样，优先抽查：
  - `doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`
  - `doc/engineering/doc-migration/task-engineering-015-migration-closure-review-2026-03-11.md`
  - `doc/engineering/project.md`

## 验证命令
- `python - <<'PY' ... 冻结快照 existing gap 校验 ... PY`
- `grep -nF -- '- [x] TASK-ENGINEERING-009' doc/engineering/project.md`
- `rg -n "当前状态: completed|下一任务: 无" doc/engineering/project.md doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`
