# oasis7 Runtime：节点基础在线时长奖励（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-uptime-base-reward.design.md`
- 对应需求文档: `doc/p2p/node/node-uptime-base-reward.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] UBR-1 (PRD-P2P-MIG-103)：完成设计文档与项目管理文档。
- [x] UBR-2 (PRD-P2P-MIG-103)：实现 `node_points` 在线挑战门槛与归一化奖励，补齐单元测试。
- [x] UBR-3 (PRD-P2P-MIG-103)：实现 `node_points_runtime` 挑战采样接线并补齐运行时测试。
- [x] UBR-4 (PRD-P2P-MIG-103)：执行 `test_tier_required` 回归，更新状态与 devlog 收口。

## 依赖
- doc/p2p/node/node-uptime-base-reward.prd.md
- `crates/oasis7/src/runtime/node_points.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/runtime/mod.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：节点基础在线时长奖励阶段完成（UBR-1~UBR-4 全部完成）。
- 最近更新：2026-02-16。
