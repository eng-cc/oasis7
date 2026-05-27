# oasis7 Runtime：生产级区块链 + P2P FS 路线图（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.design.md`
- 对应需求文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] PRG-1 (PRD-P2P-MIG-056)：完成路线图设计文档与项目管理文档。
- [x] PRG-2 (PRD-P2P-MIG-056)：实现 `oasis7_node` 链式 `block_hash`（含状态持久化兼容）。
- [x] PRG-3 (PRD-P2P-MIG-056)：实现奖励结算 `RewardSettlementEnvelope` 传输签名与消费端验签。
- [x] PRG-4 (PRD-P2P-MIG-056)：补齐测试并执行 `test_tier_required` 回归，回写文档与 devlog。

## 依赖
- doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/pos_state_store.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/reward_runtime_worker.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：PRG-1 ~ PRG-4 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
