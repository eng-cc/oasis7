# oasis7: 好玩性 L4 synthetic/human 分层（2026-05-06）（项目管理）

- 对应设计文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.design.md`
- 对应需求文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] l4-split-boundary (PRD-TESTING-L4SPLIT-001/002) [test_tier_required]: 把 evidence stack 中的 `L4` 正式拆成 `L4A synthetic` 与 `L4B human`，并同步根模块入口。 Trace: .pm/tasks/task_a68decb0a2c8460aa7d989df7370c901.yaml
- [x] l4-split-operator-entry (PRD-TESTING-L4SPLIT-003) [test_tier_required]: 同步 `testing-manual` 的 operator 入口，把 `worktree-harness/S6` 与 `run-producer-playtest/playability card` 分到不同子层。 Trace: .pm/tasks/task_a68decb0a2c8460aa7d989df7370c901.yaml
- [x] l4-split-calibration-boundary (PRD-TESTING-L4SPLIT-004) [test_tier_required]: 明确未来 calibration 扩展位存在，但当前不宣称 `L4A` 已可替代 `L4B`。 Trace: .pm/tasks/task_a68decb0a2c8460aa7d989df7370c901.yaml

## 依赖
- `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`
- `doc/testing/governance/playability-subagent-review-system-2026-05-06.prd.md`
- `doc/testing/governance/playability-simulated-player-persona-panel-2026-05-06.prd.md`
- `testing-manual.md`
- `doc/playability_test_result/playability_test_card.md`

## 状态
- 更新日期: 2026-05-06
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步:
  - 若未来要提升 `L4A` 权重，先新增 calibration ledger/topic，再决定是否调整 claim envelope。
