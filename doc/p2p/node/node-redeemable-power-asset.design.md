# oasis7 Runtime：可兑现节点资产与电力兑换闭环设计

- 对应需求文档: `doc/p2p/node/node-redeemable-power-asset.prd.md`
- 对应项目管理文档: GitHub Issue / GitHub Project

## 1. 设计定位
定义可兑现节点资产与电力兑换主闭环，统一节点资产记录、兑换条件、结算入口与兑现边界。

## 2. 设计结构
- 资产建模层：定义节点可兑现资产、余额与来源。
- 电力兑换层：把节点贡献/资产映射到电力或可消费权益。
- 结算执行层：处理兑换申请、确认与状态落账。
- 审计治理层：以 invariant report 检测守恒/签名异常但不自动修复；恢复后重验策略与报告。
- 签名演进层：兼容历史 `mintsig:v1` 摘要，以 `mintsig:v2`/`redeemsig:v1` Ed25519 和治理策略实现 fail-closed 门禁。

## 3. 关键接口 / 入口
- 节点资产台账
- 电力兑换申请/确认
- 结算落账入口
- 资产审计记录
- `RewardAssetInvariantReport`
- `RewardSignatureGovernancePolicy`

## 4. 约束与边界
- 资产余额与兑换结果必须可追溯。
- 兑换链路不得绕过治理与签名约束。
- v1 摘要不得宣传为密码学签名；v2/兑换签名策略缺少密钥或绑定时必须拒绝。
- 运行证据来自 chain-runtime CLI/runtime-root/report artifacts，历史 viewer flags 不构成当前运维入口。
- 不在本专题扩展完整外部市场流转。

## 5. 设计演进计划
- 先冻结资产与兑换规则。
- 再打通运行时结算。
- 最后持续验证审计、签名治理和旧快照兼容边界。
