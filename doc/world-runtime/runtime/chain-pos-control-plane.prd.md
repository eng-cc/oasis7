# oasis7 Runtime：链 PoS 时间与控制面合同

审计轮次: 1

- 对应设计文档：`doc/world-runtime/runtime/chain-pos-control-plane.design.md`
- 对应项目管理文档：`doc/world-runtime/runtime/chain-pos-control-plane.project.md`

## 目标

建立唯一 runtime 专业 authority，使链 PoS 时间、tick 节拍、控制面参数、状态可观测性与恢复边界可从同一现行合同检索。

## 范围

涵盖共享时间锚、slot/tick 相位、入站时序验证、strict-lag 恢复、控制面参数/status 与 restart/replay 边界；不覆盖经济、运营和发布结论。

## 接口 / 数据

`slot_clock_genesis_unix_ms`、`slot_duration_ms`、`ticks_per_slot`、`proposal_tick_phase`、`max_past_slot_lag`、missed counters 与 consensus status snapshot。

## 里程碑

当前里程碑：建立稳定 authority 并由索引可达；后续参数或共识语义改动按 runtime 联审与定向回归推进。

## 风险

将 poll interval 与 slot duration 混同、使恢复边沿变为 sticky retry，或在 replay 中按当前 wall clock 补造历史后果，都会破坏跨节点确定性。

## 1. Executive Summary

- 本专题是链 PoS 时间、tick 节拍、控制面参数与状态可观测性的唯一 runtime 专业 authority。它吸收同一实现域中已完成的 slot clock、subslot pacing 与 time-anchor 对齐专题的现行合同；历史实现过程由 Git history 与 GitHub task evidence 追溯，不再以已完成专题作为当前入口。
- 本专题定义 runtime 当前行为，不定义 validator 经济、stake、奖励、主网承诺、节点实际运维拓扑或玩家规则。
- Success Criteria:
  - SC-1: 相同 `now_ms/genesis/slot_duration_ms/ticks_per_slot` 输入在节点上得到相同 `logical_tick/slot/tick_phase`。
  - SC-2: 控制面、launcher 和 status 使用同名且已校验的 PoS 参数；`chain_node_tick_ms` 不被误述为 slot 时长。
  - SC-3: restart/recovery 后 timing counters 与持久 snapshot 一致，且 replay 不补造被跳过的 slot。

## 2. Current Runtime Contract

### 2.1 时间与相位

| 合同字段 | 当前语义 | 拒绝/保护边界 |
| --- | --- | --- |
| `slot_clock_genesis_unix_ms` | slot clock 的共享时间锚；未显式设置时由 runtime 初始化并写入状态 | 所有节点必须使用同一受控配置/状态来源；不可用本地 poll interval 替代。 |
| `slot_duration_ms` | 一个 slot 的时长；`current_slot=floor((now_ms-genesis)/slot_duration_ms)` | 必须大于零。 |
| `ticks_per_slot` | 一个 slot 内的逻辑 tick 数；`logical_tick=floor((now_ms-genesis)*ticks_per_slot/slot_duration_ms)` | 必须大于零；不要求与 slot 时长整除。 |
| `proposal_tick_phase` | 稳态允许提案的 slot 内相位；`tick_phase=logical_tick%ticks_per_slot` | 必须小于 `ticks_per_slot`。 |
| `max_past_slot_lag` | 入站 proposal/attestation 可接受的最大历史 slot 滞后 | future slot 与超过窗口的 stale slot 必须拒绝。 |

`slot=logical_tick/ticks_per_slot`。`observed_slot` 与 `observed_tick` 只能单调推进；wall-clock 跨越 slot/tick 时，runtime 累计 `missed_slot_count` / `missed_tick_count` 并把游标对齐到当前，不为漏过区间补造历史块、提案或事件。

### 2.2 提案、入站校验与一次恢复边沿

- 稳态提案只可在 `tick_phase == proposal_tick_phase`、`next_slot <= current_slot` 且全部既有 guard 通过时发生。
- 当且仅当本 tick 观察到严格落后 `next_slot < current_slot` 并完成对齐，可在该 tick 使用一次 off-phase 恢复边沿。该边沿是 edge-triggered、non-sticky；tick 结束即丢弃，若被 guard 阻断，不保留为下一 tick 的重试资格。
- 恢复边沿不绕过 pending、replication hold、participation safety、local proposal enablement、expected proposer、签名、投票阈值、execution binding、fork choice 或 finality guard。
- proposal/attestation 必须拒绝 future 或超过 `max_past_slot_lag` 的 slot；attestation 的 target epoch 必须与其提案 slot 的 epoch 映射一致。

### 2.3 控制面与状态面

- `oasis7_chain_runtime` 以 `--pos-slot-duration-ms`、`--pos-ticks-per-slot`、`--pos-proposal-tick-phase`、`--pos-slot-clock-genesis-unix-ms` 与 `--pos-max-past-slot-lag` 接收参数，并在启动前验证非法组合。
- game/web/client launcher 只做同义参数的校验和透传；`chain_node_tick_ms` 仅表示 worker poll/fallback interval，绝不是 consensus slot duration。
- `/v1/chain/status` 的 consensus section 是当前只读状态面，至少保持 timing config、`proposal_tick_phase`、missed slot/tick accounting 与相应健康语义的可检索性。status read 不得推进 slot、续租或制造运行证据。

### 2.4 持久化、恢复与确定性

- restart 读取已持久化的 consensus/timing snapshot 后继续单调计数，不得把恢复前已观测的 slot/tick 倒退或重复计为新进度。
- replay 以已记录的事件、snapshot 和 canonical log 为准；它验证时间/状态后果，不重新根据“现在的 wall clock”补发历史提案。
- timing counters、missed accounting 和 status 所见配置属于恢复诊断输入。保存、checkpoint、retention 和 retained-height replay 的具体存储合同仍由 `runtime-storage-footprint-governance.*` 承接。

## 3. Technical Ownership and Interfaces

- 代码真值：`crates/oasis7/src/chain_pos_defaults.rs`、`crates/oasis7/src/bin/oasis7_chain_runtime/cli.rs`、`startup_reconcile.rs`、`status_payload.rs` 与共识执行路径。
- 本专题拥有 runtime 内部时间计算、状态推进、恢复和状态字段语义；参数值改变、validator/consensus 规则、ABI、网络/运维步骤或玩家承诺必须由相应 owner 经 TPM 联审。
- 本专题不把默认值复制为不可更新的文档常量；当前默认 profile 以 `config/chain-pos-defaults.env` 与其解析回归为准。

## 4. Acceptance and Validation

| PRD-ID | 验证 | 证明的合同 |
| --- | --- | --- |
| PRD-WORLD_RUNTIME-001 | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib chain_pos_defaults -- --nocapture` | 默认值解析和非法 duration/ticks/phase 拒绝。 |
| PRD-WORLD_RUNTIME-001 | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime oasis7_chain_runtime_tests -- --nocapture` | CLI 校验、控制面参数与 status 结构。 |
| PRD-WORLD_RUNTIME-001 | 定向共识 timing / startup reconcile 回归 | phase gate、strict-lag one-shot recovery、future/stale admission 与 restart counter 恢复。 |
| PRD-WORLD_RUNTIME-001 | `./scripts/doc-governance-check.sh && ./scripts/readme-link-check.sh` | 专业 authority、索引和链接完整。 |

这些验证不构成 mainnet、公开网络、运行时运维或发布放行结论。

## 5. Risks and Non-Goals

- 将 `chain_node_tick_ms` 写成 slot duration 会造成跨 launcher/control-plane 的时序漂移。
- 将 strict-lag recovery 写成 sticky retry 或 guard bypass 会损害确定性与安全顺序。
- 本专题不定义 slot 经济、奖励、validator 准入、签名方案、fee policy、外部节点 runbook 或 release verdict。
