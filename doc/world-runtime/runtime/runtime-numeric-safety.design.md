# Runtime 数值安全与原子状态转移设计

- 对应需求文档：`doc/world-runtime/runtime/runtime-numeric-safety.prd.md`
- 对应项目文档：`doc/world-runtime/runtime/runtime-numeric-safety.project.md`

## 1. 设计原则

- 后继值先计算、全部校验通过后再提交。
- 算术策略是接口契约的一部分：`Result` 路径采用 checked failure；协议 ID 可采用持久化 rollover；明确允许有损边界的辅助函数才可采用 observable clamp。
- 错误映射不得丢失字段、边界值与所属子系统。
- 恢复和复制路径的数值失败必须 fail closed。

## 2. 子系统权威图

| 子系统 | 当前实现入口 | 设计要求 |
| --- | --- | --- |
| World 资源/规则 | `crates/oasis7/src/simulator/types.rs`、`runtime/world/resources.rs`、`runtime/world/step.rs`、`runtime/state/apply_domain_event_gameplay.rs` | 资源与规则聚合受检；事件 effect 预计算；失败转为拒绝且不部分提交。 |
| Node points | `crates/oasis7/src/runtime/node_points.rs` | award、cumulative、total 与 epoch successor 全部计算成功后一次提交。 |
| PoS | `crates/oasis7_consensus/src/node_pos.rs` | stake/slot 越界返回错误；proposal/vote 状态保持不变。 |
| Node engine/recovery | `crates/oasis7_node/src/pos_state_store.rs` 及 PosNodeEngine 路径 | height/slot/gap/snapshot 后继值受检；不可表示的 snapshot 拒绝启动。 |
| Replication | `crates/oasis7_node/src/replication.rs` | writer position 返回 `Result`；local commit 构造透传错误。 |
| Sequencer/lease | `crates/oasis7_consensus/src/sequencer_mainloop.rs`、`lease.rs` | slot/height/term/expiry 先计算，成功后才更新权威状态。 |
| Membership/clock | `membership_reconciliation.rs`、`membership_recovery/stores.rs`、`dht.rs`、`crates/oasis7_node/src/runtime_util.rs` | schedule expiry 在写前受检；Duration 窄化按接口选择失败或显式 clamp。 |

## 3. 状态转移模式

```text
read current state
  -> compute every numeric successor
  -> validate bounds and cross-field invariants
  -> construct complete effect / durable record
  -> commit once
```

任一步失败都直接返回稳定错误；提交前不得修改权威状态。调用方需要继续循环时，必须把错误转换为明确 rejection/evidence，而不是吞掉错误。

## 4. 策略选择

- `checked_add` / `checked_sub` / `try_from`：默认选择，适用于余额、票权、时间差、位置推进与可失败 API。
- rollover：仅在复合代际信息已经持久化且恢复/同步兼容时使用。
- clamp：仅用于函数签名明确承诺边界夹逼的兼容辅助函数；必须返回确定边界并由测试证明，调用方不得据此生成精确时间声明。

## 5. 恢复、复制与观测

- snapshot 恢复必须在加载权威状态前验证所有必要后继值。
- replication writer、gap-fill 和 proposal 摄取不得用饱和值伪造进度。
- 错误日志或 rejection evidence 至少包含子系统、字段和导致失败的值。
- 相关测试必须同时断言错误与“状态/持久化未变化”。

## 6. 演进边界

phase 7–15 仍是独立专题。后续吸收时应追加子系统权威图和 evidence ledger，而不是把未复核内容预先归入本文。
