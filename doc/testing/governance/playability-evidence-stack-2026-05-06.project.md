# oasis7: 好玩性证据栈（2026-05-06）（项目管理）

- 对应设计文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.design.md`
- 对应需求文档: `doc/testing/governance/playability-evidence-stack-2026-05-06.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] playability-evidence-stack-definition (PRD-TESTING-PLAYABILITY-001) [test_tier_required]: 建立专题 PRD / Design / Project，冻结“自动化不能单独保证好玩”的正式口径，以及五层证据栈与 `go/watch/hold/block` 组合规则。 Trace: .pm/tasks/task_efe1a5f949e84160a1237302c9064168.yaml
- [x] playability-evidence-stack-surface-mapping (PRD-TESTING-PLAYABILITY-002) [test_tier_required]: 把 `software_safe`、`pure_api`、`--no-llm`、playability card、`run-producer-playtest.sh`、`player leverage` rubric 和 limited preview 现有入口映射进统一证据栈。 Trace: .pm/tasks/task_efe1a5f949e84160a1237302c9064168.yaml
- [x] playability-evidence-stack-root-entry-sync (PRD-TESTING-PLAYABILITY-003) [test_tier_required]: 同步模块根入口 `doc/testing/prd.md`、`doc/testing/project.md`、`doc/testing/README.md` 与 `doc/testing/prd.index.md`，保证首读分流可达。 Trace: .pm/tasks/task_efe1a5f949e84160a1237302c9064168.yaml
- [x] playability-evidence-stack-subagent-governance (PRD-TESTING-PLAYABILITY-005) [test_tier_required]: 补充“所有内部人工评审默认可由对应标准角色 subagent 补齐”的治理规则，并写明非标准 `player` 角色限制，以及 `subagent review != 真实外部玩家验证` 的边界。 Trace: .pm/tasks/task_efe1a5f949e84160a1237302c9064168.yaml

## 依赖
- `doc/testing/prd.md`
- `doc/testing/project.md`
- `testing-manual.md`
- `doc/testing/evidence/gameplay-ten-minute-trust-gate-2026-04-09.md`
- `doc/playability_test_result/playability_test_card.md`
- `doc/game/prd.md`

## 状态
- 更新日期: 2026-05-06
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步:
  - 若后续新增实验/遥测系统，按 L3 能力扩展补字段，不改低层/高层边界。
  - 若后续要升级 external claim envelope，先补 L5 受控外部信号样本，而不是重复堆自动化。
  - 若后续真的把多角色 subagent review 变成默认流水线，再追加一份 execution-oriented runbook，把各角色输入输出模板固定下来。
