# NewAPI Bridge Service Operator Runbook

- 适用范围: `oasis7_newapi_bridge_service` 的最小 operator 部署。
- 目标: 持有 LetAI 管理员 key，在 `bind` 时动态创建/确保 `platform user + project + token_key`，并把结果写入共享 `bridge-state.json`，供 remote provider bridge 自动映射读取。

## 目标端点

- `GET /health`
- `GET /v1/bridge/health`
- `POST /v1/bridge/bind`
- `POST /v1/bridge/deposit-route`
- `POST /v1/bridge/reconcile`

对当前 runtime 自动映射来说，最低要求只需 `POST /v1/bridge/bind` 能成功把 `token_key` 写进 state。

## 仓库资产

- 启动脚本: `scripts/newapi-bridge-service/start-newapi-bridge-service.sh`
- 环境变量模板: `scripts/newapi-bridge-service/newapi-bridge-service.env.example`
- systemd 模板: `scripts/newapi-bridge-service/oasis7-newapi-bridge.service`

## ECS 目录建议

```text
/opt/oasis7/newapi-bridge/
  current/
  releases/<release-id>/
/etc/oasis7/newapi-bridge-service.env
/etc/oasis7/newapi-bridge/bridge-state.json
```

## 当前环境矩阵

本矩阵只描述 NewAPI bridge / remote provider bridge，不改变 hosted-login、testnet、mainnet 的环境命名。

| Lane | Host | NewAPI bridge | State | 当前验证 |
| --- | --- | --- | --- | --- |
| 测试环境 | `39.104.204.172` | `oasis7-newapi-bridge.service`，`active + enabled`，监听 `127.0.0.1:5852` | 独立测试 state: `/etc/oasis7/newapi-bridge/bridge-state.json`；可按 runbook 重置 | `GET /v1/bridge/health` 返回 `ok=true`；已通过本机 provider bridge 调用 `letai/gpt-5.4` smoke |
| 正式环境 | `39.104.205.67` | `oasis7-newapi-bridge.service`，`active + enabled`，监听 `127.0.0.1:5852` | 正式 state: `/etc/oasis7/newapi-bridge/bridge-state.json`；保留既有 active binding / project binding | `GET /v1/bridge/health` 返回 `ok=true`；已通过本机 provider bridge 调用 `letai/gpt-5.4` smoke |

两套环境当前都部署自 CI artifact `newapi-bridge-service-linux-x86_64-f628b2ab0fbb88a392d44a9d92c6b5147eaeda4f`，release 目录为 `/opt/oasis7/newapi-bridge/releases/20260529-205947-f628b2a`。

## 必需输入

- `OASIS7_NEWAPI_BRIDGE_LETAI_BASE_URL`
- `OASIS7_NEWAPI_BRIDGE_LETAI_PLATFORM_KEY`
- `OASIS7_NEWAPI_BRIDGE_STATE_PATH`

建议同时明确:

- `OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID`
- `OASIS7_NEWAPI_BRIDGE_BIND_ADDR`
- `OASIS7_NEWAPI_BRIDGE_ROUTE_TTL_SECONDS`

- `OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID` 为空时，bridge-service 不向 LetAI project upsert 请求传 `parent_channel_id`，由平台侧默认渠道策略决定 project token 归属。
- 只有当 operator 明确需要把生成的 project token 绑定到指定 LetAI channel 时，才设置 `OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID`。

## 启动后最小验证

```bash
curl -sS http://127.0.0.1:5852/v1/bridge/health

curl -sS -X POST http://127.0.0.1:5852/v1/bridge/bind \
  -H 'Content-Type: application/json' \
  -d '{"newapi_user_ref":"user-1","oasis_sender_account_id":"oc:pk:user-1"}'
```

成功后，`/etc/oasis7/newapi-bridge/bridge-state.json` 至少应出现:

- `bindings[].newapi_user_ref`
- `bindings[].platform_user_id`
- `project_bindings[].platform_project_id`
- `project_bindings[].token_key`

## 与 Remote Provider Bridge 串接

remote provider bridge 可直接读取同一份 state:

```bash
OASIS7_PROVIDER_AUTH_ROUTE_FROM_BEARER=true
OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH=/etc/oasis7/newapi-bridge/bridge-state.json
```

此时 client 直接传:

```bash
--agent-provider-auth-token newapi_user_ref:<newapi_user_ref>
```

provider bridge 会自动解析到对应的 `token_key`。
