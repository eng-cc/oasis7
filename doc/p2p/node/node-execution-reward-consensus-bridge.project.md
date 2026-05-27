# oasis7 Runtime：节点执行桥接与奖励共识触发闭环（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-execution-reward-consensus-bridge.design.md`
- 对应需求文档: `doc/p2p/node/node-execution-reward-consensus-bridge.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] ERCB-0 (PRD-P2P-MIG-093)：完成设计文档与项目管理文档。
- [x] ERCB-1 (PRD-P2P-MIG-093)：实现执行桥接器（`committed_height -> RuntimeWorld` 执行记录 + CAS 落盘 + 状态恢复）。
- [x] ERCB-2 (PRD-P2P-MIG-093)：实现奖励结算网络触发（结算包发布/订阅/应用，含单机 fallback）。
- [x] ERCB-3 (PRD-P2P-MIG-093)：实现多观察者签名轨迹（发布、验签、去重、collector 接线、报表输出）。
- [x] ERCB-4 (PRD-P2P-MIG-093)：补齐测试与 CLI 参数覆盖，执行 `test_tier_required` 回归。
- [x] ERCB-5 (PRD-P2P-MIG-093)：回写项目状态与 devlog 收口。

## 依赖
- `doc/p2p/node/node-execution-reward-consensus-bridge.prd.md`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/reward_runtime_settlement.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/distfs_probe_runtime.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs（`#[cfg(test)]`）`
- `doc/devlog/README.md`

## 状态
- 当前阶段：ERCB 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
