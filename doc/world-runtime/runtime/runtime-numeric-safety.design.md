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
| Membership/clock | `membership_reconciliation.rs`、`membership_recovery/stores.rs`、`dht.rs`、`crates/oasis7_node/src/runtime_util.rs`、`crates/oasis7_net/src/util.rs` | schedule expiry 在写前受检；Duration 窄化按接口选择失败或显式 clamp。net 时钟的边界与测试由下文 `numeric-clock-clamp` 条款维护。 |
| PoS ratio/required stake | `crates/oasis7_proto/src/distributed_pos.rs`、`crates/oasis7_consensus/src/pos.rs`、`crates/oasis7_node/src/pos_validation.rs` | `> 1/2` 判定不通过可能溢出的乘法实现；required stake 以加宽计算和可失败窄化保持三层一致。 |
| Membership retry/replay | `crates/oasis7_consensus/src/membership_recovery/types.rs`、`mod.rs`、`replay.rs` | retry/backoff、调度/cooldown 与计数/容量后继值先受检；阈值/比率比较保持数学语义；失败不部分持久化。 |
| Governance archive/audit | `membership_recovery/replay_archive.rs`、`replay_archive_tiered.rs`、`replay_audit.rs` | drill、retention、offload、rollback 与 alert window 的时间/计数异常在状态更新前显式失败。 |
| Reconciliation/sync/mempool | `membership_reconciliation.rs`、`membership_recovery_support.rs`、`mempool.rs` | 时间差、report counter 与 payload aggregation 使用 checked helper；溢出返回 `WorldError::DistributedValidationFailed`，不得污染 schedule、dedup、report 或 batch。 |
| Federated archive | `membership_recovery/replay_archive_federated.rs` | 聚合与 composite cursor 后继值先受检；失败保留 cursor/aggregate state。 |
| Rolling IDs | `runtime/snapshot.rs`、`runtime/world/mod.rs`、`runtime/world/persistence.rs` | event/action/intent/proposal 的 `era + sequence` 一起持久化；`u64::MAX` 后分配 `(era + 1, 1)`，缺失 era 的旧 snapshot 按 `serde(default)` 读取为 0。 |

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [PRD-WORLD_RUNTIME-041](../prd.md#prd-world-runtime-041) | 可失败的权威数值推进先校验再提交；规则成本溢出形成拒绝且不部分应用。 | [原子数值转移](runtime-numeric-safety.design.md#atomic-numeric-transition) | `runtime_engineer`；资源与事件应用实现。 | 单项规则成本回归不证明全仓数值安全、分布式恢复或发布就绪。 |
| [net clock clamp 专业条款](runtime-numeric-safety.prd.md#numeric-clock-clamp) | 保留 net 时钟毫秒转换的有损边界；范围内保值、超界为 `i64::MAX`、epoch 前为 `0`。 | [clock clamp](runtime-numeric-safety.design.md#numeric-clock-clamp) | `runtime_engineer`；`oasis7_net` 时钟消费者。 | 不改变其他受检时钟接口，不将边界值解释成精确时间。 |

<a id="atomic-numeric-transition"></a>

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

<a id="numeric-clock-clamp"></a>

### net clock clamp

[`crates/oasis7_net/src/util.rs`](../../../crates/oasis7_net/src/util.rs) 中，`duration_millis_to_i64_saturating` 先通过 `Duration::as_millis()` 得到毫秒值，再以 `i64::try_from` 窄化；超界返回 `i64::MAX`。`unix_now_ms_i64` 对 `SystemTime::now().duration_since(UNIX_EPOCH)` 成功值调用该转换，epoch 前的错误返回 `0`。这些辅助函数只提供时钟读数，不写入世界状态；调用方的 lease、schedule 或持久化转移仍须满足各自受检与原子性合同。

现有内联单测 `tests::duration_millis_to_i64_saturating_keeps_in_range_values` 验证 `1234 ms` 保值，`tests::duration_millis_to_i64_saturating_clamps_overflow_to_i64_max` 验证 `Duration::from_secs(u64::MAX)` 返回 `i64::MAX`。这两项单测不覆盖系统时间早于 epoch 的分支；该分支目前按源码回读确认，不能写成已经通过真实时钟回拨测试。

## 5. 恢复、复制与观测

- snapshot 恢复必须在加载权威状态前验证所有必要后继值。
- replication writer、gap-fill 和 proposal 摄取不得用饱和值伪造进度。
- 错误日志或 rejection evidence 至少包含子系统、字段和导致失败的值。
- 相关测试必须同时断言错误与“状态/持久化未变化”。

## 6. 演进边界

rollover 不建立全链路复合 ID ABI：仅 runtime 内持久化 era 并保持恢复/同步兼容，纯 `u64` ID 仍可能跨 era 复用。所有失败路径继续要求结构化错误、rejection evidence 与状态/持久化未变化的回归。

## 11. 验证设计与可追溯性

以下关系维护本专题受影响的数值与时钟边界；实际候选、命令、结果和未运行项记录在 PR 与真实验证输出中。文档修补按 [测试手册](../../../testing-manual.md) 选择 S0；数值实现变化时再执行对应定向测试，不能把本表中的测试计划写成本次已经通过的结果。

<a id="numeric-safety-validation"></a>
### 11.1 验证映射表

| 上游 requirement / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [PRD-WORLD_RUNTIME-041](../prd.md#prd-world-runtime-041) | [原子数值转移](runtime-numeric-safety.design.md#atomic-numeric-transition) | 规则成本的某个资源项溢出时拒绝动作，其他资源项保持不变。 | 固定候选；按 [rules_enforcement.rs](../../../crates/oasis7/src/runtime/tests/rules_enforcement.rs) 的 `rule_decision_cost_overflow_rejected_without_partial_apply` 执行 L1 回归：`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required rule_decision_cost_overflow_rejected_without_partial_apply`。文档修补仅核对合同与现有断言。 | PR 中的源码回读；运行时记录候选、命令与实际结果。 | 不代表其余 numeric phase、durable write、rollover、长稳或跨节点验收。 |
| [net clock clamp 专业条款](runtime-numeric-safety.prd.md#numeric-clock-clamp) | [clock clamp](runtime-numeric-safety.design.md#numeric-clock-clamp) | 范围内毫秒值保留，超界夹到 `i64::MAX`；epoch 前返回 `0` 的当前实现不得被误写成受检错误。 | 固定候选；按 [测试手册 S4](../../../testing-manual.md) 选择 net 单测，窄命令 `env -u RUSTC_WRAPPER cargo test -p oasis7_net --lib duration_millis_to_i64_saturating_` 对应上文两项内联测试；epoch 前分支另行回读 `crates/oasis7_net/src/util.rs::unix_now_ms_i64`。 | PR 中的代码/测试定位；实际运行时记录两个用例及执行环境。 | 两项单测不验证真实系统时钟回拨、epoch 前分支或消费者的分布式行为。 |
