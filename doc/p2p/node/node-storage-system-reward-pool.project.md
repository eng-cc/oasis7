# oasis7 Runtime：节点存储系统奖励池（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-storage-system-reward-pool.design.md`
- 对应需求文档: `doc/p2p/node/node-storage-system-reward-pool.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] SBR-1 (PRD-P2P-MIG-102)：完成设计文档与项目管理文档。
- [x] SBR-2 (PRD-P2P-MIG-102)：实现 `node_points` 双池结算（主池+存储池）与存储挑战门槛逻辑，补齐单元测试。
- [x] SBR-3 (PRD-P2P-MIG-102)：实现 `node_points_runtime` 存储挑战采样透传与运行时测试。
- [x] SBR-4 (PRD-P2P-MIG-102)：执行 `test_tier_required` 回归，回写文档与 devlog 收口。

## 依赖
- doc/p2p/node/node-storage-system-reward-pool.prd.md
- `crates/oasis7/src/runtime/node_points.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/runtime/mod.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：节点存储系统奖励池阶段完成（SBR-1~SBR-4 全部完成）。
- 最近更新：2026-02-16。
