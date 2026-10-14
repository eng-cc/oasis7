# Runtime Storage Gate 实测样本（2026-03-10）

历史窗口：正文的 T7.2 阻塞诊断、派工和 T7.3～T7.5 交接均发生于 2026-03-10～11，需按先失败、后修复复验的顺序读。原 Task 未唯一定位，`.tmp` 原始输出的长期可取回性未核实，源内容完整保留；不表示今日仍阻塞、重新派工或当前 release approval。

审计轮次: 4

## Meta
- 样本 ID: `RT-STORAGE-GATE-SAMPLE-20260310-234544`
- 日期: `2026-03-10`
- 执行角色: `runtime_engineer`
- 关联任务: `TASK-WORLD_RUNTIME-033 / T7.2`
- 关联脚本: `scripts/oasis7-runtime-storage-gate.sh`
- 总结论: `pass_after_fix`

## 输入
- 运行命令: `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_chain_runtime -- --node-id t72-node --world-id t72-world --storage-profile release_default --status-bind 127.0.0.1:5221 --execution-world-dir .tmp/runtime_t72_sample/execution-world --execution-records-dir .tmp/runtime_t72_sample/execution-records --storage-root .tmp/runtime_t72_sample/storage-root --execution-bridge-state .tmp/runtime_t72_sample/execution-bridge-state.json`
- 状态样本: `.tmp/runtime_t72_sample/artifacts/status.json`、`.tmp/runtime_t72_sample/artifacts/status-after-feedback.json`、`.tmp/runtime_t72_probe_20260310-235637/artifacts/status-round-1.json`
- gate 摘要: `.tmp/world_runtime_storage_gate/20260310-234544/summary.md`（fresh sample） / `.tmp/world_runtime_storage_gate/20260310-234829/summary.md`（feedback 注入后）

## 关键观察
- `storage_profile=release_default`
- `effective_budget.profile=release_default`
- 初始真实样本 `checkpoint_count=0`、`replay_summary.mode=full_log_only`。
- feedback 注入后样本一度推进到 `latest_retained_height=16`，仍未生成 checkpoint。
- 后续扩展实测把 retained height 推到 `32+` 后，`checkpoint_count=1` 已出现，说明 checkpoint 生成链路本身可工作。
- `feedback_submit` 存在 `10 / 60s` 限流；错误签名为 `DistributedValidationFailed(rate limit exceeded ...)`。
- `orphan_blob_count=0`、`last_gc_result=success`、`degraded_reason=null`。

<a id="当前结论"></a>
## 修复前的历史诊断
- 本轮真实 `oasis7_chain_runtime` 样本已成功接入 `scripts/oasis7-runtime-storage-gate.sh`。
- 先前把失败简单解释为“`release_default` 样本尚未达到 `64`”是不充分的；扩展实测显示 `checkpoint_count` 会在 `height=32` 左右出现。
- 根因已收敛为：`/v1/chain/status.storage.effective_budget` 报告的是 `release_default.execution_checkpoint_interval=64`，但 `oasis7_chain_runtime` 实际执行桥接仍在使用硬编码 `32/4` 默认值。
- 因此 T7.2 当前阻塞不再是“需要把样本推进到 64”，而是“需要把 execution bridge retention/checkpoint cadence 真正绑定到 storage profile 后再复跑 gate”。

<a id="后续动作"></a>
## 修复前的历史派工
- 由 `runtime_engineer` 修复 execution bridge 对 `StorageProfileConfig` 的实际透传，使 hot window / checkpoint interval / checkpoint keep 与 status budget 一致。
- 由 `qa_engineer` 在修复后复跑真实 runtime 样本与 `scripts/oasis7-runtime-storage-gate.sh`，确认 `release_default` 在 `<64` 不再提前出现 checkpoint，且 `height=64` 时生成首个 checkpoint。

## 根因判断
- `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs` 中 `maybe_persist_execution_checkpoint_for_record(...)` 明确要求 `record.height % checkpoint_interval_heights == 0` 才会落盘 checkpoint。
- 读码与扩展实测共同证明：生产 `oasis7_chain_runtime` 的 execution bridge 没有吃到 `StorageProfileConfig`，仍落回硬编码 `EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS=32`、`EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS=32`、`EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST=4`。
- 这解释了为何 status `effective_budget` 声称 `release_default.execution_checkpoint_interval=64`，但真实样本在 `height=32` 左右已经出现 checkpoint。
- 同时确认不存在可外部触发的显式 `save/flush/checkpoint` API；可推进 commit 的公开入口仍只有 `POST /v1/chain/feedback/submit` 与 `POST /v1/chain/transfer/submit`。


## QA 复验（修复后）
- 执行角色: `qa_engineer`
- 真实样本命令: `env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_chain_runtime -- --node-id t72-qa-posfast --world-id t72-world --node-tick-ms 5 --pos-slot-duration-ms 20 --pos-ticks-per-slot 1 --pos-proposal-tick-phase 0 --storage-profile release_default --status-bind 127.0.0.1:5226 --execution-world-dir .tmp/runtime_t72_qa_posfast_20260311-002402/execution-world --execution-records-dir .tmp/runtime_t72_qa_posfast_20260311-002402/execution-records --storage-root .tmp/runtime_t72_qa_posfast_20260311-002402/storage-root --execution-bridge-state .tmp/runtime_t72_qa_posfast_20260311-002402/execution-bridge-state.json`
- `pre64` 状态样本: `.tmp/runtime_t72_qa_posfast_20260311-002402/artifacts/status-pre64.json`
- `post64` 状态样本: `.tmp/runtime_t72_qa_posfast_20260311-002402/artifacts/status-post64.json`
- `pre64` gate 摘要: `.tmp/runtime_t72_qa_posfast_20260311-002402/gate-pre64/20260311-002427/summary.md`
- `post64` gate 摘要: `.tmp/runtime_t72_qa_posfast_20260311-002402/gate-post64/20260311-002427/summary.md`

### 复验结果
- `latest_retained_height=47` 时，`checkpoint_count=0`，`replay_summary.mode=full_log_only`，符合“`<64` 不提前出现 checkpoint”的预期。
- `latest_retained_height=65` 时，`checkpoint_count=1`，`replay_summary.mode=checkpoint_plus_log`，符合“达到 `64` 后生成首个 checkpoint”的预期。
- `pre64` gate 仍显示 `FAIL`，原因是 `checkpoint_count=0`（预期失败）且该采样点观察到一次瞬时 `orphan_blob_count=1`；继续推进到 `65` 后，`orphan_blob_count` 收敛为 `0`，gate 转为 `PASS`。

### QA 结论
- `runtime_engineer` 的 profile 透传修复已生效：真实 `release_default` 样本的 execution bridge cadence 与 status `effective_budget.execution_checkpoint_interval=64` 对齐。
- `TASK-WORLD_RUNTIME-033 / T7.2` 的“真实 runtime storage gate 接线 + cadence 一致性验证”可视为完成，后续应继续推进 T7.3~T7.5。
