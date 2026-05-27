# oasis7 Runtime：区块链 + P2P FS 硬改造（Phase 7）项目管理文档（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.design.md`
- 对应需求文档: `doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] HP7-0 (PRD-P2P-MIG-051)：输出设计文档与项目管理文档。
- [x] HP7-1 (PRD-P2P-MIG-051)：实现 sequencer action signer 白名单配置校验与规范化集合。
- [x] HP7-2 (PRD-P2P-MIG-051)：补齐 sequencer allowlist 单测并执行回归，回写文档状态与 devlog。

## 依赖
- doc/p2p/blockchain/blockchain-p2pfs-hardening-phase7.prd.md
- `crates/oasis7_consensus/src/sequencer_mainloop.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：HP7-0 ~ HP7-2 全部完成。
- 阻塞项：无。
- 最近更新：2026-02-17。
