# Runtime 版本级候选 soak 实测证据（2026-03-11）

审计轮次: 4

## Meta
- Evidence ID: `RT-VERSION-SOAK-20260311`
- Date: `2026-03-11`
- Owner Role: `runtime_engineer`
- Review Partner: `qa_engineer`
- Scope: `version candidate runtime soak`
- Conclusion: `ready`

## Evidence Binding
| Item | Source | Result | Notes |
| --- | --- | --- | --- |
| 实际 run 目录 | `.tmp/release_gate_p2p_dcg010/20260306-180215/` | `present` | 对应真实 `dry_run=false` 发布门禁长跑样本。 |
| 摘要文档 | `.tmp/release_gate_p2p_dcg010/20260306-180215/summary.md` | `pass` | `triad_distributed` 拓扑、`soak_release` profile、300s 长跑。 |
| 结构化指标 | `.tmp/release_gate_p2p_dcg010/20260306-180215/summary.json` | `pass` | `overall_status=ok`、`metric_gate.status=pass`。 |
| 历史 provenance | Git history / GitHub task evidence | `historical_only` | 该 run 曾以 `E5` 登记；历史 `pass` / `Go` 不构成当前 release authority。 |

## Key Runtime Signals
- `summary.json.dry_run = false`，说明该样本不是命令渲染或演练输出。
- `duration_secs_per_topology = 300`，满足当前版本级候选对“真实 soak summary / metrics”最小绑定要求。
- `topologies[0].metric_gate.status = pass`。
- `topologies[0].metrics.consensus_hash_consistent = true`。
- `topologies[0].metrics.consensus_hash_mismatch_count = 0`。
- `topologies[0].metrics.consensus_hash_missing_samples = 0`。
- `topologies[0].metrics.distfs_failure_ratio = 0`。
- `topologies[0].metrics.lag_p95 = 0`。
- `totals.report_samples_total = 213`，证明该样本有持续运行期采样，而非空跑。

## Interpretation
- 该样本来自当时的 `soak_release` 链路，可作为版本级历史 runtime soak provenance；当前 release 判断仍须服从 testing manual、现行 longrun claim tiers、正式 public-testnet readiness 与 fresh artifact。
- 当前版本级候选对 `runtime_soak` 的要求是“存在真实版本级 soak summary / metrics 绑定”，并未强制要求必须先获得新的 S10 五节点实跑结果。
- 因此，`doc/testing/longrun/s10-five-node-real-game-soak.prd.md` 继续保留为后续增强与扩大覆盖范围的专题目标，但不再构成本候选的唯一阻断。

## Decision
- 将版本级候选中的 `runtime_soak` 从 `blocked` 提升为 `ready`。
- 将版本级候选总状态从 `conditional` 提升为 `ready`。
- 后续若进入更高风险版本或需要更严格放行口径，再把 S10 五节点真实样本作为增量加强项，而不是回退当前候选结论。

## Validation
- `rg -n 'dry_run|overall_status|metric_gate|consensus_hash_consistent|consensus_hash_mismatch_count|report_samples_total' .tmp/release_gate_p2p_dcg010/20260306-180215/summary.json`
- `rg -n 'release_gate_p2p_dcg010|dry_run|overall_status|metric_gate' doc/world-runtime/evidence/runtime-version-candidate-{evidence,soak-evidence}-2026-03-11.md`
