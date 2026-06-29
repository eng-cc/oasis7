# oasis7: 发布候选 readiness 统一入口（2026-03-11）（项目管理）

- 对应设计文档: `doc/core/release-candidate-readiness-entry-2026-03-11.design.md`
- 对应需求文档: `doc/core/release-candidate-readiness-entry-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] CRR-1 (PRD-CORE-RR-001/002) [test_tier_required]: 定义候选 header、五类证据槽位与状态聚合规则。
- [x] CRR-2 (PRD-CORE-RR-001/003) [test_tier_required]: 将该主题回写到 core 主项目，并把下一任务推进到首份候选看板实例化。
- [x] CRR-3 (PRD-CORE-RR-002/003) [test_tier_required]: 完成 `producer_system_designer -> qa_engineer` handoff。

## 依赖
- `doc/core/release-candidate-readiness-entry-2026-03-11.prd.md`
- `doc/core/release-candidate-readiness-entry-2026-03-11.design.md`
- 历史 readiness handoff 文件已退役删除；当前证据入口为本项目页、正式专题三件套与 `.pm/tasks/task_<32hex>.execution.md`。
- `doc/core/project.md`
- `doc/core/reviews/stage-closure-go-no-go-task-game-018-2026-03-10.md`

## 状态
- 更新日期: 2026-03-11
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步: 执行 `TASK-CORE-018`，基于该入口实例化首份候选级看板。
