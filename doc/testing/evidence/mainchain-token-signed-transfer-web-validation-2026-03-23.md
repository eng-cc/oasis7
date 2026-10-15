# 主链 Token Web 签名转账验证证据（2026-03-23）

脱敏说明：历史命令中的具体私钥值已替换为 `<REDACTED_PRIVATE_KEY>`；该命令因此不再是逐字原件。候选、公开身份、运行结果和失败边界保持原记录。Git 历史仍可能包含原值，本次脱敏不表示密钥已撤销、已轮换或已安全；不得将此历史命令当作当前凭据配置示例。

审计轮次: 1

## Meta
- 关联专题: `PRD-P2P-TXAUTH-001/002/003`
- 关联任务: `STRAUTH-3B`
- 责任角色: `qa_engineer`
- 协作角色: `viewer_engineer`
- 当前结论: `pass`
- 目标: 用 Web-first / agent-browser 证据确认 wasm 转账窗口已经能在页面侧产出合法 signed request，并对 signer/bootstrap 缺失给出本地失败提示。

## 最终结论
- `viewer_engineer` 先修复了三个真实 Web blocker：
  - wasm `SystemTime` 平台 panic
  - wasm `process::id()` 平台 panic
  - wasm transfer canonical JSON 字节顺序与 runtime helper 不一致，导致 `invalid_signature`
- 修复后，`qa_engineer` 通过 canvas test hook + agent-browser 完成两条 required/full 证据：
  - 成功路径：页面侧 signed transfer submit 已到达 runtime，返回 `action_id=1`，后续 tracked status=`confirmed`
  - 失败路径：删除 `__OASIS7_VIEWER_AUTH_ENV` 后，页面侧直接提示 `转账签名失败: viewer auth bootstrap is unavailable`，且没有发出 transfer POST

## 执行命令
- 构建与定向验证:
  - `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_auth -- --nocapture`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher`
  - `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`
  - `env -u NO_COLOR trunk build --dist /tmp/oasis7-strauth3b-web-launcher-4`
- Web launcher:
  - 带 signer bootstrap 成功路径:
    - `env OASIS7_VIEWER_AUTH_PUBLIC_KEY=fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3 OASIS7_VIEWER_AUTH_PRIVATE_KEY=<REDACTED_PRIVATE_KEY> target/debug/oasis7_web_launcher --listen-bind 127.0.0.1:5410 --console-static-dir /tmp/oasis7-strauth3b-web-launcher-4 --chain-node-id qa-strauth-3b-hook --chain-status-bind 127.0.0.1:5121 --no-open-browser`
  - 无 env 复验:
    - `target/debug/oasis7_web_launcher --listen-bind 127.0.0.1:5410 --console-static-dir /tmp/oasis7-strauth3b-web-launcher-4 --chain-node-id qa-strauth-3b-noauth --chain-status-bind 127.0.0.1:5121 --no-open-browser`
- 页面侧自动化:
  - `agent-browser --session strauth3b reload`
  - 通过 `window.__OASIS7_LAUNCHER_TEST_QUEUE` 注入:
    - `open_transfer_window`
    - `set_transfer_nonce_mode(manual)`
    - `set_transfer_draft(...)`
    - `submit_transfer`
  - 通过 `window.__OASIS7_LAUNCHER_TEST_STATE` 读取:
    - `transfer_submit_state`
    - `tracked_action_id`
    - `tracked_status`
  - 通过页面级 fetch capture 记录真实 `/api/chain/transfer` method/body

## 成功路径证据
- 页面状态:
  - `chain_runtime_status=已就绪`
  - `transfer_submit_state.kind=success`
  - `transfer_submit_state.message=转账请求已提交: action_id=1, submitted_at=1774268949126`
  - `tracked_action_id=1`
  - `tracked_status.status=confirmed`
- 页面真实 POST body 已抓取，包含：
  - `from_account_id=awt:pk:fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3`
  - `nonce=7`
  - `public_key=fded5085f1e8099257b7bfb2346eb6bd4194c3351d8f97686b18cfcc5969e0a3`
  - `signature=awttransferauth:v1:6378392f...`
- 直接链上查询同时证明当前 local world 仍无预置账户:
  - `GET /v1/chain/transfer/accounts -> ok=true, accounts=[]`
- QA 结论:
  - 本轮成功标准不是“有余额并完成资金转移”，而是“页面侧签名真实到达 runtime submit，并能拿到结构化 lifecycle 记录”
  - 该标准已满足

## 失败路径证据
- 仅去掉 launcher env 不足以构成失败路径：
  - `oasis7_web_launcher` 会从本地 `config.toml` 回退注入 bootstrap
- 因此本轮采用更精确的页面级 bootstrap 缺失验证：
  - 在浏览器中显式 `delete window.__OASIS7_VIEWER_AUTH_ENV`
- 结果:
  - `transfer_submit_state.kind=failed`
  - `transfer_submit_state.message=转账签名失败: viewer auth bootstrap is unavailable`
  - 页面 fetch capture 中没有 `/api/chain/transfer` 的 POST，请求仅剩 accounts 轮询
- QA 结论:
  - signer/bootstrap 缺失会在页面侧被阻断，不会降级成裸 transfer submit

## 风险与剩余项
- 当前 `accounts=[]`，说明 local world 还没有预置可见主链账户；本证据验证的是 submit/auth path，不是早期经济状态可玩性。
- `oasis7_web_launcher` 当前仍会从 `config.toml` 回退注入 signer bootstrap；若后续需要“无 signer 环境”的端到端部署验证，应另开专题把 launcher fallback 策略与 QA 场景矩阵写清。
- 本轮完成不改变整体安全 verdict；治理 ceremony / external signer / production keystore 仍未完成。
