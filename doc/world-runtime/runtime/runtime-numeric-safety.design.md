# Runtime 数值安全与原子状态转移设计

- 对应需求文档：`doc/world-runtime/runtime/runtime-numeric-safety.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

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
| PoS ratio/required stake | `crates/oasis7_proto/src/distributed_pos.rs`、`crates/oasis7_consensus/src/pos.rs`、`crates/oasis7_node/src/pos_validation.rs` | `> 1/2` 判定不通过可能溢出的乘法实现；required stake 以加宽计算和可失败窄化保持三层一致。 |
| Membership retry/replay | `crates/oasis7_consensus/src/membership_recovery/types.rs`、`mod.rs`、`replay.rs` | retry/backoff、调度/cooldown 与计数/容量后继值先受检；阈值/比率比较保持数学语义；失败不部分持久化。 |
| Governance archive/audit | `membership_recovery/replay_archive.rs`、`replay_archive_tiered.rs`、`replay_audit.rs` | drill、retention、offload、rollback 与 alert window 的时间/计数异常在状态更新前显式失败。 |
| Reconciliation/sync/mempool | `membership_reconciliation.rs`、`membership_recovery_support.rs`、`mempool.rs` | 时间差、report counter 与 payload aggregation 使用 checked helper；溢出返回 `WorldError::DistributedValidationFailed`，不得污染 schedule、dedup、report 或 batch。 |
| Federated archive | `membership_recovery/replay_archive_federated.rs` | 聚合与 composite cursor 后继值先受检；失败保留 cursor/aggregate state。 |
| Rolling IDs | `runtime/snapshot.rs`、`runtime/world/mod.rs`、`runtime/world/persistence.rs` | event/action/intent/proposal 的 `era + sequence` 一起持久化；`u64::MAX` 后分配 `(era + 1, 1)`，缺失 era 的旧 snapshot 按 `serde(default)` 读取为 0。 |

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

rollover 不建立全链路复合 ID ABI：仅 runtime 内持久化 era 并保持恢复/同步兼容，纯 `u64` ID 仍可能跨 era 复用。所有失败路径继续要求结构化错误、rejection evidence 与状态/持久化未变化的回归。
