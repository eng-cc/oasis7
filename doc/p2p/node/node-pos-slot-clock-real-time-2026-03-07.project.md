# oasis7 Runtime：PoS 固定时间槽（Slot/Epoch）真实时钟驱动（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.design.md`
- 对应需求文档: `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-P2P-008-T0 (PRD-P2P-NODE-CLOCK-001/002/003) [test_tier_required]: 完成专题 PRD 与项目管理建档，并回写 `doc/p2p/prd.md` / `doc/p2p/project.md` / `doc/p2p/prd.index.md`。
- [x] TASK-P2P-008-T1 (PRD-P2P-NODE-CLOCK-001) [test_tier_required]: 在 `NodePosConfig/PosNodeEngine` 引入 wall-clock slot 计算、漏槽对齐与提案门控。
- [x] TASK-P2P-008-T2 (PRD-P2P-NODE-CLOCK-002/003) [test_tier_required]: 增加时间窗口校验与快照可观测字段（`last_observed_slot`、`missed_slot_count`、`max_past_slot_lag`、`inbound_rejected_*`）。产出：proposal/attestation 未来槽与过旧槽拒绝、attestation target epoch 映射校验、拒绝原因快照可观测、定向回归通过。
- [x] TASK-P2P-008-T3 (PRD-P2P-NODE-CLOCK-001/002/003) [test_tier_required + test_tier_full]: 补齐单元/跨节点回归，回写文档与 devlog 收口。产出：入站窗口拒绝单元回归 + gossip/network 跨节点路径定向回归通过。

## 依赖
- `doc/p2p/node/node-pos-slot-clock-real-time-2026-03-07.prd.md`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/lib_impl_part1.rs`
- `crates/oasis7_node/src/lib_impl_part2.rs`
- `crates/oasis7_node/src/pos_state_store.rs`
- `testing-manual.md`

## 状态
- 更新日期: 2026-03-07
- 当前状态: done
- 下一任务: 无
- 阻塞项: 无
- 进展: `TASK-P2P-008-T0~T3` 已全部完成；已完成 wall-clock slot/epoch 驱动、漏槽对齐、入站 proposal/attestation 时间窗口校验（未来槽/过旧槽）、attestation epoch 映射门禁与拒绝原因快照可观测，并通过单元与跨节点定向回归。
- 说明: 本文档仅维护执行计划与任务状态；实施过程记录写入 `doc/devlog/README.md`。
