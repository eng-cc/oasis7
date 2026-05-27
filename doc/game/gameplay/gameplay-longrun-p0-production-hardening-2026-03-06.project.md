# Gameplay Long-Run P0 Production Hardening（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`

审计轮次: 1

## 审计备注
- 主项目入口：`doc/game/project.md`
- 专题 PRD：`doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- 本文档仅维护长期在线 P0 生产硬化专题的执行任务，不重复定义目标态规格。

## 任务拆解（含 PRD-ID 映射）

### T0 文档建档
- [x] TASK-GAME-012 (PRD-GAME-006) [test_tier_required]: 新增专题 PRD 与项目管理文档，完成根文档追踪映射。

### T1 状态权威分层
- [x] TASK-GAME-013 (PRD-GAME-006-01) [test_tier_required]: 明确传播层/裁决层提交边界，落地冲突仲裁拒绝路径与审计事件。

### T2 回放与回滚
- [x] TASK-GAME-014 (PRD-GAME-006-02) [test_tier_required + test_tier_full]: 建立回放漂移定位、快照回滚 runbook、恢复对账自动化。

### T3 反作弊与反女巫
- [x] TASK-GAME-015 (PRD-GAME-006-03) [test_tier_required + test_tier_full]: 扩展对抗样本检测、惩罚与申诉证据链，补齐漏检与误伤监控。

### T4 经济闭环
- [x] TASK-GAME-016 (PRD-GAME-006-04) [test_tier_required]: 建立经济源汇审计任务与异常阈值门禁（通胀/套利风险）。

### T5 可运维性
- [x] TASK-GAME-017 (PRD-GAME-006-05) [test_tier_required]: 对齐 SLO、告警升级、灰度策略、灾备演练并纳入发布阻断规则。

## 依赖
- `doc/game/prd.md`
- `doc/game/project.md`
- `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
- `doc/game/gameplay/gameplay-longrun-p0-replay-rollback-runbook-2026-03-06.md`
- `doc/world-runtime/prd.md`
- `testing-manual.md`
- `scripts/p2p-longrun-soak.sh`

## 状态
- 更新日期: 2026-03-06
- 当前状态: completed
- 下一任务: `无（PRD-GAME-006 已闭环）`
- 阻塞项: 无
- 说明: 过程记录写入 `doc/devlog/README.md`，后续按任务完成度逐项回写状态。
