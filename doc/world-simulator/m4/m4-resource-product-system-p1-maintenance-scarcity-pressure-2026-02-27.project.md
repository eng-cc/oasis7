# M4 资源与产品系统 P1：维护压力与本地稀缺供给延迟（项目管理文档）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.design.md`
- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0：输出 P1 设计文档与项目管理文档。
- [x] T1：代码接线（负载折旧 + world fallback 供给延迟）。
- [x] T2：补齐 `test_tier_required` 单测并执行回归。
- [x] T3：回写文档状态与 devlog，完成收口。

## 依赖
- doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md
- `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md`
- `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md`
- `doc/world-simulator/m4/material-multi-ledger-logistics.prd.md`
- `crates/oasis7/src/runtime/world/economy.rs`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_economy.rs`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T0~T3）。
- 阻塞项：无。
- 下一步：无（P1 范围内事项已收口）。
