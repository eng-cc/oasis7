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
- [x] node-points-runtime-closure (PRD-P2P-MIG-090-001) [test_tier_required]: 承接原 runtime-closure 专题 ID，完成 epoch 幂等结算与恢复/重放闭环，保证快照更新不静默改写奖励台账。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)
- [x] node-points-multi-node-closure (PRD-P2P-MIG-089-001) [test_tier_required]: 承接原 multi-node-closure-test 专题 ID，完成至少 3 节点、连续 2 epoch 的闭环验证，覆盖积分池守恒、贡献排序、惩罚效果与累计积分单调性。 Trace: #2652 (task_33241c6a236149efbe1790f03e1cc1f6)
- [x] reward-runtime-production-hardening-phase1-authority (PRD-P2P-MIG-100) [test_tier_required]: 已吸收 collector snapshot/restart、显式身份绑定与兑换 signer-match 收口；后续回归继续覆盖 collector 恢复、epoch 幂等与重复 tick/replay 不增发。历史过程只由 Git history 与 GitHub task evidence 追溯。 Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)
- [x] node-points-dual-pool-authority (PRD-P2P-MIG-102-001) [test_tier_required]: 吸收双固定池、挑战资格、质押封顶、独立分配与反重复存储奖励合同；原专题三件套退役后从 Git history 与 GitHub task evidence 追溯。 Trace: #2768 (task_f98e5ca7d8c84f20988039c03f90dde5)
- [x] node-points-challenge-uptime-authority (PRD-P2P-MIG-103-001) [test_tier_required]: 吸收挑战优先、时长回退与阈值归一化在线积分合同；原专题三件套退役后从 Git history 与 GitHub task evidence 追溯。 Trace: #2768 (task_f98e5ca7d8c84f20988039c03f90dde5)

## 依赖
- doc/p2p/node/node-contribution-points.prd.md
- `crates/oasis7/src/runtime/mod.rs`
- `crates/oasis7/src/runtime`（新增节点积分模块）
- `doc/p2p/prd.md`（分布式运行时与复制恢复合同）
- `doc/p2p/blockchain/p2p-blockchain-p2pfs-hardening.prd.md`

## 状态
- 当前阶段：节点贡献积分激励及 storage/uptime 专业权威合并完成；本次只合并现有语义，不证明新行为、参数重调、真实公网或生产 readiness。
- 最近更新：2026-07-29（专业权威合并）。
