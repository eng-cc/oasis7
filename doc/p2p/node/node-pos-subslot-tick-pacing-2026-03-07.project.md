# oasis7 Runtime：PoS 槽内 Tick 相位门控与自适应节拍（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.design.md`
- 对应需求文档: `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-P2P-009-T0 (PRD-P2P-NODE-TICK-001/002/003) [test_tier_required]: 完成专题 PRD 与项目管理建档，并回写 `doc/p2p/prd.md` / `doc/p2p/project.md` / `doc/p2p/prd.index.md`。
- [x] TASK-P2P-009-T1 (PRD-P2P-NODE-TICK-001/002) [test_tier_required]: 在 `NodePosConfig/PosNodeEngine` 引入 `ticks_per_slot`、`proposal_tick_phase`、logical tick 观测与提案相位门控，并补齐状态快照持久化字段。
- [x] TASK-P2P-009-T2 (PRD-P2P-NODE-TICK-003) [test_tier_required]: 在 `NodeRuntime` 引入自适应 tick 调度等待计算（动态单 tick 时长），并补齐 runtime 单元测试。
- [x] TASK-P2P-009-T3 (PRD-P2P-NODE-TICK-001/002/003) [test_tier_required + test_tier_full]: 补齐定向 required/full 回归，回写专题与模块项目文档并沉淀 devlog 证据。
- [ ] p2p-testnet-clean-rebuild-liveness-corrective (PRD-P2P-NODE-TICK-002) [test_tier_required]: 落地 issue #2264 冻结头评审的纠正项；直接回归必须证明 pending proposal 与 replication successor probe guard 均会消耗 tick-local 恢复边沿，并在新 head 上重跑 CI 与 involved-role review。 Trace: #2264 (task_813a3014b1124abfb09f01e50a4597a4)

## 依赖
- `doc/p2p/node/node-pos-subslot-tick-pacing-2026-03-07.prd.md`
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/lib_impl_part1.rs`
- `crates/oasis7_node/src/pos_state_store.rs`
- `crates/oasis7_node/src/runtime_util.rs`
- `testing-manual.md`

## 状态
- 更新日期: 2026-07-14
- 当前状态: corrective task in progress
- 下一任务: `task_813a3014b1124abfb09f01e50a4597a4`
- 阻塞项: 无
- 进展: `TASK-P2P-009-T0~T3` 历史实现保持完成；当前纠正任务已补齐 pending proposal 与 replication successor probe guard 消耗 tick-local 恢复边沿的直接回归，并纠正 PRD Executive Summary 表述；待同任务其余已接受 finding 集成后，在新 head 上重跑 CI 与 involved-role review。
- 说明: 本文档仅维护执行计划与任务状态；实施过程记录写入 `doc/devlog/README.md`。
