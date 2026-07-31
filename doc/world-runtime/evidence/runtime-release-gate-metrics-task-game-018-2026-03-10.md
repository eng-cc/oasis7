# Runtime 发布门禁指标记录（TASK-GAME-018 / 2026-03-10）

审计轮次: 4

## Meta
- 发布候选 / 版本: `TASK-GAME-018 / ROUND-009`
- 指标记录 ID: `RT-GATE-GAME-018-20260310`
- 日期: `2026-03-10`
- 负责人: `runtime_engineer`
- 关联 PRD-ID: `PRD-WORLD_RUNTIME-001/002/003` / `PRD-WORLD_RUNTIME-014/015`
- 关联任务: `TASK-WORLD_RUNTIME-002/003/004/033`
- 关联边界清单: `doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md`
- 关联回归模板: `doc/world-runtime/templates/runtime-release-gate-metrics-template.md` / `doc/world-runtime/templates/runtime-security-numeric-regression-template.md`
- runtime 结论: `go`

## 关键指标
| 指标 | 说明 | 来源 | 当前状态 (`pass` / `fail` / `blocked`) | 证据路径 | 备注 |
| --- | --- | --- | --- | --- | --- |
| replay / state root 一致性 | 确定性回放与恢复后状态根一致 | `required` | `pass` | `env -u RUSTC_WRAPPER cargo test -p oasis7 from_snapshot_replay_rebuilds_missing_tick_consensus_records -- --nocapture` | 2026-03-10 本轮实测通过。 |
| WASM ABI / hash / registry | 工件、接口、registry 可追溯且无漂移 | `full` | `pass` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full --test module_release_sla_triad -- --nocapture` | 覆盖 wasm hash、artifact identity、submit->shadow->approve->apply 链路。 |
| 治理状态机 / 拒绝路径 | propose/shadow/approve/apply 与拒绝原因完整 | `required` | `pass` | `env -u RUSTC_WRAPPER cargo test -p oasis7 longrun_operability_release_gate_blocks_stage_and_economy_pressure -- --nocapture` | 覆盖 stage / rollback drill / economy gate 的阻断原因。 |
| 安全失败签名 | 越权、缺审计、receipt 断裂等失败签名为 0 | `required` | `pass` | `doc/world-runtime/checklists/runtime-core-boundary-acceptance-checklist.md` / `doc/world-runtime/templates/runtime-security-numeric-regression-template.md` | 本轮定向回归未见新增安全失败签名。 |
| 数值语义失败签名 | 数值漂移、边界值异常、恢复后不一致等失败签名为 0 | `required` | `pass` | `env -u RUSTC_WRAPPER cargo test -p oasis7 snapshot_retention_policy_prunes_old_entries -- --nocapture` | 保留/裁剪后语义保持一致，本轮无异常信号。 |
| storage / GC / replay summary | profile、GC 结果、恢复摘要满足当前候选要求 | `required` | `pass` | `env -u RUSTC_WRAPPER cargo test -p oasis7 storage_footprint_fixture_baseline_covers_2500_ticks -- --nocapture` / `doc/world-runtime/runtime/runtime-storage-footprint-governance.prd.md` / `.tmp/s10_longrun_t2/20260308-113318/summary.md` | T7.1 基线已覆盖 2500 ticks；现有 S10 摘要仅为 dry-run，不作为阻断失败。 |

## 风险与例外
| 风险 ID | 描述 | 是否阻断 | 缓解措施 | 负责人 | 复审时间 |
| --- | --- | --- | --- | --- | --- |
| `R-RUNTIME-033-001` | `TASK-WORLD_RUNTIME-033` 的 T7.2~T7.5 仍未全部完成 | `no` | 当前先以 task-game-018 候选所需最小 runtime P0 证据放行；更大 footprint/GC/soak 联合验证继续按原专题推进 | `runtime_engineer` | `下一轮 runtime 候选` |
| `R-RUNTIME-033-002` | 现有 `.tmp/s10_longrun_t2/t3` 摘要为 dry_run | `no` | 后续真正发布候选仍需补真实 soak summary，不影响本轮 task 级 P0 收口 | `runtime_engineer` | `下一轮 runtime 候选` |

## 结论摘要
- 通过项：回放恢复、snapshot retention、storage footprint baseline、WASM/hash/registry、治理阻断路径均有本轮或可追溯证据。
- 阻断项：无 task 级 P0 阻断。
- 条件放行项：更大范围 footprint/GC/soak 联合验证仍在 `TASK-WORLD_RUNTIME-033` 后续切片中推进。
- 历史处置：本记录曾作为 runtime P0 从 `blocked` 转为 `ready` 的依据；Git history 只保留当时裁决，不接受状态回写，当前状态以 GitHub task issue evidence 和现行 runtime PRD/design 为准。
