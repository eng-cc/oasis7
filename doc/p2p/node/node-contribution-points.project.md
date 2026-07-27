# oasis7 Runtime：节点贡献积分激励（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-contribution-points.design.md`
- 对应需求文档: `doc/p2p/node/node-contribution-points.prd.md`

审计轮次: 5
## 专业权威口径
- 本文件是节点贡献积分任务状态的当前主入口。
- runtime closure 与 multi-node closure test 的完成态、验收边界和追踪关系已合并到本文件；已删除源文档只通过 Git history 与 GitHub task evidence 追溯。

## 任务拆解（含 PRD-ID 映射）
- [x] NCP-1 (PRD-P2P-MIG-091)：完成设计文档与项目管理文档。
- [x] NCP-2 (PRD-P2P-MIG-091)：实现节点积分结算引擎（额外计算/存储/在线/惩罚 + 积分台账）。
- [x] NCP-3 (PRD-P2P-MIG-091)：补齐单元测试并在 runtime 模块导出接口，执行 test_tier_required 回归。
- [x] NCP-4 (PRD-P2P-MIG-091)：回写项目状态与 devlog，完成收口。
- [x] node-points-runtime-closure (PRD-P2P-MIG-091) [test_tier_required]: 完成 epoch 幂等结算与恢复/重放闭环，保证快照更新不静默改写奖励台账。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)
- [x] node-points-multi-node-closure (PRD-P2P-MIG-091) [test_tier_required]: 完成至少 3 节点、连续 2 epoch 的闭环验证，覆盖积分池守恒、贡献排序、惩罚效果与累计积分单调性。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)

## 依赖
- doc/p2p/node/node-contribution-points.prd.md
- `crates/oasis7/src/runtime/mod.rs`
- `crates/oasis7/src/runtime`（新增节点积分模块）
- `doc/p2p/distributed/distributed-runtime.prd.md`
- `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md`

## 状态
- 当前阶段：节点贡献积分激励阶段完成（NCP-1~NCP-6 全部完成）；本地闭环不构成真实公网证明或生产 readiness。
- 最近更新：2026-07-27（专业权威合并）。
