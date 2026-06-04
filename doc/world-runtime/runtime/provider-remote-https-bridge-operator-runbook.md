# Remote HTTPS Provider Bridge Operator Runbook

- 适用范围: `agent_provider_transport=remote_https` 的 repo-owned 参考装配。
- 当前参考实现: `oasis7_provider_local_bridge` + `scripts/provider-remote-https/letai_provider_cli.py` + `nginx` HTTPS 反向代理。
- 本 runbook 不覆盖 `oasis7_newapi_bridge_service`；后者是 `OC -> LetAI quota/token_key` 额度桥，不是 runtime decision provider。

## 目标端点

对外必须暴露以下 4 个 provider contract 端点:

- `GET /v1/provider/info`
- `GET /v1/provider/health`
- `POST /v1/world-simulator/decision`
- `POST /v1/world-simulator/feedback`

repo-owned 参考装配中，这 4 个端点由 `oasis7_provider_local_bridge` 提供，公网 `https://` 入口由 `nginx` 代理到本机 `127.0.0.1:5841`。参考 nginx 模板已包含按 IP 与 `Authorization` 的限流，以及连接数限制，用来降低单个 bearer 快速打空 LetAI quota 的风险。

## 当前环境矩阵

本矩阵只描述 remote provider bridge 的 runtime decision provider 口径；NewAPI bridge 自身是额度/token state 服务。

| Lane | Host | Provider bridge | 对外入口 | State 来源 | 当前 live gate |
| --- | --- | --- | --- | --- | --- |
| 测试环境 | `39.104.204.172` | `oasis7-remote-provider-bridge.service`，监听 `127.0.0.1:5841` | 暂无公网域名；本机/内网验证优先 | 本机 `/etc/oasis7/newapi-bridge/bridge-state.json` | 2026-06-04 live gate: state realigned to the same NewAPI project/token as 205; loopback decision passes with `provider_version=letai/gpt-5.4` |
| 正式环境 | `39.104.205.67` | `oasis7-remote-provider-bridge.service`，监听 `127.0.0.1:5841` | `https://t2t.oasis7.tech` | 本机 `/etc/oasis7/newapi-bridge/bridge-state.json` | 2026-06-04 live gate: loopback decision passes with `provider_version=letai/gpt-5.4`; current Codex control host to public TLS is reset before nginx, while 204-to-205 public TLS/HTTP succeeds |

两套 provider bridge 当前都启用自动映射模式：client 传 `newapi_user_ref:<user_ref>` 或 `bridge_user_id:<id>`，provider bridge 从本机 NewAPI bridge state 解析对应 `token_key`。`GET /v1/provider/health` 可能因上游 health URL 返回 `HTTP 401` 而显示 `degraded`；是否能调用模型以 `POST /v1/world-simulator/decision` smoke 为准。

## 仓库资产

- Wrapper: `scripts/provider-remote-https/letai_provider_cli.py`
- Per-user route manager: `scripts/provider-remote-https/manage-user-route.py`
- Bridge 启动脚本: `scripts/provider-remote-https/start-remote-provider-bridge.sh`
- 环境变量模板: `scripts/provider-remote-https/remote-provider-bridge.env.example`
- systemd 模板: `scripts/provider-remote-https/oasis7-remote-provider-bridge.service`
- nginx 模板: `scripts/provider-remote-https/t2t.oasis7.tech.nginx.conf`

systemd 模板默认使用低权限专用用户:

- remote provider bridge: `oasis7-provider`
- newapi bridge: `oasis7-newapi`

部署时需要先创建对应 user/group，并把 env/state 文件权限授权给这些用户。

## ECS 目录建议

```text
/opt/oasis7/remote-provider-bridge/
  current/                      # 指向当前发布目录
  releases/<release-id>/        # 每次上传一份独立 release
/etc/oasis7/remote-provider-bridge.env
/etc/nginx/ssl/t2t.oasis7.tech.pem
/etc/nginx/ssl/t2t.oasis7.tech.key
```

## 必需输入

- `OASIS7_REMOTE_LLM_API_KEY`
- `OASIS7_REMOTE_LLM_MODEL`
- `OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN`

推荐同时明确:

- `OASIS7_REMOTE_LLM_BASE_URL`
- `OASIS7_REMOTE_LLM_HEALTH_URL`
- `OASIS7_REMOTE_LLM_TIMEOUT_MS`
- `OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS`
- `OASIS7_REMOTE_LLM_USER_AGENT`

`OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN` 建议至少用:

```bash
openssl rand -hex 32
```

## Per-User Token Key Mode

若要做到“每个用户一个 `token_key`”，当前参考装配支持按 `bridge bearer token -> route label -> upstream token_key/model/base_url` 路由。

还支持自动映射模式：`bridge bearer token -> newapi bridge state -> token_key`。这里 bearer token 直接承载用户标识，不再需要单独维护 auth-routes / llm-routes JSON。

需要两份 JSON:

- `/etc/oasis7/remote-provider-auth-routes.json`
  - `bridge bearer token -> route label`
- `/etc/oasis7/remote-provider-llm-routes.json`
  - `route label -> { api_key, model, base_url }`

示例模板已预铺在 ECS:

- `/etc/oasis7/remote-provider-auth-routes.json.example`
- `/etc/oasis7/remote-provider-llm-routes.json.example`

启用 per-user mode 时，正式 env 需要至少新增:

```bash
OASIS7_PROVIDER_AUTH_ROUTE_MAP_PATH=/etc/oasis7/remote-provider-auth-routes.json
OASIS7_REMOTE_LLM_ROUTES_PATH=/etc/oasis7/remote-provider-llm-routes.json
```

此时可不再使用单一 `OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN` / `OASIS7_REMOTE_LLM_API_KEY` 作为唯一真值。

### 自动映射模式

若 `oasis7_newapi_bridge_service` 已在持久化 state 里维护 `newapi_user_ref -> bridge_user_id -> token_key`，可以直接启用:

```bash
OASIS7_PROVIDER_AUTH_ROUTE_FROM_BEARER=true
OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH=/path/to/newapi-bridge/bridge-state.json
```

此时:

- Client 传 `--agent-provider-auth-token newapi_user_ref:<user_ref>`，bridge 会自动按 `newapi_user_ref` 找 `token_key`
- 或传 `--agent-provider-auth-token bridge_user_id:<id>`，bridge 会按 `bridge_user_id` 查找
- 不再需要 `/etc/oasis7/remote-provider-auth-routes.json`
- 不再需要 `/etc/oasis7/remote-provider-llm-routes.json`

适用前提:

- `oasis7_newapi_bridge_service` 的 state 文件能被 remote provider bridge 读取
- state 内对应 binding 为 `active`
- 对应 `project_bindings` 已写入真实 `token_key`
- 对于全链路测试用户，`newapi_user_ref` 推荐使用链上公钥 fingerprint，例如 `pk_<public-key-fingerprint>`；client bearer 使用 `newapi_user_ref:pk_<public-key-fingerprint>`。测试邮箱只作为 QA / hosted-login persona 标签，不作为 provider bearer 主选择器。

推荐不要手改 JSON，直接用脚本给用户签发 bridge token:

```bash
python3 scripts/provider-remote-https/manage-user-route.py upsert-user \
  --auth-routes /etc/oasis7/remote-provider-auth-routes.json \
  --llm-routes /etc/oasis7/remote-provider-llm-routes.json \
  --user alice \
  --api-key <alice-letai-token-key> \
  --model gpt-5.4
```

脚本行为:

- 若该用户还没有 bridge token，会自动生成一个随机 bearer token。
- 若该用户已存在 bridge token，默认沿用原 token，只更新上游 `api_key/model/base_url`。
- 若需要轮换 bridge token，加 `--rotate-bridge-token`。
- 运行成功后会输出该用户应使用的 bridge bearer token 和一条 `curl` 样例。

## 本机验证

在 ECS 上准备好环境变量文件后，至少执行:

```bash
python3 /opt/oasis7/remote-provider-bridge/current/scripts/provider-remote-https/letai_provider_cli.py \
  agent --agent letai --message '{"decision":"wait"}' --timeout 15 --json

/opt/oasis7/remote-provider-bridge/current/scripts/provider-remote-https/start-remote-provider-bridge.sh
```

另开终端验证 provider 合同:

```bash
curl -sS -H "Authorization: Bearer $OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN" \
  http://127.0.0.1:5841/v1/provider/info

curl -sS -H "Authorization: Bearer $OASIS7_PROVIDER_BRIDGE_AUTH_TOKEN" \
  http://127.0.0.1:5841/v1/provider/health
```

per-user mode 下，可先签发一个测试用户，再用脚本输出的 bearer token 验证:

```bash
python3 /opt/oasis7/remote-provider-bridge/current/scripts/provider-remote-https/manage-user-route.py upsert-user \
  --auth-routes /etc/oasis7/remote-provider-auth-routes.json \
  --llm-routes /etc/oasis7/remote-provider-llm-routes.json \
  --user alice \
  --api-key <alice-letai-token-key> \
  --model gpt-5.4

curl -sS -H "Authorization: Bearer <alice-bridge-token>" \
  http://127.0.0.1:5841/v1/provider/info
```

## nginx 验证

真实环境门禁优先使用 live gate。该脚本会通过 SSH 读取 204/205 真实
`/etc/oasis7/newapi-bridge/bridge-state.json`，派生 active
`newapi_user_ref:<ref>` selector，然后分别打真实 ECS loopback provider 和 205
公网 nginx ingress。它不打印 raw `token_key`、密码或私钥。

```bash
./scripts/provider-remote-https/provider-bridge-live-gate.sh --decision-count 1
./scripts/provider-remote-https/provider-bridge-live-gate.sh --decision-count 1 --loopback-target 204 --skip-public
./scripts/provider-remote-https/provider-bridge-live-gate.sh --decision-count 1 --loopback-target 205 --skip-public
./scripts/provider-remote-https/provider-bridge-live-gate.sh --decision-count 1 --skip-loopback
```

长跑额度消耗/低额度验证也必须走真实环境，使用 dedicated lowquota persona 后再执行:

```bash
./scripts/provider-remote-https/provider-bridge-live-gate.sh \
  --skip-public \
  --loopback-target 204 \
  --lowquota-target 204 \
  --lowquota-decision-count 20

./scripts/provider-remote-https/provider-bridge-live-gate.sh \
  --skip-loopback \
  --lowquota-target public205 \
  --lowquota-decision-count 20
```

默认 CI 只跑无副作用的 provider contract 回归；真实环境门禁需要凭据和公网可达性，
用 `OASIS7_CI_RUN_PROVIDER_LIVE_GATE=true ./scripts/ci-tests.sh --required` 显式启用。
mock 测试只用于保护 harness 解析和失败判定，不作为环境通过证据。

公网入口验证:

```bash
curl -sS -H "Authorization: Bearer <token>" https://t2t.oasis7.tech/v1/provider/info
curl -sS -H "Authorization: Bearer <token>" https://t2t.oasis7.tech/v1/provider/health
```

自动化合同 smoke:

```bash
./scripts/provider-remote-https/provider-bridge-contract-smoke.sh \
  --base-url https://t2t.oasis7.tech \
  --auth-token <token> \
  --decision-count 1 \
  --min-successes 1
```

该脚本同时验证 `/v1/provider/info`、`/v1/provider/health` 与
`POST /v1/world-simulator/decision`。默认允许 health 返回 `degraded`，因为上游
health URL 可能返回 `HTTP 401`；是否能调用模型仍以 decision smoke 为准。若需要把
health 也作为硬门禁，追加 `--require-health-ok`。

若 LetAI 上游对默认 `oasis7-letai-provider-cli/1.0` User-Agent 做了额外拦截，可在 env 里显式覆写:

```bash
OASIS7_REMOTE_LLM_USER_AGENT=curl/8.5.0
```

## Runtime 接入

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_game_launcher -- \
  --scenario llm_bootstrap \
  --with-llm \
  --agent-provider-mode provider_backed \
  --agent-provider-backend provider_local_bridge \
  --agent-provider-contract worldsim_provider_v1 \
  --agent-provider-transport remote_https \
  --agent-provider-url https://t2t.oasis7.tech \
  --agent-provider-auth-token <token> \
  --agent-provider-connect-timeout-ms 15000 \
  --agent-provider-profile oasis7_p0_low_freq_npc \
  --agent-execution-lane player_parity
```

## 当前已知边界

- 该参考装配复用了 `oasis7_provider_local_bridge` 的 provider contract 和 prompt/decision parser，因此 `provider_id` 仍会表现为 `provider_local_bridge`。
- `provider_health` 默认通过 `OASIS7_REMOTE_LLM_HEALTH_URL` 做 upstream 探针；若 LetAI/兼容服务不支持该路径，需要显式改成可用的健康检查地址。
- 在未提供真实 LetAI `api_key/model` 前，ECS 端只能部署模板和二进制，不能完成最终 smoke。
- 自动映射模式当前只接受 `newapi_user_ref:<user_ref>` / `bridge_user_id:<id>` 这类显式 bearer selector；不再接受裸 `newapi_user_ref`，避免把可猜测的短字符串直接当作公网 bridge token。
