# M4 资源与产品系统 P3：分层档案化与链路扩展实现（项目管理文档）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.design.md`
- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] T0：输出 P3 设计文档与项目管理文档。
- [x] T1：落地 Profile 结构（ABI + WorldState）与默认目录注入。
- [x] T2：接线 Profile 驱动规则（优先级、运损、阶段门槛、瓶颈标签、role_tag 优先级）。
- [x] T3：扩展内置模块链路并同步 bootstrap/hash/identity 清单。
- [x] T4：完成 required/full 回归，回写文档状态与 devlog 收口。

## 依赖
- doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md
- `doc/world-runtime/prd.md`
- `crates/oasis7_wasm_abi/src/economy.rs`
- `crates/oasis7/src/runtime/state.rs`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_core.rs`
- `crates/oasis7/src/runtime/world/event_processing/action_to_event_economy.rs`
- `crates/oasis7/src/runtime/world/bootstrap_economy.rs`
- `crates/oasis7/src/runtime/world/artifacts/m4_builtin_*`

## 状态
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 当前阶段：已完成（T0/T1/T2/T3/T4 全部完成）。
- 阻塞项：无。
- 下一步：无（P3 结项）。
