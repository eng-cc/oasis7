# oasis7 主链 Token 到 New API 内部额度桥接方案（设计文档）

- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`

审计轮次: 1

## 1. 设计目标
- 把 `OC` 到 `New API` 站内额度的路径定义成一条受控、独立部署、可对账的服务桥。
- 保持 `oasis7` runtime 只提供链上资产真值，不把 `New API` credit 逻辑塞回链上或浏览器。
- 为后续实现冻结最小状态机、部署边界、接口契约与 operator runbook 输入。

## 2. 架构拆分
- `bridge-portal`
  - 负责用户绑定、查看 deposit route、查看 credit 状态。
  - 不持有链上私钥；只展示 route / order / beneficiary。
- `bridge-core`
  - 负责 route 分配、pricing 解析、`bridge_ledger` 状态机、幂等重试与 operator action。
- `chain-watcher`
  - 负责从 `oasis7` 链侧观察 deposit truth。
  - 只读，不写链。
- `newapi-credit-adapter`
  - 负责把 confirmed deposit 转成 `New API` 的 quota mutation 或 redeem credit issuance。
  - 必须支持幂等键与审计回写。
- `bridge-db`
  - 保存用户绑定、route、pricing、`bridge_ledger`、operator 审计日志。

## 3. 最小数据模型
- `bridge_user_binding`
  - `bridge_user_id`
  - `newapi_user_ref`
  - `oasis_sender_account_id`
  - `status`
  - `created_at`
- `deposit_route`
  - `route_id`
  - `beneficiary_ref`
  - `deposit_account_id`
  - `route_type`
  - `expires_at`
  - `status`
- `bridge_ledger`
  - `bridge_deposit_id`
  - `route_id`
  - `chain_tx_id`
  - `to_account_id`
  - `amount_oc`
  - `pricing_version`
  - `credit_units`
  - `idempotency_key`
  - `state`
  - `adapter_receipt`
  - `review_reason`
  - `operator_note`
  - `created_at`
  - `updated_at`
- `pricing_schedule`
  - `pricing_version`
  - `effective_at`
  - `oc_amount`
  - `credit_units`
  - `bonus_units`
  - `status`

## 4. 状态机
- `deposit_route`
  - `draft -> issued -> settled`
  - `draft -> issued -> expired`
  - `issued -> disabled`
- `bridge_ledger`
  - `detected -> pending_confirmations -> confirmed -> crediting -> credited -> reconciled`
  - `detected -> manual_review`
  - `pending_confirmations -> manual_review`
  - `crediting -> failed -> manual_review`
  - `manual_review -> resolved -> reconciled`
  - `manual_review -> closed`

## 5. 关键接口契约
- `POST /v1/bridge/bind`
  - 输入: `newapi_user_ref`, `oasis_sender_account_id`
  - 输出: `binding_status`
  - 约束: 只建立受控绑定，不隐式开户。
- `POST /v1/bridge/deposit-route`
  - 输入: `bridge_user_id`, `pricing_version` 或 `topup_plan_id`
  - 输出: `route_id`, `deposit_account_id`, `expires_at`
  - 约束: 每个活跃 route 必须可唯一映射 beneficiary。
- `POST /bridge/operator/review/{bridge_deposit_id}`
  - 输入: `resolution`, `operator_note`
  - 输出: `next_state`
  - 约束: 手工处理必须写审计原因。
- `New API credit adapter`
  - 输入: `idempotency_key`, `beneficiary_ref`, `credit_units`, `target_type`
  - 输出: `adapter_receipt`, `applied=true|false`
  - 约束: 同一 `idempotency_key` 重试不得重复 credit。

## 6. 推荐实现顺序
1. 先做 `bridge_ledger` 状态机和 route/binding 数据模型。
2. 再做 watcher 与确认窗口逻辑。
3. 再做 `New API` credit adapter。
4. 最后做 operator dashboard 和用户 portal。

## 7. 风险门禁
- 没有唯一入账映射时，不允许自动 credit。
- 没有确认窗口时，不允许自动 credit。
- `New API` adapter 不支持幂等时，不允许自动 credit。
- bridge custody 若需要进入浏览器或 public bootstrap，提案直接失败。
- 对外文案若把该能力称为“公开兑换所”或“双向提现”，提案直接失败。
