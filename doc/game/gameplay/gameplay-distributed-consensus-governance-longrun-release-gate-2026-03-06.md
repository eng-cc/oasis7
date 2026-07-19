# Gameplay Distributed Consensus Governance Long-Run 发布门禁报告与回滚预案（2026-03-06）

审计轮次: 3

- 产品边界：`doc/product/world-infrastructure/world-continuity-governance-and-recovery.prd.md`
- 专业权威：`doc/world-runtime/prd.md`、`doc/p2p/prd.md`、`doc/testing/prd.md`
- 对应任务: `TASK-GAME-DCG-010`
- 结论时间: 2026-03-06

## 1. 发布门禁结论
- Gate Result: `pass`（当前证据满足 `test_tier_required` + `S9 release baseline`）。
- 放行建议: `Go`（可进入下一阶段灰度/候选发布）；生产全量放行前继续执行 7 天与 30 天长窗作业。
- 阻断项: 无。

## 2. 门禁范围（PRD-GAME-005）
- 执行共识层: tick 证书链一致性、回放一致性与快照恢复。
- 治理共识层: `timelock + epoch` 生效门禁、紧急刹车/否决。
- 身份与反女巫: 身份快照权重、惩罚-申诉-复核闭环。
- 长稳与故障注入: P2P/存储/共识在线长跑（含 `chaos`）与共识哈希一致性门禁。

## 3. 证据与结果
| 证据ID | 命令/来源 | 结果 | 关键结论 | 产物 |
| --- | --- | --- | --- | --- |
| E1 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::governance:: -- --nocapture` | pass | 治理门禁 + 身份惩罚申诉回归通过 | 本地测试日志 |
| E2 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture` | pass | 快照投票与惩罚后投票权恢复闭环通过 | 本地测试日志 |
| E3 | `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime::tests::persistence:: -- --nocapture` + `runtime::tests::audit::` | pass | 快照恢复、审计事件过滤回归通过 | 本地测试日志 |
| E4 | `./scripts/p2p-longrun-soak.sh --profile soak_smoke --topologies triad --duration-secs 90 --no-prewarm --chaos-continuous-enable --chaos-continuous-interval-secs 20 --chaos-continuous-start-sec 20 --chaos-continuous-max-events 2 --chaos-continuous-actions restart,pause --out-dir .tmp/p2p_longrun_dcg009` | pass | 新增共识哈希一致性门禁在短窗 chaos 下可用 | `.tmp/p2p_longrun_dcg009/20260306-175501/` |
| E5 | `./scripts/p2p-longrun-soak.sh --profile soak_release --topologies triad_distributed --duration-secs 300 --no-prewarm --max-stall-secs 240 --max-lag-p95 50 --max-distfs-failure-ratio 0.1 --chaos-continuous-enable --chaos-continuous-interval-secs 30 --chaos-continuous-start-sec 30 --chaos-continuous-max-events 8 --chaos-continuous-actions restart,pause --chaos-continuous-seed 1772284566 --chaos-continuous-restart-down-secs 1 --chaos-continuous-pause-duration-secs 2 --out-dir .tmp/release_gate_p2p_dcg010` | pass | `overall_status=ok`、`metric_gate=pass`、`consensus_hash_consistent=true`、`consensus_hash_mismatch_count=0` | `.tmp/release_gate_p2p_dcg010/20260306-180215/` |

### 3.1 S9 Release Baseline 摘要（E5）
- `summary.json.overall_status = "ok"`
- `totals.topology_failed_count = 0`
- `topologies[0].metric_gate.status = "pass"`
- `topologies[0].metrics.consensus_hash_consistent = true`
- `topologies[0].metrics.consensus_hash_mismatch_count = 0`
- `topologies[0].metrics.consensus_hash_missing_samples = 0`
- `topologies[0].metrics.consensus_hash_mismatch_file = .tmp/release_gate_p2p_dcg010/20260306-180215/triad_distributed/.consensus_hash_mismatch.tsv`

### 3.2 告警解释（非阻断）
- `running_false_samples/http_failure_samples`：由计划内 `restart/pause` chaos 注入导致，属于预期噪声。
- `committed_height_not_monotonic nodes=sequencer`：重启窗口导致单节点局部高度回落告警，未出现共识哈希分歧，`consensus_hash_consistent=true`。
- `settlement_apply_attempts_zero`：当前场景未启用相关结算流量，不影响本专题门禁判定。

## 4. 回滚预案（Runbook）

### 4.1 触发条件（任一满足即触发）
- `summary.json.overall_status != "ok"` 或 `topology_failed_count > 0`。
- `metric_gate.status = fail` 且失败原因包含 `consensus_hash_divergence`。
- `consensus_hash_mismatch_count > 0`。
- 线上出现治理事件回放错误、快照恢复失败或状态根不一致。

### 4.2 目标
- RTO: 30 分钟内完成回退并恢复服务。
- RPO: 以最近稳定快照点为准（不做跨快照重写）。

### 4.3 执行步骤
1. 进入紧急状态：
   - 启动治理紧急刹车（按宪章 guardian 阈值提交紧急治理事件）。
   - 暂停新提案生效与高风险经济动作。
2. 现场保全：
   - 归档当前 run 目录与节点日志（`summary.json/timeline.csv/chaos_events.log/nodes/*`）。
   - 备份当前世界目录与快照目录。
3. 回退到稳定版本：
   - 部署上一稳定发行二进制（建议固定到发布清单中的稳定 commit/tag）。
   - 以“最近稳定快照 + 对应 journal”恢复世界状态。
4. 恢复验证（必须通过）：
   - 运行 `soak_smoke`（建议 `triad` + 最小 chaos）验证 `overall_status=ok`。
   - 校验 `consensus_hash_consistent=true` 且 `consensus_hash_mismatch_count=0`。
5. 解除紧急状态：
   - 经治理确认后解除紧急刹车，恢复常规治理与经济流量。

### 4.4 回滚后补救
- 在 24 小时内提交故障复盘：触发条件、影响面、根因、修复与防复发项。
- 将复盘结论回写到专题 PRD 与项目文档，并补充对应自动化门禁。

## 5. 后续动作
- 继续执行 `TASK-GAME-DCG-009` 的长窗计划：
  - 7 天 endurance（含 chaos + feedback）。
  - 30 天 soak（含故障注入策略轮换）。
- 每轮长窗结果按本报告模板增量归档，作为生产全量放行前置条件。
