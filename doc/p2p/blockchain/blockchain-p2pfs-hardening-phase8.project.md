# oasis7 Runtime：区块链 + P2P FS 硬改造（Phase 8）项目管理文档（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.design.md`
- 对应需求文档: `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] HP8-0 (PRD-P2P-MIG-052)：输出设计文档与项目管理文档。
- [x] HP8-1 (PRD-P2P-MIG-052)：实现共享 ed25519 公钥规范化/allowlist 工具并接线 membership、sequencer、signature。
- [x] HP8-2 (PRD-P2P-MIG-052)：补齐共享工具单测并执行回归，回写文档状态与 devlog。

## 依赖
- doc/p2p/blockchain/blockchain-p2pfs-hardening-phase8.prd.md
- `crates/oasis7_consensus/src/lib.rs`
- `crates/oasis7_consensus/src/signature.rs`
- `crates/oasis7_consensus/src/membership_logic.rs`
- `crates/oasis7_consensus/src/sequencer_mainloop.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：HP8-0 ~ HP8-2 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
