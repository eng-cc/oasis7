# oasis7 主链 Token 到 LetAI Run OpenAPI 额度桥接方案（设计文档）

- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`

审计轮次: 2

## 1. 设计目标
- 把 `OC` 到 LetAI Run 调用额度的路径定义成一条受控、独立部署、可对账的 OpenAPI bridge。
- 让 bridge-service 直接表达 LetAI 的真实对象模型：`platform user -> project -> token_key -> topup -> query verification`。
- 保持 `oasis7` runtime 只提供链上资产真值，不把 LetAI 用户/项目/Token 逻辑塞回链上或浏览器。

## 2. 架构拆分
- `bridge-portal`
  - 负责用户绑定、查看 deposit route、查看 project/token/topup 状态。
  - 不持有链上私钥；只展示 route / beneficiary / audit status。
  - 默认不明文展示或导出 `token_key`。
- `bridge-core`
  - 负责 route 分配、pricing 解析、`bridge_ledger` 状态机、LetAI 资源映射、幂等重试与 operator action。
- `chain-watcher`
  - 负责从 `oasis7` 链侧观察 deposit truth。
  - 只读，不写链。
- `receiving-account-manager`
  - 负责维护每个 newapi 接入方的 OC 收款账号池、账号状态、轮换和 route 分配视图。
  - 当前推荐按接入方维护小池化账号组，而不是每笔充值现生成新地址。
- `letai-openapi-adapter`
  - 负责 `users/upsert`、项目创建/Token 返回、用户 topup、额度概览、日志查询与项目 Token 汇总。
  - 必须支持幂等键、query verification 与审计回写。
- `bridge-db`
  - 保存 bridge binding、route、pricing、`bridge_ledger`、LetAI user/project/token 映射与 operator 审计日志。

## 3. 最小数据模型
- `bridge_user_binding`
  - `provider_id`
  - `bridge_user_id`
  - `newapi_user_ref`
  - `oasis_sender_account_id`
  - `letai_external_user_id`
  - `platform_user_id`
  - `letai_external_user_name`
  - `email`
  - `metadata`
  - `status`
  - `created_at`
  - `updated_at`
- `letai_project_binding`
  - `bridge_user_id`
  - `letai_external_project_id`
  - `platform_project_id`
  - `project_name`
  - `token_key`
  - `token_status`
  - `created_at`
  - `updated_at`
- `deposit_route`
  - `route_id`
  - `provider_id`
  - `bridge_user_id`
  - `beneficiary_ref`
  - `deposit_account_id`
  - `deposit_token`
  - `route_type`
  - `pricing_version`
  - `topup_plan_id`
  - `expires_at`
  - `status`
- `provider_receiving_account`
  - `provider_id`
  - `account_id`
  - `account_label`
  - `pool_slot`
  - `custody_mode`
  - `status`
  - `rotation_epoch`
  - `last_assigned_at`
  - `created_at`
  - `updated_at`
- `bridge_ledger`
  - `bridge_deposit_id`
  - `route_id`
  - `bridge_user_id`
  - `chain_tx_id`
  - `chain_action_id`
  - `amount_oc`
  - `expected_amount_oc`
  - `pricing_version`
  - `credit_units`
  - `bonus_units`
  - `total_credit_units`
  - `platform_user_id`
  - `platform_project_id`
  - `token_key`
  - `external_order_id`
  - `quota`
  - `amount_audit`
  - `currency`
  - `topup_receipt`
  - `user_snapshot`
  - `project_snapshot`
  - `topup_log_snapshot`
  - `idempotency_key`
  - `state`
  - `review_reason`
  - `operator_note`
  - `credit_attempt_count`
  - `last_error_code`
  - `last_error`
  - `observed_at`
  - `updated_at`
- `pricing_schedule`
  - `pricing_version`
  - `oc_amount`
  - `quota_units`
  - `bonus_quota_units`
  - `status`
  - 示例配置真值入口: `scripts/newapi-bridge-service/pricing-rules.example.env`

## 4. 状态机
- `bridge_user_binding`
  - `active_local_only -> user_ready`
  - `user_ready -> project_ready`
  - `project_ready -> project_token_ready`
  - 任意状态可进入 `manual_review`
- `deposit_route`
  - `draft -> issued -> settled`
  - `draft -> issued -> expired`
  - `issued -> disabled`
- `provider_receiving_account`
  - `provisioning -> active`
  - `active -> draining`
  - `draining -> retired`
  - `active -> disabled`
- `bridge_ledger`
  - `detected -> pending_confirmations -> confirmed`
  - `confirmed -> provisioning_user -> provisioning_project -> crediting -> credited -> verifying -> reconciled`
  - `confirmed -> provisioning_user -> manual_review`
  - `confirmed -> provisioning_project -> manual_review`
  - `crediting -> failed -> crediting`
  - `credited -> verifying -> manual_review`
  - `manual_review -> resolved -> reconciled`
  - `manual_review -> closed`

## 5. 关键接口契约
- `POST /v1/bridge/bind`
  - 输入: `provider_id`, `newapi_user_ref`, `oasis_sender_account_id`, optional `external_user_name`, `email`, `metadata`
  - 输出: `bridge_user_id`, local binding status
  - 约束: 只建立受控 bridge binding，不隐式 topup；`provider_id` 是 route 选择收款账号池的上游真值。
- `POST /v1/bridge/deposit-route`
  - 输入: `bridge_user_id`, `pricing_version` 或 `topup_plan_id`
  - 当前代码输出: `route_id`, `deposit_account_id`, `expires_at`
  - 目标态扩展输出: `route_id`, `provider_id`, `deposit_account_id`, optional `deposit_token`, `expires_at`
  - 约束: 每个活跃 route 必须可唯一映射 beneficiary；目标态里 `deposit_account_id` 默认从 `bridge_user_binding.provider_id` 对应的接入方收款账号池选择，不要求 route 自己拥有独立链地址。
- `POST /v1/bridge/reconcile`
  - 输入: 空 body；由 operator 或后台 interval 触发一次 `scan routes -> observe deposits -> ensure user/project/token -> topup -> verify`
  - 输出: `latest_committed_height`、`observed_new_deposit_count`、`reconciled_credit_count`、`manual_review_count`
  - 约束: 若 `chain-base-url`、LetAI base URL 或 platform key 未配置，返回 operator-facing error。
- `POST /v1/bridge/operator/review/{bridge_deposit_id}`
  - 输入: `resolution`, `operator_note`
  - 输出: `next_state`
  - 约束: 当前最小实现仍只支持 `mark_resolved|close`。
- `LetAI OpenAPI`
  - `POST /api/platform/open/users/upsert`
  - `POST /api/platform/open/users/:platform_user_id/topups`
  - `GET /api/platform/open/users/:platform_user_id/...`
  - “创建或获取项目并返回 Token”
  - “查询项目 Token 汇总”
  - 约束:
    - `Authorization: Bearer <platform-key>`
    - `token_key` 只用于实际模型调用，不用于管理接口
    - `external_user_id` / `external_project_id` / `external_order_id` 必须稳定幂等

## 6. 编排顺序
1. 当前代码只生成 route 并返回 `deposit_account_id`；目标态扩展为 route allocator 按 `bridge_user_binding.provider_id` 选择一个 active 收款账号池成员，并可选签发一次性 `deposit_token`。
2. watcher 观察 route 对应链上 confirmed deposit。
3. pricing engine 把 `OC` 金额折算成 LetAI `quota`。
4. `users/upsert` 确保 `platform_user_id` 存在，并回写 binding。
5. `ensure project + token_key` 确保 bridge user 绑定独立 project，回写 `platform_project_id/token_key`。
6. 调用 topup，使用稳定 `external_order_id`。
7. 调用用户概览 / 日志 / 项目汇总做验证。
8. 回写 ledger snapshots，标记 `reconciled` 或 `manual_review`。

## 7. 收款策略
- 当前推荐:
  - 每个 newapi 接入方维护一组受控 OC 收款账号。
  - 初始规模可用小池化，例如 5 个账号。
  - 每次充值仍然生成独立 route，route 只是在账号池里选择一个当前可用账号，而不是现生成新地址。
  - `provider_id` 必须在 bind 阶段就固定到 `bridge_user_binding`，避免 route 分配时临时猜测“属于哪个接入方”。
- 归因顺序:
  1. `route_id` / `provider_id`
  2. `deposit_account_id`
  3. `expires_at` 有效期
  4. `pricing_version` / `topup_plan_id`
  5. `from_account_id`
  6. optional `deposit_token`（若链上 `message/memo` 可用）
- 为什么不默认全平台单账号:
  - 接入方之间缺少资金隔离。
  - operator 轮换和风控粒度太粗。
  - manual review 时容易把接入方账混在一起。
- 为什么不默认每单现生成新地址:
  - 需要完整密钥派生、托管和归集系统。
  - 当前 bridge 更缺 route/ledger/custody 的稳定闭环，而不是地址数量。
- `message/memo` 边界:
  - 若未来链上转账支持 `message/memo`，默认只放短 `deposit_token`。
  - `deposit_token` 只做辅助归因，不替代 bridge state / ledger。
  - 不建议把完整加密对账信息直接塞进链上消息体。

## 8. 推荐实现顺序
1. 先扩展 `model.rs` / `store.rs`，补齐 `provider_id`、收款账号池、LetAI user/project/token/topup/query 字段。
2. 替换 generic `credit_adapter.rs` 为 LetAI OpenAPI adapter，支持多接口。
3. 重写 `service.rs` route 分配与 reconcile 编排，从 `Confirmed` 推进到 `Reconciled`。
4. 若链上准备支持 `message/memo`，补 `deposit_token` 签发/验证与 explorer 读取。
5. 重写测试桩，覆盖账号池复用、user/project/token/topup/query 的 happy path、retry 和 manual review。
6. 最后再补 richer operator dashboard / portal。

## 9. 风险门禁
- 没有 `platform_user_id` 时，不允许继续 project/topup。
- 没有 `platform_project_id/token_key` 时，不允许继续 topup。
- topup 没有 query verification snapshot 时，不允许标记 reconciled。
- `external_order_id` 不稳定时，不允许自动重试。
- 当前 exact-match pricing rule 之外的充值形态一律进入 `manual_review`。
- `token_key` 若需要进入浏览器或 public bootstrap，提案直接失败。
- 收款账号池若没有真实 custody 能力，只靠逻辑命名，提案直接失败。
- 若多个活跃 route 共用同一收款账号但缺少足够归因字段，提案直接失败。
