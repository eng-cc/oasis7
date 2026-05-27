# oasis7 Runtime：生产级区块链 + P2P FS 路线图 Phase B（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.design.md`
- 对应需求文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] PRG-B1 (PRD-P2P-MIG-054)：完成 Phase B 设计文档与项目管理文档。
- [x] PRG-B2 (PRD-P2P-MIG-054)：实现 `oasis7_node` 内生执行 hook、快照字段扩展与持久化兼容。
- [x] PRG-B3 (PRD-P2P-MIG-054)：实现 `oasis7_viewer_live` execution driver 接线，默认走节点内生执行并保留 fallback。
- [x] PRG-B4 (PRD-P2P-MIG-054)：补齐测试并执行 `test_tier_required` 回归，回写文档与 devlog。

## 依赖
- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.design.md`
- `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md`
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/pos_state_store.rs`
- `crates/oasis7/src/bin/oasis7_viewer_live.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/execution_bridge.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：PRG-B1 ~ PRG-B4 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
