# Role Handoff Brief

审计轮次: 4

## Meta
- Handoff ID: `HANDOFF-ENGINEERING-009-2026-03-11-ACTIVE-LEGACY-CLOSURE`
- Date: `2026-03-11`
- From Role: `producer_system_designer`
- To Role: `qa_engineer`
- Related PRD-ID: `PRD-ENGINEERING-004`
- Related Task ID: `TASK-ENGINEERING-009`
- Priority: `P1`

## Goal
- 复核 active legacy 文档分批迁移 umbrella 任务已具备完成态证据链，并确认 engineering 主项目可关闭 `TASK-ENGINEERING-009`。

## Why Now
- `TASK-ENGINEERING-015` 已完成全量收口复核；如果不把 `TASK-ENGINEERING-009` 作为 umbrella 任务回写完成，engineering 主项目会一直显示为未收口。
- 关闭后，engineering 模块主项目能够正式进入 completed 状态。

## Inputs
- 代码 / 文档入口：`doc/engineering/project.md`、`doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`、`doc/engineering/doc-migration/task-engineering-009-active-legacy-migration-closure-2026-03-11.md`
- 已完成内容：分批迁移执行、根入口 redirect 收口、全量迁移复核、主项目趋势与季度治理补齐
- 已知约束：本次验证任务级证据链，不新增迁移批次
- 依赖前置项：`TASK-ENGINEERING-010 ~ TASK-ENGINEERING-015`

## Expected Output
- 接收方交付物 1：确认 `TASK-ENGINEERING-009` 满足 `test_tier_required`
- 接收方交付物 2：如发现缺口，仅登记缺口，不重新打开已完成子批次
- 需要回写的文档 / 日志：`doc/devlog/README.md`（若 QA 追加结论）

## Done Definition
- [x] 满足验收点 1：分批迁移子任务与收口复核均已完成
- [x] 满足验收点 2：engineering 主项目已回写 `TASK-ENGINEERING-009` 完成态
- [x] 满足验收点 3：engineering 主项目状态已切为 completed

## Risks / Blockers
- 风险：后续若新增 legacy 文档，必须新建专题，避免污染本次完成态
- 阻断项：无
- 需要升级给谁：如 QA 发现冻结快照与现存目录仍有口径冲突，升级给 `producer_system_designer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：`python - <<'PY' ... 冻结快照 existing gap 校验 ... PY && grep -nF -- '- [x] TASK-ENGINEERING-009' doc/engineering/project.md && rg -n "当前状态: completed|下一任务: 无" doc/engineering/project.md doc/engineering/doc-migration/legacy-doc-migration-collaboration-2026-03-03.project.md`

## Notes
- 接收方确认范围：`已确认 active legacy 文档迁移 umbrella 任务具备完成态证据链，可关闭 TASK-ENGINEERING-009`
- 接收方确认 ETA：`2026-03-11 same-day`
- 接收方新增风险：`无`
