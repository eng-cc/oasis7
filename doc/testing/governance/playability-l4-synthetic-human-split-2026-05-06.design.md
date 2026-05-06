# oasis7: 好玩性 L4 synthetic/human 分层（2026-05-06）设计

- 对应需求文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.prd.md`
- 对应项目管理文档: `doc/testing/governance/playability-l4-synthetic-human-split-2026-05-06.project.md`

审计轮次: 1

## 1. 设计定位
把 `L4` 从单一标签改成双层内部证据：`L4A synthetic` 负责内部高强度模拟，`L4B human` 负责真人继续游玩意愿。

## 2. 结构
- `L4A`:
  - 标准角色 subagent
  - simulated player personas
  - formal player surface 上的 synthetic continuation prediction
- `L4B`:
  - `run-producer-playtest.sh`
  - playability test card
  - headed human rerun / 受控访谈

## 3. 数据流
- `L4A outputs`:
  - `synthetic_continue_likely/unclear/unlikely`
- `L4B outputs`:
  - `human_continue_observed/mixed/not_observed`
- 汇总:
  - `synthetic_ready`
  - `human_ready`
  - `external_ready`
- Current scaffold:
  - `scripts/prepare-playability-l4-review.sh` 负责把 `L4A` packet/cards、`L4B` 卡片副本、summary 和推荐命令固定到同一 artifact 目录。
  - 该 scaffold 只收口 operator 入口，不改变 `L4A` 不能替代 `L4B` 的边界。

## 4. 约束
- `L4A` 不能直接替代 `L4B`。
- 若 `L4A` 与 `L4B` 冲突，必须显式记录 divergence。
- calibration 只保留为未来扩展，不属于本轮可用能力。
