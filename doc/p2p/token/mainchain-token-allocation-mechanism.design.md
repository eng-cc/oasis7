# oasis7 主链 Token 分配与发行机制（已实现）设计

- 对应需求文档: `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

## 1. 设计定位
定义 oasis7 主链 Token 分配与发行机制主设计，统一总量、分配对象、发行节奏与治理约束。

## 2. 设计结构
- 发行模型层：定义 token 总量、发行节奏与基础约束。
- 分配策略层：明确节点、治理、生态等对象的分配规则，以及 node-to-main-token 内部地址绑定。
- 执行落账层：把分配结果映射到主链状态与发放流程；二期 treasury 分发只允许 staking/ecosystem/security 三个 bucket。
- 治理审计层：策略更新和 treasury 分发必须绑定 Approved/Applied proposal；以 `distribution_id` 记录幂等审计。

## 3. 关键接口 / 入口
- token 发行模型
- 分配对象与比例
- `World::node_main_token_account` / `World::bind_node_main_token_account`
- `Action::DistributeMainTokenTreasury` 与 `DomainEvent::MainTokenTreasuryDistributed`
- `main_token_treasury_distribution_records` 分配审计记录

## 4. 约束与边界
- 总量与分配比例必须可追溯、可审计。
- 分配执行不得绕过治理约束；策略更新与 treasury 分发均要求已批准或已应用 proposal。
- distribution 会改变 recipient liquid balance 与 circulating supply，但不改变 total supply。
- 地址绑定是 runtime 内部字符串模型，不扩展为外部钱包/跨链协议，也不改变经济参数。

## 5. 设计演进计划
- 先冻结主链 token 主模型。
- 再细化受治理的地址绑定和 treasury 分发执行路径。
- 保持历史二期实施过程在 Git history 与 GitHub task evidence 可追溯。
