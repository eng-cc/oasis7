# Viewer 控制可预期性改版任务清单设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-control-predictability-tasklist-2026-02-28.project.md`

> 2026-06-25 状态说明：本文保留为早期控制可预期性任务清单和历史风险锚点；当前正式玩家控制反馈 surface 以 `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md`、`PRD-GAME-014` 和 `software_safe` 的 `Control Proof` / `Agency Moves` 测试为准。本文不再单独代表当前 canonical viewer 方案。

## 1. 设计定位
定义围绕“玩家控制代理感”的控制可预期性改版任务清单：把 accepted 无推进、控制命中率低和卡住恢复体验差等问题收敛为可解释、可恢复、可量化的一组任务。

## 2. 设计结构
- 指标目标层：TTFC、命中率、无进展窗口和主观意愿形成统一门槛。
- 状态模型层：区分 accepted / executing / completed / blocked 等反馈语义。
- 可发现性层：向玩家直接展示高频可用动作与最小参数示例。
- 恢复路径层：无进展时提供原因和推荐动作，而不是仅靠日志排障。

## 3. 关键接口 / 入口
- TTFC / effectiveControlHitRate / maxNoProgressWindowMs
- 控制反馈状态模型
- `run-game-test-ab.sh`
- Playability 卡片与门槛

## 4. 约束与边界
- 不在本轮改核心经济/战斗/治理规则。
- Viewer 只能优化反馈、引导和恢复路径，runtime 深层问题需分阶段治理。
- 指标和主观评分门槛需要同步维护，避免只看单一口径。
- 本文档是改版任务清单，不直接替代具体功能专题实现文档。

## 5. 设计演进计划
- 先冻结目标指标与反馈状态模型。
- 再按任务分层推进控制反馈和恢复能力。
- 最后通过 A/B 数据与卡片评分验证“可预期性”改版是否有效。
