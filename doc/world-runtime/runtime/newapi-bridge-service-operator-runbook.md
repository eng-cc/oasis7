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

## 必需输入

- `OASIS7_NEWAPI_BRIDGE_LETAI_BASE_URL`
- `OASIS7_NEWAPI_BRIDGE_LETAI_PLATFORM_KEY`
- `OASIS7_NEWAPI_BRIDGE_STATE_PATH`

推荐同时明确:

- `OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID`
- `OASIS7_NEWAPI_BRIDGE_BIND_ADDR`
- `OASIS7_NEWAPI_BRIDGE_ROUTE_TTL_SECONDS`

生产注意:

- 当前 LetAI 生产环境若省略 `OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID`，动态创建出的 project token 可能默认落到已废弃分组 `cc`，随后对 `https://api.letai.run/v1/models` / `POST /v1/chat/completions` 会返回 `HTTP 403` 与 `分组 cc 已被弃用`。
- 因此在真实 ECS 部署里，`OASIS7_NEWAPI_BRIDGE_LETAI_PARENT_CHANNEL_ID` 应视为必填项，而不是可选优化项。

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
--agent-provider-auth-token <newapi_user_ref>
```

provider bridge 会自动解析到对应的 `token_key`。
