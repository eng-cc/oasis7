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

### 2026-05-08 落地切片
- watcher 当前通过 operator-configured `--chain-base-url` 轮询 `GET /v1/chain/explorer/overview` 与 `GET /v1/chain/explorer/txs?account_id=<deposit_account_id>&status=confirmed`，以 `committed_height - block_height + 1` 计算确认数。
- bridge-service 只扫描自己发出的 `deposit_route`，不做“按 `oc:bridge:` 前缀扫全链”的猜测式归属；任何未绑定、超额、少额、重复 route 入账都直接进入 `manual_review`。
- pricing 当前使用 operator-configured repeatable `--pricing-rule <pricing_version>:<oc_amount>:<credit_units>[:<bonus_units>]` 精确匹配；`topup_plan_id` 目前不会自动折算，命中时进入 `manual_review`。
- `New API` credit path 不在 repo 内硬编码；bridge-service 只要求 operator 通过 `--credit-adapter-url` / `--credit-adapter-auth-token` 注入真实 admin-side 写入口，并对同一 `idempotency_key` 做幂等。

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
  - `chain_action_id`
  - `from_account_id`
  - `to_account_id`
  - `amount_oc`
  - `expected_amount_oc`
  - `pricing_version`
  - `credit_units`
  - `bonus_units`
  - `total_credit_units`
  - `idempotency_key`
  - `confirmations`
  - `required_confirmations`
  - `block_height`
  - `target_type`
  - `state`
  - `adapter_receipt`
  - `review_reason`
  - `review_resolution`
  - `operator_note`
  - `credit_attempt_count`
  - `last_error_code`
  - `last_error`
  - `observed_at`
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
- `POST /v1/bridge/reconcile`
  - 输入: 空 body；由 operator 或后台 interval 触发一次 `scan routes -> observe deposits -> promote confirmations -> apply credit`
  - 输出: `latest_committed_height`、`observed_new_deposit_count`、`reconciled_credit_count`、`manual_review_count`
  - 约束: 若 `chain-base-url` 或 `credit-adapter-url` 未配置，返回 operator-facing error，不猜测默认路径。
- `POST /v1/bridge/operator/review/{bridge_deposit_id}`
  - 输入: `resolution`, `operator_note`
  - 输出: `next_state`
  - 约束: 当前最小实现只支持 `mark_resolved|close`，不在 bridge-service 内提供“人工改 credit_units 后重发”的隐式后门。
- `New API credit adapter`
  - 输入: `bridge_deposit_id`, `beneficiary_ref`, `pricing_version`, `amount_oc`, `credit_units`, `bonus_units`, `total_credit_units`, `target_type`, `chain_tx_id`, `idempotency_key`
  - 输出: 任意 JSON `adapter_receipt`
  - 约束: bridge-service 以 HTTP `2xx` 视为 success；非 `2xx` / timeout 会保留同一 `idempotency_key` 重试，不得双发。

## 6. 推荐实现顺序
1. 已完成 `bridge_ledger` 状态机和 route/binding 数据模型。
2. 已完成 watcher 与确认窗口逻辑（基于 explorer poll + block height confirmation）。
3. 已完成 `New API` credit adapter 的 generic HTTP bridge。
4. 后续再做 operator dashboard、用户 portal 和 richer manual-review action。

## 7. 风险门禁
- 没有唯一入账映射时，不允许自动 credit。
- 没有确认窗口时，不允许自动 credit。
- `New API` adapter 不支持幂等时，不允许自动 credit。
- 当前 exact-match pricing rule 之外的充值形态一律不得自动折算，避免把 underpay / overpay / plan drift 静默吞成额度。
- bridge custody 若需要进入浏览器或 public bootstrap，提案直接失败。
- 对外文案若把该能力称为“公开兑换所”或“双向提现”，提案直接失败。
