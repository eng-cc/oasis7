# Runtime 版本级候选联合证据（2026-03-11）

审计轮次: 4

## Meta
- Evidence ID: `RT-VERSION-CANDIDATE-20260311`
- Date: `2026-03-11`
- Owner Role: `runtime_engineer`
- Scope: `version candidate runtime footprint / GC / soak`
- Historical Window Conclusion: `ready`（仅限 2026-03-11 绑定输入；不是当前 candidate / release readiness）

## Slot Summary
| Slot | Status | Evidence Path | Conclusion |
| --- | --- | --- | --- |
| `runtime_footprint` | `historical_ready` | `doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md` | 当轮真实 `release_default` 样本与 gate 摘要填充了 footprint 槽位。 |
| `runtime_gc` | `historical_ready` | `doc/world-runtime/evidence/runtime-sidecar-orphan-gc-failsafe-2026-03-11.md` | 当轮证明 sidecar orphan 为窗口态且可在后续 save/GC 后收敛。 |
| `runtime_soak` | `historical_ready` | `doc/world-runtime/evidence/runtime-version-candidate-soak-evidence-2026-03-11.md` | 当轮绑定真实 `soak_release` 长跑 summary / metrics。 |

## Footprint Evidence
- 证据入口：`doc/world-runtime/evidence/runtime-storage-gate-sample-2026-03-10.md`
- 可采信结论：
  - `release_default` profile 下真实 runtime 样本已通过 storage gate 接线。
  - QA 复验已确认 `<64` 不提前出现 checkpoint、`65` 时出现首个 checkpoint。
  - 该证据足以把版本级 `runtime_footprint` 从 `watch` 提升到 `ready`。

## GC Evidence
- 证据入口：`doc/world-runtime/evidence/runtime-sidecar-orphan-gc-failsafe-2026-03-11.md`
- 可采信结论：
  - sidecar orphan 并非稳定泄漏，而是 save/GC 时序窗口信号。
  - 自动化测试已证明下一次成功 `save_to_dir()` 后 `orphan_blob_count` 可收敛到 `0`。
  - 该证据足以把版本级 `runtime_gc` 从 `watch` 提升到 `ready`。

## Soak Evidence
- 证据入口：`doc/world-runtime/evidence/runtime-version-candidate-soak-evidence-2026-03-11.md`
- 可采信结论：
  - 已绑定 `.tmp/release_gate_p2p_dcg010/20260306-180215/summary.json` 的真实 `dry_run=false` 样本。
  - 历史 gameplay release-gate 曾把同一 run 登记为 `E5`；该专题已退役，当前只把 run ID 与原始 summary/metrics 作为历史 provenance，不作为现行放行结论。
  - 该证据足以把版本级 `runtime_soak` 从 `blocked` 提升到 `ready`。

## Overall Interpretation
- runtime 在版本级候选上已从“只有 task 级边界验收”提升到“footprint + GC + soak 三槽位均有真实可引用证据”。
- 因此 runtime 联合证据在该历史窗口的结论为 `ready`；当前结论必须由新同代码、profile 与输入窗口证据重签。
