# oasis7 主链 Token 分配与发行机制（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-allocation-mechanism.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
- [x] TAM-0 (PRD-P2P-MIG-112)：完成设计文档与项目管理文档建档。
- [x] TAM-1 (PRD-P2P-MIG-112)：实现主链 Token 状态模型、快照字段与基础查询接口。
- [x] TAM-2 (PRD-P2P-MIG-112)：实现创世分配初始化与 vesting 领取动作闭环。
- [x] TAM-3 (PRD-P2P-MIG-112)：实现 epoch 增发公式与分配执行（含余数确定性策略）。
- [x] TAM-4 (PRD-P2P-MIG-112)：实现 Gas/罚没/模块费用销毁与协议金库记账。
- [x] TAM-5 (PRD-P2P-MIG-112)：实现参数治理边界（范围校验、生效延迟、提案审计事件）。
- [x] TAM-6 (PRD-P2P-MIG-112)：实现 NodePoints -> 主链 Token 桥接占位接口并接入结算路径。
- [x] TAM-7 (PRD-P2P-MIG-112)：补齐 `test_tier_required` / `test_tier_full` 测试矩阵与回归脚本。
- [x] TAM-8 (PRD-P2P-MIG-112)：文档回写、发布说明与运行手册补充。
- [x] mainchain-token-allocation-phase2-governance-bridge-consolidation (PRD-P2P-MIG-112) [test_tier_required]: 吸收节点主链账户绑定、Approved/Applied proposal gate、受治理 treasury 分发和幂等审计；实现过程由 Git history 与 GitHub task evidence 追溯。 Trace: #2682 (task_172abebb99354d4fad395aa05a581193)

## 依赖
- doc/p2p/token/mainchain-token-allocation-mechanism.prd.md
- `doc/p2p/prd.md`
- `doc/p2p/node/node-redeemable-power-asset.prd.md`
- `doc/p2p/node/node-contribution-points.prd.md`（积分来源与双池结算）
- `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`（native action/event 与主链 Token bridge 当前权威）
- `crates/oasis7/src/runtime/reward_asset.rs`
- `crates/oasis7/src/runtime/state.rs`
- `crates/oasis7/src/runtime/world/resources.rs`
- `crates/oasis7/src/runtime/state/apply_domain_event_main_token.rs`
- `testing-manual.md`

## 状态
- 当前阶段：TAM-0 ~ TAM-8 全部完成。
- 下一步：后续经济、custody、mint、release 或外部钱包需求必须另立专业 authority；本专题不授予这些结论。
- 最近更新：2026-02-26。
