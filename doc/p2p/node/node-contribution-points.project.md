# oasis7 Runtime：节点贡献积分激励（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-contribution-points.design.md`
- 对应需求文档: `doc/p2p/node/node-contribution-points.prd.md`

审计轮次: 5
## ROUND-002 主从口径
- 项目管理主入口：`doc/p2p/node/node-contribution-points.project.md`。
- 从项目文档：`node-contribution-points-runtime-closure.project.md`、`node-contribution-points-multi-node-closure-test.project.md` 仅维护增量任务与状态，主计划以本文件为准。

## 任务拆解（含 PRD-ID 映射）
- [x] NCP-1 (PRD-P2P-MIG-091)：完成设计文档与项目管理文档。
- [x] NCP-2 (PRD-P2P-MIG-091)：实现节点积分结算引擎（额外计算/存储/在线/惩罚 + 积分台账）。
- [x] NCP-3 (PRD-P2P-MIG-091)：补齐单元测试并在 runtime 模块导出接口，执行 test_tier_required 回归。
- [x] NCP-4 (PRD-P2P-MIG-091)：回写项目状态与 devlog，完成收口。

## 依赖
- doc/p2p/node/node-contribution-points.prd.md
- `crates/oasis7/src/runtime/mod.rs`
- `crates/oasis7/src/runtime`（新增节点积分模块）
- `doc/p2p/distributed/distributed-runtime.prd.md`
- `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md`

## 状态
- 当前阶段：节点贡献积分激励阶段完成（NCP-1~NCP-4 全部完成）。
- 最近更新：2026-02-16。
