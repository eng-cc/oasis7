# oasis7 Runtime：PoS 槽内 Tick 相位门控与自适应节拍（10 Tick/Slot）

- 对应设计文档: `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.design.md`
- 对应项目管理文档: `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: 现有 PoS 仅按 slot 进行提案门控，缺少槽内 tick 相位语义；运行线程使用固定 `tick_interval`，难以稳定贴合目标的 `10 tick/slot` 节奏。
- Proposed Solution: 在 `PosNodeEngine` 引入 `ticks_per_slot` 逻辑 tick 公式与槽内相位门控（仅在目标相位允许提案），并在 `NodeRuntime` 增加基于 wall-clock 的自适应调度等待时间计算。
- Success Criteria:
  - SC-1: 相同 `now_ms/genesis/slot_duration_ms/ticks_per_slot` 输入在多节点计算出一致的 `logical_tick/slot/tick_phase`。
  - SC-2: 使用 `config/chain-pos-defaults.env` 中的 `POS_SLOT_DURATION_MS` 与 `POS_TICKS_PER_SLOT` 默认基线时，每个 slot 最多触发一次本地提案窗口。
  - SC-3: 稳态提案仅在配置相位触发（默认 `tick_phase == ticks_per_slot-1`）；仅当本 tick 观察到严格 slot 落后（`next_slot < current_slot`）并把 `next_slot` 对齐到 `current_slot` 时，允许同一 tick 使用一次 off-phase 恢复窗口。该窗口 edge-triggered、non-sticky，若被更高优先级 guard 阻断则不保留重试资格。
  - SC-4: 快照可观测 `last_observed_tick/missed_tick_count/tick_phase/ticks_per_slot`。
  - SC-5: 覆盖 `test_tier_required` 与 `test_tier_full` 定向回归。

## 2. User Experience & Functionality
- User Personas:
  - 协议维护者：需要明确“每 slot 10 tick”的协议语义与可验证计算规则。
  - 节点运营者：需要节点在时钟抖动下仍保持稳定节拍，避免固定 sleep 引入累计漂移。
  - 质量复核者：需要可观测指标来验证 tick 相位门控和漏 tick 统计。
- User Scenarios & Frequency:
  - 共识参数更新后执行一次节拍一致性回归。
  - 多节点联测时按批次检查提案是否落在期望相位。
  - 线上抖动演练后复核漏 tick 与调度行为。
- User Stories:
  - PRD-P2P-NODE-TICK-001: As a 协议维护者, I want logical tick derived from wall-clock within slot, so that `10 tick/slot` semantics are deterministic.
  - PRD-P2P-NODE-TICK-002: As a 节点运营者, I want proposal phase gating, so that block proposal cadence follows configured tick phase.
  - PRD-P2P-NODE-TICK-003: As a 运维/质量复核者, I want adaptive runtime pacing and metrics, so that drift and missed ticks are observable.
- Critical User Flows:
  1. Flow-NODE-TICK-001: `节点读取 wall-clock -> 计算 logical_tick/slot/phase -> 更新单调观测游标`
  2. Flow-NODE-TICK-002: `观察 next_slot < current_slot -> 对齐到 current_slot 并只为当前 tick 生成一次恢复边沿 -> 依次检查 pending/replication hold/participation safety/local proposal enablement/expected proposer 等既有 guard -> guard 全部通过且（命中目标相位或当前 tick 持有恢复边沿）才允许提案 -> tick 结束即丢弃恢复边沿`
  3. Flow-NODE-TICK-003: `运行线程计算下一 logical tick 边界 -> 动态等待 -> timeout 后执行引擎 tick`
  4. Flow-NODE-TICK-004: `节点抖动导致 tick 跳跃 -> 统计 missed_tick_count -> 快照暴露用于审计`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 槽内 logical tick 计算 | `slot_duration_ms`、`ticks_per_slot`、`slot_clock_genesis_unix_ms`、`now_ms` | 每次 tick 计算 `logical_tick/slot/tick_phase` | `observed_tick` 单调递增 | `tick=floor((now-genesis)*ticks_per_slot/slot_duration)`；`slot=tick/ticks_per_slot`；`phase=tick%ticks_per_slot` | 全节点统一公式 |
| 提案相位门控与严格落后恢复 | `next_slot`、`current_slot`、`tick_phase`、`proposal_tick_phase`、本 tick 对齐结果 | 稳态仅在目标相位触发；严格落后对齐 tick 可使用一次 off-phase 恢复窗口 | `strict_lag -> aligned/recovery_edge -> proposing|idle`；下一 tick 回到 exact-phase gate | `next_slot<=current_slot && (tick_phase==proposal_tick_phase || aligned_this_tick)`；`aligned_this_tick` 不持久化、不重放 skipped slots | 仅 expected validator proposer 可触发；pending、replication hold、participation safety、local proposal enablement、签名、投票阈值、execution binding、fork choice、finality 等 guard 均不绕过 |
| 自适应调度 | `adaptive_tick_scheduler_enabled`、`tick_interval` | 运行线程按下一 logical tick 边界动态等待 | `waiting -> timeout -> tick` | `wait=max(1, next_boundary-now)`，异常时回退固定间隔 | 本地调度逻辑，无跨节点权限 |
| 可观测指标 | `last_observed_tick`、`missed_tick_count`、`tick_phase`、`ticks_per_slot` | 快照暴露当前节拍与漏 tick 情况 | `updated` | 漏 tick 按逻辑 tick 跳跃量累加 | 只读暴露 |
- Acceptance Criteria:
  - AC-1: `NodePosConfig` 支持 `ticks_per_slot`、`proposal_tick_phase`、`adaptive_tick_scheduler_enabled` 配置与校验。
  - AC-2: `PosNodeEngine` 使用 logical tick 计算 `slot/tick_phase`，并保持回拨不倒退。
  - AC-3: 稳态提案仅在 `proposal_tick_phase` 命中时触发，默认 `ticks_per_slot-1`；严格落后对齐只在同一 tick 提供一次 non-sticky off-phase 恢复窗口。若该 tick 被 pending、replication hold、participation safety、local proposal disablement 或 expected-proposer 等 guard 阻断，后续 off-phase tick 不得重试，必须等待配置相位或新的严格落后边沿。
  - AC-4: `NodeConsensusSnapshot` 与持久化快照补齐 tick 级可观测字段。
  - AC-5: `NodeRuntime` 支持动态等待下一 logical tick 边界，并在异常时回退固定间隔。
  - AC-6: 覆盖单元与跨节点定向回归（phase 门控、漏 tick、调度等待、与既有窗口校验兼容）。
- Non-Goals:
  - 不在本任务引入链上参数治理流程（参数变更生效治理另立专题）。
  - 不在本任务改造 fork choice/finality 规则。
  - 不在本任务保证“每 slot 必提交”，仅保证提案节奏门控。
  - 不补提或重放被跳过的 slots，不把恢复边沿持久化为跨 tick 状态，也不放宽 proposer 选择、参与安全、复制、签名、投票阈值、execution binding 或 finality guard。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 在 `PosNodeEngine` 扩展 slot 内 logical tick 观测模型；在 `NodeRuntime` worker loop 以 logical tick 边界为参考计算下一次 `recv_timeout`；共识提交逻辑维持既有投票阈值模型。
- Integration Points:
  - `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
  - `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.project.md`
  - `crates/oasis7_node/src/types.rs`
  - `crates/oasis7_node/src/lib.rs`
  - `crates/oasis7_node/src/lib_impl_part1.rs`
  - `crates/oasis7_node/src/pos_state_store.rs`
  - `crates/oasis7_node/src/runtime_util.rs`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - `ticks_per_slot == 0`：配置阶段拒绝。
  - `proposal_tick_phase >= ticks_per_slot`：配置阶段拒绝。
  - `slot_duration_ms` 与 `ticks_per_slot` 非整除：按整数边界公式计算，不要求整除。
  - `now_ms < genesis_unix_ms`：钳制为 tick=0，并保持单调观测。
  - 时钟回拨：`last_observed_tick` 不倒退；仅记录最新有效值。
  - 动态调度计算溢出或异常：回退到固定 `tick_interval`。
- Non-Functional Requirements:
  - NFR-1: logical tick 计算为 O(1)/tick。
  - NFR-2: 回拨场景下 `last_observed_tick` 倒退容忍度为 0。
  - NFR-3: 动态调度不降低停止信号响应（维持 `recv_timeout` 可中断）。
  - NFR-4: 不削弱既有签名校验、proposal/attestation 时间窗口校验。
- Security & Privacy: 调度与相位门控仅影响时间节奏，不放宽 validator 身份、签名与执行绑定校验。

## 5. Risks & Roadmap
- Phased Rollout:
  - M0: 文档建档与任务拆解。
  - M1: logical tick 与相位门控落地。
  - M2: 运行线程动态调度与快照可观测。
  - M3: required/full 回归收口。
- Technical Risks:
  - 风险-1: 启用动态调度后若参数配置不当，可能改变现网吞吐节奏。
  - 风险-2: 相位门控会拉长“无票/低票”场景的可见提交间隔。
  - 风险-3: 与历史依赖固定 `tick_interval` 的测试基线存在偏差。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-NODE-TICK-001 | TASK-P2P-009-T1 | `test_tier_required` | 单元测试：logical tick/slot/phase 计算、回拨单调 | PoS 引擎时钟观测路径 |
| PRD-P2P-NODE-TICK-002 | TASK-P2P-009-T1/T3、task_813a3014b1124abfb09f01e50a4597a4 | `test_tier_required` + `test_tier_full` | 单元+跨节点：稳态 exact-phase 门控、严格落后同 tick 恢复、non-sticky 序列、disabled/participation guard 消耗恢复边沿、expected proposer 不绕过，以及每 slot 单提案窗口 | 共识提案触发路径 |
| PRD-P2P-NODE-TICK-003 | TASK-P2P-009-T2/T3 | `test_tier_required` + `test_tier_full` | runtime 调度等待计算、漏 tick 指标与端到端回归 | 运行线程节拍与可观测 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-NODE-TICK-001 | 使用 slot 内 logical tick 与相位门控 | 保持“任意 tick 满足 `next_slot<=current_slot` 即提案” | 需要显式 `10 tick/slot` 节奏语义。 |
| DEC-NODE-TICK-002 | 动态调度仅调整 worker 等待时间 | 修改共识阈值以强制每 slot 提交 | 不应改变共识安全判定。 |
| DEC-NODE-TICK-003 | 动态调度异常时回退固定间隔 | 异常直接停机 | 优先可用性与渐进降级。 |
| DEC-NODE-TICK-004 | 严格 slot 落后对齐产生仅当前 tick 有效的一次性 off-phase 恢复边沿；稳态继续 exact-phase gate | 持久化恢复资格并在后续 off-phase tick 重试；逐个重放 skipped slots；永久取消 phase gate | 2026-07-14 producer decision：恢复 fresh-validator liveness 的同时保持边沿 non-sticky，且不绕过任何既有共识/复制/参与安全 guard。Trace: issue #2264 comment 4970131779。 |
