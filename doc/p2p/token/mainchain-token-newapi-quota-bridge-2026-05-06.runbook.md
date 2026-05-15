# oasis7 主链 Token 到 LetAI Run OpenAPI 额度桥接方案（Operator Runbook）

- 对应需求文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
- 对应设计文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.design.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`

审计轮次: 1

## Meta
- Owner Role: `runtime_engineer`
- Review Role: `qa_engineer`
- Scope: `bridge-service` 独立部署、配置注入、首次演练、日常 reconcile、manual review 与回滚边界
- Current Verdict: `operator_runbook_drafted`

## 1. 适用范围
- 本 runbook 只覆盖 `one-way OC -> LetAI Run OpenAPI quota` bridge 的 operator 侧执行。
- 本 runbook 不覆盖：
  - 浏览器下发 `token_key`
  - `OC <- quota/token_key` 兑回
  - 公开兑换所 / AMM / orderbook
  - richer operator dashboard
- 当前 bridge 仍属于 `limited preview operator-managed service-credit bridge`。

## 2. 开始前输入
每次部署或演练前，必须先固定：

- bridge-service 二进制来源
  - 当前入口: `crates/oasis7/src/bin/oasis7_newapi_bridge_service.rs`
- 状态文件路径
  - 建议单独目录，例如 `output/newapi-bridge/bridge-state.json`
- 链侧只读输入
  - `--chain-base-url`
  - `--chain-confirmations-required`
- LetAI 管理面输入
  - `--letai-base-url`
  - `--letai-platform-key`
  - `--letai-parent-channel-id`
- 定价输入
  - 至少一条 `--pricing-rule <pricing_version:oc_amount:credit_units:bonus_units>`
- 运维输入
  - 监听地址 `--bind-addr`
  - 自动 reconcile 间隔 `--reconcile-interval-seconds`
  - route TTL `--route-ttl-seconds`
- 值班 owner
  - `runtime_engineer`
  - `qa_engineer`

## 3. 硬阻断条件
- 缺任一 LetAI 凭证输入：不得启动自动 bridge。
- `--chain-base-url` 未配置：不得执行 reconcile。
- 未冻结任何 `--pricing-rule`：不得对外发放 deposit route。
- 状态文件路径位于公共静态目录、HTML 产物目录或浏览器可下载路径：直接阻断。
- bridge-service 与 public web/player plane 混部且没有最小访问隔离：直接阻断。
- operator 无法证明 `token_key` 不会进入公共日志、公共 API 或浏览器 bootstrap：直接阻断。

## 4. 推荐启动命令
示例仅作为 operator 参考，实际值必须来自受控环境：

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_newapi_bridge_service -- \
  --bind-addr 127.0.0.1:5852 \
  --state-path output/newapi-bridge/bridge-state.json \
  --route-ttl-seconds 900 \
  --deposit-account-prefix oc:bridge: \
  --chain-base-url http://127.0.0.1:5010 \
  --chain-confirmations-required 1 \
  --pricing-rule pv-1:100:10:5 \
  --letai-base-url https://api.letai.run \
  --letai-platform-key "$LETAI_PLATFORM_KEY" \
  --letai-parent-channel-id "$LETAI_PARENT_CHANNEL_ID" \
  --reconcile-interval-seconds 15
```

约束：
- `LETAI_PLATFORM_KEY` 只能来自受控服务端环境变量或等价 secret store。
- 不要把真实 `platform key` / `parent channel id` 写进 repo、脚本默认值或 public CI log。
- 若只做手工演练，可把 `--reconcile-interval-seconds` 设为 `0`，改用手工 `POST /v1/bridge/reconcile`。

## 5. 首次部署检查
1. 先跑定向测试：
   - `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture`
2. 再确认文档/行数门禁：
   - `./scripts/check-rust-file-size.sh`
   - `./scripts/doc-governance-check.sh`
3. 启动 bridge-service 后，先检查健康接口：
   - `GET /health`
   - `GET /v1/bridge/health`
4. 确认 health 输出中：
   - `ok=true`
   - `binding_count/project_binding_count/route_count/ledger_count` 初值符合预期
5. 确认状态文件已落到 operator 指定路径，而不是默认 public 目录。

## 6. 首次演练流程

### 6.1 建立用户绑定
- 调用 `POST /v1/bridge/bind`
- 输入：
  - `newapi_user_ref`
  - `oasis_sender_account_id`
  - 可选 `external_user_name/email/project_name/project_metadata`
- 期望：
  - 返回 `bridge_user_id`
  - 不返回 `platform_user_id/platform_project_id/token_key`

### 6.2 分配充值路由
- 调用 `POST /v1/bridge/deposit-route`
- 输入：
  - `bridge_user_id`
  - `pricing_version` 或 `topup_plan_id`
- 期望：
  - 返回唯一 `deposit_account_id`
  - `route_status=issued`

### 6.3 链上入账
- 让绑定用户向 `deposit_account_id` 转入与 `pricing_version` 对应的 `OC`
- 期望：
  - chain watcher 观察到 route 对应入账
  - 不匹配金额、重复 route、过期 route 默认进入 `manual_review`

### 6.4 reconcile
- 手工触发：
  - `POST /v1/bridge/reconcile`
- 或等待后台 interval
- 期望顺序：
  - `confirmed -> provisioning_user -> provisioning_project -> crediting -> credited -> reconciled`
- 审计真值至少包括：
  - `platform_user_id`
  - `platform_project_id`
  - `token_key`
  - `external_order_id`
  - `user_snapshot/project_snapshot/topup_log_snapshot`

## 7. 日常巡检
- health 接口关注：
  - `manual_review_count`
  - `failed_credit_count`
  - `pending_confirmation_count`
  - `reconciled_count`
- 状态文件巡检关注：
  - 是否存在长期停留在 `Failed` 的记录
  - 是否存在 `ManualReview` 且无 `operator_note`
  - 同一 `bridge_deposit_id` 是否出现多条业务 order
  - `token_key` 是否只出现在受控状态文件，不出现在 public logs / API 回包

## 8. 异常收口

### 8.1 常见异常
- `underpay` / `overpay`
- `expired_route_deposit`
- `duplicate_route_deposit`
- `binding_not_found`
- `project_binding_not_found`
- `letai_topup_log_mismatch`
- `letai_project_summary_mismatch`
- LetAI 5xx / timeout / decode failed

### 8.2 处理原则
- 单条异常不得阻断整轮 reconcile。
- 缺 binding / project binding 时，当前实现会把该 ledger row 直接落到 `manual_review`。
- LetAI topup/query 异常优先保留稳定 `external_order_id`；不得重新造第二个业务 order。
- `token_key` 缺失或 query verification mismatch，不得人工口头判成成功。

### 8.3 operator review
- 当前最小接口：
  - `POST /v1/bridge/operator/review/{bridge_deposit_id}`
- 当前 resolution 只支持：
  - `mark_resolved`
  - `close`
- 若需要“调额度后重发”“换 project 后重试”之类 richer action，必须另开任务，不在本轮 runbook 内假装支持。

## 9. 回滚边界
- 当前 bridge 是独立服务，回滚优先级：
  1. 停止新的 route 发放
  2. 停止自动 reconcile
  3. 保留状态文件与链上证据
  4. 回退 bridge-service 二进制到上一个已验证版本
- 禁止做法：
  - 手工删除 `bridge-state.json` 再“重建”
  - 清掉 `external_order_id` 强行重发
  - 把失败记录从 ledger 中直接抹掉
- 可接受做法：
  - 保留 ledger 原始记录
  - 通过 operator review 显式关闭不再处理的异常单
  - 在新的二进制版本上继续读取同一状态文件恢复处理

## 10. 证据回写
每次正式部署、首次演练或异常收口后，至少回写：
- `.pm/tasks/<TASK-UID>.execution.md`
- `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md`
- 必要时附：
  - health 快照
  - 定向测试命令与结果
  - `bridge_deposit_id -> external_order_id -> review_reason/resolution` 证据

## 11. 当前缺口
- richer operator runbook automation 还未脚本化。
- dashboard / replay / re-credit UI 仍未实现。
- 当前 runbook 仍默认 operator 手工持有 deployment 输入，不含 secret rotation / KMS / HSM 闭环。
