# oasis7 Runtime：Observer 同步源运行态统计设计

- 对应需求文档: `doc/p2p/observer/observer-sync-mode-runtime-metrics.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 1. 设计定位
本文件记录 Observer 同步源运行态统计的历史设计合同及 dormant 状态，不表示当前 crate 提供这些运行时指标。`crates/oasis7_net/src/lib.rs` 当前未声明 `observer` 或 `observer_metrics`，也未从 crate facade 导出本设计所列 API；相关源文件和同文件测试未进入 `oasis7_net` 当前编译模块图或其 test targets。

同步源选择、追平进度与回退状态只有在 runtime 变更单独接回模块图和所需 facade 后，才可成为可调用运行能力。文件存在、设计描述或 `test_tier_required` 标签均不构成编译、执行或测试通过证据；本文不声称任何当前 S9A tier 已通过。

## 2. 设计结构
- 状态采集层（重新激活目标）：采集同步源选择、追平高度和延迟。
- 模式报告层（重新激活目标）：非 DHT/DHT observed report 同时保留 mode、原始同步报告与真实 `fallback_used`。
- 自动记录桥接层（重新激活目标）：单轮成功报告记录一次，follow 按轮记录且复用原聚合/终止语义；metrics 继续由调用方持有。
- 指标输出层（重新激活目标）：沉淀 runtime metrics 与聚合字段。
- 告警信号层（重新激活目标）：为异常 lag、切源与失败建立告警基础。
- 回归验证层（重新激活门槛）：通过已编译并执行的定向测试校验指标准确性，再按 S9A 所需 tier 取得独立证据；当前没有此类通过声明。

<a id="observer-metrics-dormant-status-contract"></a>
### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [PRD dormant status boundary](observer-sync-mode-runtime-metrics.prd.md#observer-metrics-dormant-status-requirement) | 当前保持 metrics source dormant；仅把接口、计数和测试列为重新激活目标，不把文件或 `test_tier_required` 读成 active API 或 pass evidence。 | [本设计的 dormant status contract](observer-sync-mode-runtime-metrics.design.md#observer-metrics-dormant-status-contract) | `runtime_engineer` 确认模块图/facade；`qa_engineer` 审核测试和 tier 证据边界。 | 不在此激活 Rust 模块、声称 S9A tier 通过、公开可达性或 release readiness。 |

## 3. 关键接口 / 入口
- 以下名称描述 dormant source 的设计目标，不是当前 crate 可调用接口：
  - 同步源运行时状态
  - `HeadSyncModeReport` / `HeadSyncModeWithDhtReport`
  - 单轮/follow metrics bridge
  - runtime metrics 字段
  - 告警聚合入口
  - 指标回归场景

## 4. 约束与边界
- 指标必须来源于真实运行态，不得手工拼装。
- 字段命名需与 observer 其他文档保持一致。
- fallback 标识必须在模式分发层计算，桥接不得引入隐式全局 metrics 或改变 `HeadFollowReport`。
- 不在本专题定义完整告警策略。
- 重新激活须由单独的 runtime 变更接回 source 模块与所需 facade，并让相关测试通过 `oasis7_net` 的受支持目标实际编译。colocated test 源码目前不属于 crate test targets。
- PRD 中的 `test_tier_required` 表示未来重新激活前的验证义务，不是运行结果。`module_required`、`module_full`、`integration_required` 与 `release_full` 的范围及证据边界以 [S9A 多节点状态同步闭环设计](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.design.md) 为准；其他 tier、代理或测试绿灯不能替代相应证据，也不证明公开可达性、testnet readiness 或完整游戏体验。

## 5. 设计演进计划
以下均为未来重新激活后的顺序目标，当前未由本文声称已交付：
- 先由 runtime 变更接回模块图/facade，并验证运行态字段。
- 再接 metrics 输出。
- 最后补告警口径与定向回归，并按 S9A 适用 tier 单独记录结果。

## 11. 验证与证据
### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [PRD dormant status boundary](observer-sync-mode-runtime-metrics.prd.md#observer-metrics-dormant-status-requirement) | [本设计的 dormant status contract](observer-sync-mode-runtime-metrics.design.md#observer-metrics-dormant-status-contract) | 本次只核对 dormant 模块与测试状态；未来重新激活必须实际编译对应模块并按 S9A 适用 tier 单独取证。 | N/A: reason=metrics source is not in the active crate module graph; scope=this dormant-source documentation snapshot and not a future runtime activation; owner_role=blockchain_ops_engineer; evidence_ref=https://github.com/eng-cc/oasis7/issues/3935#issuecomment-5834239459; re-evaluate=when a runtime-owned change admits these modules into the crate. | 本 Issue 的 exact-head review 与 doc/required-CI receipts；未来 runtime activation task 的 Cargo/S9A evidence。 | 本次不证明 source 已编译、metrics 运行、任何 tier 通过、公开网络可达、testnet readiness 或完整游戏体验。 |
