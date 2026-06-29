# oasis7: 版本级候选 readiness 扩展（2026-03-11）（项目管理）

- 对应设计文档: `doc/core/release-candidate-version-escalation-2026-03-11.design.md`
- 对应需求文档: `doc/core/release-candidate-version-escalation-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] VRC-1 (PRD-CORE-VRC-001/002) [test_tier_required]: 定义版本级候选扩展规则与 runtime 长跑三槽位。
- [x] VRC-2 (PRD-CORE-VRC-001/003) [test_tier_required]: 生成首份版本级候选看板，并挂接现有入口。
- [x] VRC-3 (PRD-CORE-VRC-002/003) [test_tier_required]: 完成 `producer_system_designer -> qa_engineer` handoff，并将 core 下一任务推进到 runtime 联合证据补齐。

## 依赖
- `doc/core/release-candidate-version-escalation-2026-03-11.prd.md`
- `doc/core/release-candidate-version-escalation-2026-03-11.design.md`
- `doc/core/reviews/release-candidate-readiness-board-task-game-018-2026-03-11.md`
- `doc/core/reviews/release-candidate-readiness-board-version-2026-03-11.md`
- 历史 version-candidate handoff 文件已退役删除；当前证据入口为本项目页、正式专题三件套、readiness board 与 `.pm/tasks/task_<32hex>.execution.md`。

## 状态
- 更新日期: 2026-03-11
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步: 执行 `TASK-CORE-020`，补齐 runtime footprint / GC / soak 联合证据并刷新版本级候选看板。
