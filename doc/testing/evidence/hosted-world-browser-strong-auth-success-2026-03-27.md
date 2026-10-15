# hosted world 浏览器 strong-auth 成功证据（2026-03-27）

脱敏说明：历史命令中的具体私钥值已替换为 `<REDACTED_PRIVATE_KEY>`；该命令因此不再是逐字原件。候选、公开身份、运行结果和失败边界保持原记录。Git 历史仍可能包含原值，本次脱敏不表示密钥已撤销、已轮换或已安全；不得将此历史命令当作当前凭据配置示例。

审计轮次: 1

## Meta
- 关联专题: `PRD-P2P-023-B/C/D/E`
- 关联任务: `TASK-P2P-041-B/C/D/E`
- 责任角色: `qa_engineer`
- 协作角色: `runtime_engineer`, `viewer_engineer`
- 当前结论: `pass`
- 目标: 固化真实 hosted public join 栈下，`player_session + backend strong-auth grant` 已可成功放行 `prompt_control preview/apply`，且不再复现 `release_token does not map to an active player slot`。

## 最终结论
- `release_token` 生命周期竞争已在真实 hosted 栈上关闭。
- 使用真实匹配的 hosted strong-auth signer、`--with-llm`、浏览器本地临时 key 和 backend approval code 时，`prompt_control preview` 与 `prompt_control apply` 都能成功拿到:
  - `strongAuthLastGrantError = null`
  - `strongAuthLastGrantActionId = prompt_control_preview` / `prompt_control_apply`
  - `lastPromptFeedback.stage = preview_ack` / `apply_ack`
  - `lastPromptFeedback.ok = true`
- runtime live snapshot 同时确认当前浏览器玩家已稳定绑定到 `agent-1`:
  - `agent_player_bindings.agent-1 = hosted-player-0000019d2d1e0d35-00000002`
- 本次实测还确认了一个环境前提:
  - 默认 `llm_bootstrap` 场景里，`agent-0` 已被历史 player 绑定，因此首轮若选 `agent-0` 会先命中 `player_bind_failed`
  - 改用未绑定的 `agent-1` 后，真实 strong-auth success path 可稳定通过

## 修复对应关系
- 这次通过依赖两层修复同时生效:
  - `HostedPlayerSessionIssuer::observe_runtime_active_players()` 会先用当前 runtime probe snapshot 给仍在 runtime 中活跃的 active slot 续租，再做过期清理，避免 probe 当轮可见的 active slot 仍被旧 `last_seen` 提前剪掉
  - public player plane 现在会在 `admission/issue/refresh/release/strong-auth grant` 前同步 probe 一次 `live_bind`，把“刚完成 register、但后台 presence monitor 还没来得及看到”的 runtime binding 先收敛到 issuer，再执行 `refresh/grant`

## 执行命令
- 启动真实 hosted 栈:
  - `env OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY=48f1d2d8b67b270dbeacec50399b044bfd772ea3cdfabc29f62850bb030d84a3 OASIS7_HOSTED_STRONG_AUTH_PRIVATE_KEY=<REDACTED_PRIVATE_KEY> OASIS7_HOSTED_STRONG_AUTH_APPROVAL_CODE=preview-code env -u RUSTC_WRAPPER cargo run -q -p oasis7 --bin oasis7_game_launcher -- --deployment-mode hosted_public_join --with-llm --viewer-static-dir crates/oasis7_viewer --viewer-host 127.0.0.1 --viewer-port 6101 --web-bind 127.0.0.1:6102 --live-bind 127.0.0.1:6103 --chain-disable --no-open-browser`
- 浏览器动作:
  - `agent-browser --session hosted-prompt-rerun2 open 'http://127.0.0.1:6101/?ws=ws%3A%2F%2F127.0.0.1%3A6102&hosted_access=...'`
  - `source scripts/agent-browser-lib.sh && ab_eval hosted-prompt-rerun2 'window.__AW_TEST__.setStrongAuthApprovalCode("preview-code")'`
  - `source scripts/agent-browser-lib.sh && ab_eval hosted-prompt-rerun2 'window.__AW_TEST__.sendPromptControl("preview", { agentId: "agent-1", shortTermGoal: "Verify hosted strong auth after route-side runtime reconcile" })'`
  - `source scripts/agent-browser-lib.sh && ab_eval hosted-prompt-rerun2 'window.__AW_TEST__.sendPromptControl("apply", { agentId: "agent-1", shortTermGoal: "Hosted strong-auth apply success after route-side runtime reconcile" })'`
  - `source scripts/agent-browser-lib.sh && ab_eval hosted-prompt-rerun2 'window.__AW_TEST__.getState()'`
- runtime 绑定侧佐证:
  - `python3 - <<'PY' ... request_snapshot ... print(snapshot["model"]["agent_player_bindings"]) ... PY`

## 浏览器证据
### 1. prompt preview
- `lastPromptFeedback`:
  - `action = prompt_preview`
  - `stage = preview_ack`
  - `ok = true`
  - `response.operation = apply`
  - `response.preview = true`
  - `response.version = 1`
- strong-auth:
  - `strongAuthLastGrantActionId = prompt_control_preview`
  - `strongAuthLastGrantError = null`

### 2. prompt apply
- `lastPromptFeedback`:
  - `action = prompt_apply`
  - `stage = apply_ack`
  - `ok = true`
  - `response.operation = apply`
  - `response.preview = false`
  - `response.version = 1`
- strong-auth:
  - `strongAuthLastGrantActionId = prompt_control_apply`
  - `strongAuthLastGrantError = null`

### 3. session / admission
- `authRegistrationStatus = registered`
- `authRuntimeStatus = registered`
- `authBoundAgentId = agent-1`
- `hostedAdmission.active_player_sessions = 1`
- `hostedAdmission.runtime_bound_player_sessions = 2`
- `hostedAdmission.runtime_only_player_sessions = 1`
- `hostedAdmission.released_players_total = 1`

## runtime snapshot 佐证
- `agent_player_bindings` 最终为:
  - `agent-0 -> hosted-player-0000019d2cf0e763-00000001`
  - `agent-1 -> hosted-player-0000019d2d1e0d35-00000002`

## 风险与剩余项
- 本证据仅证明 preview-grade backend reauth 已可真实放行 hosted `prompt_control preview/apply`。
- `main_token_transfer` 仍保持 `blocked_until_strong_auth`，不应被本次通过误读为 hosted 资产动作已开放。
- 后端 signer 仍是 env 托管 + approval code；production signer custody / dedicated asset-governance strong-auth lane 仍待后续专题收口。
