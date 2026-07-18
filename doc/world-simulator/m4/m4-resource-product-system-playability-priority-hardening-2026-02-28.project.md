# M4 资源产业链可玩性优先强化（项目管理文档）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.design.md`
- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0：完成设计文档与项目管理文档建档。
- [x] T1：补齐 `polymer_resin` 内置配方链路与 bootstrap/artifact 同步。
- [x] T2：落地 `unlock_stage` 校验与 `TransferMaterial` 显式优先级。
- [x] T3：落地 `maintenance_sink` 持续消耗并更新默认 profile 参数。
- [x] T4：补齐测试与回归，回写文档状态和 devlog 收口。

## 依赖
- doc/world-simulator/m4/m4-resource-product-system-playability-priority-hardening-2026-02-28.prd.md
- `doc/world-runtime/prd.md`
- `doc/world-simulator/m4/m4-industrial-economy-wasm.prd.md`
- `crates/oasis7/src/runtime/world/bootstrap_economy.rs`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_core.rs`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_economy.rs`
- `crates/oasis7/src/runtime/tests/economy*.rs`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T0~T4）。
- 阻塞项：无。
- 下一步：无（本轮可玩性优先强化已结项）。
