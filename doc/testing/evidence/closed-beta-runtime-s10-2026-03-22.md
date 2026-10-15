# Close Beta Runtime Evidence (2026-03-22)

历史记录边界：以下判定、命令、角色交接及“当前/下一步”均属于文中明确的采集窗口；保留原始失败、通过、未执行范围和审计定位，不作为当前规范、进度或持续跟踪义务。当前行动与验收回到[测试入口](../README.md)及对应专业 authority，使用新的候选和证据窗口；后续通过、合入或任务关闭不改写本次运行事实。

审计轮次: 5

## Meta
- 责任角色: `runtime_engineer`
- 目标: 为 `closed_beta_candidate` 场景收集 runtime 侧 five-node no-LLM soak / replay/rollback / release gate 证据，并确认脚本默认链路是否具备可重复执行的清理闭环。
- 主执行命令: `timeout 130 ./scripts/s10-five-node-game-soak.sh --duration-secs 120 --out-dir output/longrun/closed-beta-20260322 --no-prewarm --no-llm`
- 诊断复跑 1: `timeout 80 ./scripts/s10-five-node-game-soak.sh --duration-secs 60 --out-dir output/longrun/closed-beta-repro-20260322 --no-prewarm --no-llm`
- 诊断复跑 2: `timeout 140 ./scripts/s10-five-node-game-soak.sh --duration-secs 120 --out-dir output/longrun/closed-beta-repro-20260322 --no-prewarm --no-llm`
- cleanup 验证: `./scripts/s10-five-node-game-soak.sh --duration-secs 10 --base-port 6410 --out-dir output/longrun/closed-beta-cleanup-check-20260322 --no-prewarm --no-llm`
- clean-room 候选复验: `./scripts/s10-five-node-game-soak.sh --duration-secs 120 --base-port 6310 --out-dir output/longrun/closed-beta-rerun-fixed-20260322 --no-prewarm --no-llm`
- candidate 长跑: `./scripts/s10-five-node-game-soak.sh --duration-secs 600 --base-port 6510 --out-dir output/longrun/closed-beta-candidate-20260322 --no-prewarm --no-llm`
- replay/rollback drill 1: `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::basic::from_snapshot_replay_rebuilds_missing_tick_consensus_records -- --nocapture`
- replay/rollback drill 2: `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_required runtime::tests::persistence::rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift -- --nocapture`
- 当前结果: `pass`
- 关键产物:
  - 失败样本 A: `output/longrun/closed-beta-20260322/20260322-113809/{summary.md,summary.json,timeline.csv,nodes/*,failures.md}`
  - 通过样本 B: `output/longrun/closed-beta-repro-20260322/20260322-115646/{summary.md,summary.json,timeline.csv,nodes/*}`
  - 失败样本 C: `output/longrun/closed-beta-repro-20260322/20260322-115830/{summary.md,summary.json,nodes/s10-sequencer/stderr.log,failures.md}`
  - cleanup 验证 D/E: `output/longrun/closed-beta-cleanup-check-20260322/{20260322-120244,20260322-120255}/`
  - 通过样本 F: `output/longrun/closed-beta-rerun-fixed-20260322/20260322-120458/{summary.md,summary.json,timeline.csv,nodes/*}`
  - 通过样本 G: `output/longrun/closed-beta-candidate-20260322/20260322-121320/{summary.md,summary.json,timeline.csv,nodes/*}`

## 结论
- `TASK-GAME-029` 已完成。runtime lane 的正式真值现在是：cleanup 后的 clean-room `120s` 样本、`600s+` 候选样本和 replay/rollback required-tier drill 全部通过，可以把 runtime 证据包交给 `qa_engineer` 并入 unified gate。
- 证据分层如下：
  - 失败样本 A `output/longrun/closed-beta-20260322/20260322-113809` 在 42 秒左右出现 `process_exit`，`metric_gate=fail`；但 sequencer 备份目录里实际已生成 `reward-runtime-report/epoch-0.json` 等产物，说明最初的 `minted_records_empty` / `reward_runtime_metrics_not_ready` 不是根因，而是进程退出后的次生告警。
  - 通过样本 B `output/longrun/closed-beta-repro-20260322/20260322-115646` 跑满 60 秒，`process_status=ok`、`metric_gate=pass`、`minted_non_empty_samples=5`、`reward_runtime_available_samples=5`，证明 five-node no-LLM 链路本身可跑通。
  - 失败样本 C `output/longrun/closed-beta-repro-20260322/20260322-115830` 在 startup 阶段直接报 `bind 127.0.0.1:5811 failed: Address already in use`，将问题进一步收敛到脚本 cleanup。
  - 已修复 `scripts/s10-five-node-game-soak.sh`，增加针对当前 run 的 `node-id + status-bind + node-gossip-bind` 精确 cleanup 与端口释放等待；cleanup 验证 D/E 在 `--base-port 6410` 下实现两轮 back-to-back 启动/收尾，无 `Address already in use`、无 startup fail、无 lingering process。
  - 通过样本 F `output/longrun/closed-beta-rerun-fixed-20260322/20260322-120458` 在 clean-room 端口组 `6310` 下跑满 120 秒，`process_status=ok`、`metric_gate=pass`、`minted_non_empty_samples=120`、`reward_runtime_available_samples=120`、`settlement_apply_attempts=2`，且 `nodes/*/exit-status.txt` 全部是脚本收尾阶段写入的 `exit_status=143/signal=15`，证明当前退出签名属于受控 `SIGTERM` 停机而不是 runtime crash。
  - 通过样本 G `output/longrun/closed-beta-candidate-20260322/20260322-121320` 在 clean-room 端口组 `6510` 下跑满 600 秒，`process_status=ok`、`metric_gate=pass`、`minted_non_empty_samples=1035`、`reward_runtime_available_samples=1035`、`settlement_apply_attempts=10`、`committed_height_monotonic=true`，且 `nodes/*/exit-status.txt` 仍全部是受控 `SIGTERM` 收尾。
  - replay/rollback drill 两条 required-tier 测试均返回 `rc=0`，分别覆盖 `from_snapshot replay rebuilds missing tick consensus records` 与 `rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift`。
- 因此，runtime lane 的正式缺口已经关闭；当前剩余阻断只存在于 unified gate 的其他 lane，而不是 runtime 自身。

## 门禁与指标
- 失败样本 A `20260322-113809`:
  - `metric_gate=fail`
  - `running_false_samples=5`
  - `minted_non_empty_samples=0`
  - `reward_runtime_available_samples=0`
  - `committed_height_monotonic=false`
- 通过样本 B `20260322-115646`:
  - `metric_gate=pass`
  - `status_samples_ok=120`
  - `balances_samples_ok=120`
  - `minted_non_empty_samples=5`
  - `settlement_positive_samples=1`
  - `reward_runtime_available_samples=5`
  - `committed_height_monotonic=true`
- 失败样本 C `20260322-115830`:
  - `startup_failed`
  - `metric_gate=fail`
  - sequencer stderr: `bind 127.0.0.1:5811 failed: Address already in use`
- cleanup 验证 D/E:
  - 两轮都 `process_status=ok`
  - 两轮结束后 `ps -ef | rg 'oasis7_chain_runtime --node-id s10-.*(641|643)'` 无残留
- 通过样本 F `20260322-120458`:
  - `process_status=ok`
  - `metric_gate=pass`
  - `minted_non_empty_samples=120`
  - `settlement_positive_samples=24`
  - `reward_runtime_available_samples=120`
  - `settlement_apply_attempts=2`
  - `committed_height_monotonic=true`
- 通过样本 G `20260322-121320`:
  - `process_status=ok`
  - `metric_gate=pass`
  - `minted_non_empty_samples=1035`
  - `settlement_positive_samples=207`
  - `reward_runtime_available_samples=1035`
  - `settlement_apply_attempts=10`
  - `committed_height_monotonic=true`
- replay/rollback drill:
  - `runtime::tests::basic::from_snapshot_replay_rebuilds_missing_tick_consensus_records`: `pass`
  - `runtime::tests::persistence::rollback_with_reconciliation_recovers_from_detected_tick_consensus_drift`: `pass`
- 说明:
  - `node_state_backup/*` 是脚本启动前通过 `isolate_node_state_dirs()` 挪走的旧状态，但失败样本 A 的 sequencer 备份目录确实证明 reward runtime 在退出前已经生成过有效产物，因此原文档里“奖励/结算证据完全不存在”的表述需要废弃。
  - cleanup 验证 D/E 是为脚本修复服务的短诊断样本；样本 F/G 与 replay/rollback drill 才构成当前 runtime lane 的正式候选证据包。

## 交接
- 当前 runtime lane 交接结论:
  1. clean-room `600s+` soak 已 `pass`，并附带可追踪的 `exit-status.txt` 收尾签名。
  2. replay/rollback required-tier drill 已 `pass`。
  3. runtime lane 可以交给 `qa_engineer` 并入 unified gate，但项目整体阶段仍需等待 viewer、pure API、no-UI smoke 与 trend baseline 一起收口。

## 下一步
1. 将样本 G 与两条 replay/rollback drill 结果回填到 unified QA gate，完成 `TASK-GAME-031` 的 runtime lane 同步。
2. 推动 headed Web/UI、pure API、no-UI smoke 在同一 candidate 上补跑 fresh sample。
3. 继续保持项目阶段为 `internal_playable_alpha_late`，直到 unified gate 的其他阻断项关闭。
