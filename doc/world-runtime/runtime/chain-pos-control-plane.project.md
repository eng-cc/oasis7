# oasis7 Runtime：链 PoS 时间与控制面合同（项目管理）

审计轮次: 1

- 对应需求文档：`doc/world-runtime/runtime/chain-pos-control-plane.prd.md`
- 对应设计文档：`doc/world-runtime/runtime/chain-pos-control-plane.design.md`

## 任务拆解

- [x] p2p-node-time-control-plane-authority-consolidation (PRD-WORLD_RUNTIME-001) [test_tier_required]: 建立唯一 world-runtime PoS 时间/控制面 authority，吸收 slot clock、subslot pacing 与 time-anchor 当前合同；实现过程和已关闭历史任务仅由 Git history 与 GitHub task evidence 追溯。 Trace: #2682 (task_172abebb99354d4fad395aa05a581193)

## 依赖

- 当前代码合同：`crates/oasis7/src/chain_pos_defaults.rs` 与 `crates/oasis7/src/bin/oasis7_chain_runtime/` 的 CLI、status、startup reconcile 路径。
- 持久化/retention 细则：`runtime-storage-footprint-governance.*`。

## 当前合同清单

- 参数：`slot_clock_genesis_unix_ms`、`slot_duration_ms`、`ticks_per_slot`、`proposal_tick_phase`、`max_past_slot_lag`。
- 状态：`logical_tick`、slot/epoch、`missed_slot_count`、`missed_tick_count` 与只读 status consensus snapshot。
- 恢复：restart 恢复 counters/config；replay 不因当前 wall clock 重建被跳过的 slot。
- 非目标：不吸收 triad operator authority、Token 授权/交易目标态、validator economics、custody 或 release claims。

## 验收命令

- `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib chain_pos_defaults -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime oasis7_chain_runtime_tests -- --nocapture`
- 对 timing/reconcile 改动，追加 phase gating、strict-lag recovery、future/stale admission 和 restart/replay 定向回归。
- `./scripts/doc-governance-check.sh`
- `./scripts/readme-link-check.sh`
- `git diff --check`

## 状态

本专题是当前稳定 professional authority。它不表示任何历史源文件已经可删除：删除必须在完整吸收、全部 inbound-reference repair 和相应专业验证之后，由当前任务的集成路径另行确认。
