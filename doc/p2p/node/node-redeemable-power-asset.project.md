# oasis7 Runtime：可兑现节点资产与电力兑换闭环（项目管理文档）

- 对应设计文档: `doc/p2p/node/node-redeemable-power-asset.design.md`
- 对应需求文档: `doc/p2p/node/node-redeemable-power-asset.prd.md`

审计轮次: 5
## 专业权威口径
- 本文件统一维护可兑现资产、审计与签名治理的任务状态；被吸收源文件只从 Git history 与 GitHub task evidence 追溯。

## 任务拆解（含 PRD-ID 映射）
- [x] RPA-0 (PRD-P2P-MIG-098)：完成设计文档与项目管理文档。
- [x] RPA-1 (PRD-P2P-MIG-098)：实现 `PowerCredit` 资产账本与配置（含快照持久化）。
- [x] RPA-2 (PRD-P2P-MIG-098)：将 `NodePoints` epoch 结算结果接入链状态铸造记录（`NodeRewardMintRecord`）。
- [x] RPA-3 (PRD-P2P-MIG-098)：实现 `RedeemPower` 动作闭环（余额扣减、Agent 电力增加、事件产出）。
- [x] RPA-4 (PRD-P2P-MIG-098)：实现守恒与风控（储备池、每 epoch 额度、最小兑换单位、nonce 防重放）。
- [x] RPA-5 (PRD-P2P-MIG-098)：接线运行时主链路（`oasis7_viewer_live`/runtime 开关与配置）。
- [x] RPA-6 (PRD-P2P-MIG-098)：实现最小需求侧支付入口（系统订单池预算分配）并接入结算。
- [x] RPA-7 (PRD-P2P-MIG-098)：实现身份签名治理最小收口（`node_id <-> public_key` 校验，拒绝未绑定提交）。
- [x] RPA-8 (PRD-P2P-MIG-098)：增强 DistFS 证明语义字段并补齐 `test_tier_required`/`test_tier_full` 回归。
- [x] RPA-9 (PRD-P2P-MIG-098)：文档状态回写、devlog 收口、发布说明整理。
- [x] redeemable-asset-audit-authority (PRD-P2P-MIG-096-001) [test_tier_required] + [test_tier_full]: 承接 AHA-0~AHA-5，完成 `mintsig:v1` 历史摘要、invariant report、检测非修复边界与恢复审计追踪。 Trace: #2655 (task_e86ac688cfbf4cc78809fd78c401c6cc)
- [x] redeemable-asset-signature-authority (PRD-P2P-MIG-097-001) [test_tier_required] + [test_tier_full]: 承接 SGC-0~SGC-5，完成 `mintsig:v2`、`redeemsig:v1`、v1/v2 兼容策略和 fail-closed 治理门禁追踪。 Trace: #2655 (task_e86ac688cfbf4cc78809fd78c401c6cc)

## 依赖
- `doc/p2p/node/node-redeemable-power-asset.prd.md`
- `crates/oasis7/src/runtime/node_points.rs`
- `crates/oasis7/src/runtime/node_points_runtime.rs`
- `crates/oasis7/src/runtime/state.rs`
- `crates/oasis7/src/runtime/events.rs`
- `crates/oasis7_proto/src/distributed.rs`
- `crates/oasis7_consensus/src/pos.rs`
- chain-runtime CLI、runtime root 与 reward report artifacts
- `crates/oasis7_distfs/src/lib.rs`
- `doc/devlog/README.md`

## 状态
- 当前阶段：RPA、AHA 与 SGC 语义合并完成；组件完成态不构成 release verdict。
- 下一步：持续验证旧快照 v1 兼容、v2 settlement/redemption 拒绝路径与 invariant report；不把进程重启当作资产对账。
- 最近更新：2026-07-27（专业权威合并）。
