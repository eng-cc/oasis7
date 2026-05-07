# oasis7: 好玩性 L4 synthetic/agent/human 分层（2026-05-06）设计

- 对应需求文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
- 对应项目管理文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md`

审计轮次: 2

## 1. 设计定位
把 `L4` 收口成两层正式内部证据，并把内部真人试玩降为 `L4B` 可选校准：
- `L4A synthetic`: 内部高强度模拟
- `L4B embodied-agent`: agent 实际进入产品并操作

## 2. 结构
- `L4A`:
  - 标准角色 subagent
  - simulated player personas
  - formal player surface 上的 synthetic continuation prediction
- `L4B`:
  - `run-producer-playtest.sh`
  - agent-browser / GUI Agent 实际操作
  - `l4b-agent-playtest-card.md`
- `L4B` optional corroboration:
  - `run-producer-playtest.sh`
  - headed internal human rerun / playability test card
  - `optional-internal-human-corroboration.md`

## 3. 数据流
- `L4A outputs`:
  - `synthetic_continue_likely/unclear/unlikely`
- `L4B outputs`:
  - `agent_continue_observed/mixed/not_observed`
- `L4B optional corroboration outputs`:
  - `corroborates_l4b/mixed/contradicts_l4b`
- 汇总:
  - `synthetic_ready`
  - `agent_ready`
  - `external_ready`
- Current scaffold:
  - `scripts/prepare-playability-l4-review.sh` 负责把 `L4A` packet/cards、`L4B` agent 卡、可选内部真人佐证 notes、summary 和推荐命令固定到同一 artifact 目录。
  - 该 scaffold 只收口 operator 入口，不改变 `L4A` 不能替代 `L4B`、`L4B` 不能替代 `L5` 的边界。

## 4. 约束
- `L4A` 不能直接替代 `L4B`。
- `L4B` 不能直接替代 `L5`。
- 若 `L4A` 与 `L4B` 冲突，必须显式记录 divergence。
- 若内部真人校准与 `L4B` 冲突，必须显式记录 divergence，并保留 `L5` 缺失状态。
- calibration 只保留为未来扩展，不属于本轮可用能力。
