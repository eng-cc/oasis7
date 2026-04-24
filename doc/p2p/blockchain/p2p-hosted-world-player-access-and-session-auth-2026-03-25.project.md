# oasis7 hosted world 玩家访问与会话鉴权（项目管理文档）

- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.design.md`
- 对应需求文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`

审计轮次: 1
## 任务拆解（含 PRD-ID 映射）
- [x] TASK-P2P-041-A (PRD-P2P-023-A/B/C) [test_tier_required + test_tier_full]: `runtime_engineer` 拆分 public player plane 与 private control plane，冻结 endpoint taxonomy、`/api/gui-agent/action` split 策略、hosted verdict 与 admission control，并移除 hosted-world 公共路径中的浏览器长期 signer bootstrap。
- [x] TASK-P2P-041-B (PRD-P2P-023-B/D) [test_tier_required + test_tier_full]: `viewer_engineer` 落地 `guest session -> player session` 网页 join/login/reconnect UX，并按 capability 禁用敏感动作。
- [x] TASK-P2P-041-C (PRD-P2P-023-B/C) [test_tier_required + test_tier_full]: `runtime_engineer` + `agent_engineer` 落地 session 验证、`player_id -> entity` 绑定、resume/revoke 与 ownership 冲突处理。
- [x] TASK-P2P-041-D (PRD-P2P-023-B/D) [test_tier_required + test_tier_full]: `runtime_engineer` + `viewer_engineer` 落地 `strong auth` 升级链路，覆盖 `main token transfer` 与敏感 prompt/control 动作。
- [x] TASK-P2P-041-E (PRD-P2P-023-C/E) [test_tier_required + test_tier_full]: `qa_engineer` 建立 hosted-world abuse suite，覆盖 replay、expired session、revocation、operator/public URL 混淆、admission limit 和 capability bypass。
- [x] TASK-P2P-041-F (PRD-P2P-023-E) [test_tier_required]: `liveops_community` 建立 hosted operator runbook、分享规范、incident/rotation 流程与 claims boundary。

## 角色拆解
### TASK-P2P-041-A / runtime_engineer
- 输入:
  - `crates/oasis7/src/bin/oasis7_web_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/server.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/static_http.rs`
  - `crates/oasis7/src/bin/oasis7_hosted_access.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs`
- 输出:
  - public/private plane endpoint 清单
  - `/api/gui-agent/action` split 方案
  - join admission control 最小契约
  - hosted-world browser signer bootstrap 退场方案
  - required/full 回归入口
- 完成定义:
  - public join 路径不再依赖长期私钥 bootstrap
  - world/control 接口不再作为 public player origin 默认可达面
  - `/api/gui-agent/action` 未拆分前保持 private，拆分后才允许 player-safe 子集进入 public player plane
  - public join 有显式 session issuance / full-world / rate-limit 规则

### TASK-P2P-041-B / viewer_engineer
- 输入:
  - `crates/oasis7_viewer/src/egui_right_panel_chat_auth.rs`
  - `crates/oasis7_viewer/src/viewer_automation.rs`
  - `crates/oasis7_client_launcher/src/transfer_auth.rs`
  - `crates/oasis7_viewer/software_safe.js`
- 输出:
  - join/login/reconnect UX
  - capability-based button state
  - hosted-world 网页错误文案
- 完成定义:
  - guest/player/strong-auth 三档在 UI 明确可见
  - 没有能力时按钮禁用且错误可读

### TASK-P2P-041-C / runtime_engineer + agent_engineer
- 输入:
  - TASK-P2P-041-A endpoint/signer/admission 边界
  - TASK-P2P-041-B 会话与能力模型
- 输出:
  - session validation
  - entity bind/resume/revoke
  - ownership 冲突规则
- 完成定义:
  - 同一玩家实体 ownership 可验证
  - 断线恢复和撤销不会穿透到其他玩家实体

### TASK-P2P-041-D / runtime_engineer + viewer_engineer
- 输入:
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
- 输出:
  - hosted-world strong-auth action list
  - challenge/proof/verification 路径
  - Web sensitive-action regression
- 完成定义:
  - `main token transfer` 不再通过浏览器长期私钥默认签名
  - prompt/control 类高风险动作必须明确走强鉴权或 private plane

### TASK-P2P-041-E / qa_engineer
- 输入:
  - TASK-P2P-041-A~D 的平面、session、strong-auth 设计
- 输出:
  - abuse suite
  - failure signature
  - block/pass 判定模板
- 完成定义:
  - replay / revoke / expiry / capability bypass / admission limit 有 required/full 证据

### TASK-P2P-041-F / liveops_community
- 输入:
  - TASK-P2P-041-A~E 结论
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- 输出:
  - hosted operator runbook
  - incident/rotation/public claims 模板
  - 分享 URL 规范
- 完成定义:
  - hosted world 分享、误分享、撤销和事故通报均有 runbook

## 当前结论
- 当前阶段:
  - 游戏阶段口径: `limited playable technical preview`
  - 安全阶段口径: `crypto-hardened preview`
  - hosted-world player access verdict: `specified_not_implemented`
- 已实现的 `TASK-P2P-041-A` P0 收口:
  - `oasis7_game_launcher --deployment-mode hosted_public_join` 会停止向公开 viewer HTML 注入长期 signer bootstrap。
  - `oasis7_web_launcher --deployment-mode hosted_public_join` 会把 `/api/state`、`/api/start`、`/api/stop`、`/api/chain/start`、`/api/chain/stop`、`/api/gui-agent/*` 与 console static 路径收口为 loopback-only private control plane。
  - 新增 `/api/public/state`，对外只暴露 join 级 public snapshot，不再把 operator state / logs / config 作为默认公共面。
  - launcher snapshot 现已冻结 `deployment_mode`、hosted verdict、`gui-agent` surface 状态与 admission contract 默认值，供后续 viewer/runtime/QA 接续。
- 已完成的 `TASK-P2P-041-B` viewer UX 收口:
  - `software_safe.js` 现会显式显示 `guest_session / player_session / strong_auth` 梯度、`deploymentHint`、`auth source` 与 reconnect 提示，不再只显示 `auth=ready|missing`。
  - prompt/chat 现在会按 capability 给出结构化禁用原因：至少区分 `guest_session`、`observer_only` 与 `strong_auth_required` 占位，而不是继续用单一 “viewer auth bootstrap is unavailable”。
  - `__AW_TEST__.getState()` 已补 `authTier`、`authSource`、`authDeploymentHint` 与 `authSurface`，便于后续 QA/agent-browser 对 hosted public join 的 session/capability 状态做证据采样。
  - `software_safe.js` 现在会在 `hosted_public_join` 下优先尝试 `GET /api/public/player-session/issue`，拿到 server-issued `player_id` 后由浏览器本地生成/持久化临时 Ed25519 key，并在 page reload 后复用这份本地会话态。
  - viewer 现已消费 `authoritative_recovery register_session/reconnect_sync` ack/error：首次 issue 后会注册 player session，刷新或断线重连后会先走 `reconnect_sync`，若 runtime 返回 `session_not_found` 则回退到显式重新注册。
  - `software_safe.js` 现已把 `session_register` 浏览器签名 payload 中的 `force_rebind` 固定编码为布尔字段，即使值为 `false` 也不再省略；这修正了 hosted public join 首次注册时 `verify auth signature failed` 的浏览器/Rust CBOR 载荷不一致。
  - viewer 现已提供显式 `Release Hosted Player Session` 动作：会向 runtime 发送 `revoke_session`，并向同源 public player plane 发送 `/api/public/player-session/release` 归还 active slot，然后清掉浏览器本地持久化的 hosted player session。
  - viewer 现已可直接读取 `/api/public/player-session/admission`，并在 guest lane 显示当前 `activeSlots/issueBudget`；若先前因为 `world_full/rate_limited` 留在 guest，也可以通过显式 `Acquire Hosted Player Session` 动作重试，不必靠发送 chat/prompt 侧向触发。
  - admission snapshot 现还会回出最近一次 runtime probe 看到的 `runtime_bound_player_sessions`；viewer summary 会并排显示 `activeSlots` 和 `runtimeBound`，便于 QA 区分“issuer 占位”和“runtime 真正还绑着几个玩家”。
  - admission snapshot 现还会回出 `runtime_probe_status/runtime_probe_error/last_runtime_probe_unix_ms`；viewer summary 可直接看 runtime probe 当前是 `ok`、`error` 还是尚未启动，不必只靠外部日志猜测 public player plane 是否还在对账。
  - viewer 现已在 hosted player session 注册成功后自动调用 `/api/public/player-session/refresh` 并启动 lease heartbeat；public admission 也会暴露 `slot_lease_ttl_ms`，让 stale slot 可被自动回收，而不是无限占位。
  - viewer 的 lease heartbeat 现会同时发送轻量 `reconnect_sync` 探针；即便玩家空闲不发 chat/prompt，也能周期性发现 runtime 侧的 `session_revoked/session_not_found`，不再只能等下次主动交互才发现被踢/被撤销。
  - viewer summary 与 `__AW_TEST__.getState()` 现已显式暴露 `authRuntimeStatus/authBoundAgentId` 与 recovery error；WebSocket 断线时也会清掉挂起的 `syncInFlight` 并标记 `runtime=disconnected`，避免 `reconnect_sync` 在短断线后卡死不再自动恢复。
  - viewer 现会记住最近一次 `requested_agent_id`；若 runtime 对 `register_session` 返回 `player_bind_failed: explicit rebind required`，会自动携带 `force_rebind=true` 做一次受控重试，而不是只把玩家留在报错态。
  - `__AW_TEST__.getState()` 与 viewer summary 现还会显式暴露 `pendingRequestedAgentId/pendingForceRebind`，便于 QA 直接确认 hosted viewer 是否真的进入了 rebind 重试，而不是只靠日志推断。
  - viewer 的语义动作现在会等待 `session_registered` ack 后再继续发送 chat/prompt；若首个 `register_session` 因 explicit rebind 被拒，当前动作会留在队列里，等 `force_rebind` 成功后继续，而不是要求玩家手动再点一次。
  - viewer summary 现会在 rebind 期间显式显示 `rebind target/mode` 与“当前动作会在注册成功后继续”的提示，让 same-player explicit rebind 不再只是后台自动恢复。
  - viewer 现还会在 rebind 成功后保留一条人类可读的完成提示；`__AW_TEST__.getState()` 也会同步暴露 `authRebindNotice`，避免提示只在进行中可见、成功后立即消失。
- 已完成的 `TASK-P2P-041-C` session/bind 收口:
  - runtime-live 新增显式 `session_register`，并要求 prompt/chat/gameplay 在 player action 之前先完成 session 注册；原先“第一个签名动作自动注册 active key”的隐式登录已收口。
  - `RuntimeSessionPolicy::validate_known_session_key` 现会在未注册 session 时返回 `session_not_found`，不再把未注册玩家默认为 epoch 0 放行。
  - runtime 现额外维护 `player_id -> agent_id` 单实体占用真值；同一 player 默认不能静默切到第二个 agent，必须走签名保护的显式 `force_rebind`。
  - `ReconnectSync` / `SessionRegistered` / `SessionRotated` ack 已带回当前 `agent_id`，`RevokeSession` 会清掉该 player 的绑定与 nonce/replay 痕迹，保持“撤销即失效、需重新注册”的 hosted v1 语义。
  - public player plane 的 `refresh/release` 现已收紧为 `player_id + release_token` 双绑定校验，并补充 `player_id_required/player_id_mismatch` 单测，避免 token-only 误归还或误续租其他玩家 slot。
  - `oasis7_game_launcher` 的 public player plane 现在会启动独立 runtime presence monitor 线程，按固定间隔短连 `live_bind` 拉取 runtime snapshot，对账后立即释放连接；同时 `oasis7_viewer_live` 也已补成可接受 browser/web-bridge 长连与 probe 并发访问，不再出现“浏览器一打开，runtime probe 就在 hello_ack 超时”的新型抢占。凡是“曾经已在 runtime 里出现过、现在又从 runtime binding 中消失”的 player slot，会被 issuer 立即回收，并对旧 browser session 返回 `session_revoked`，让 operator kick / remote revoke 能稳定回流到 `world_full` 判定。
  - `ViewerWebBridge::run()` 现已从串行 `accept -> serve_stream` 改成“每个 websocket 一条独立桥接线程”；这补掉了 browser/web-bridge 层最后一个单连接瓶颈。真实本地 hosted 栈上的双 `agent-browser` session 现可同时收敛到 `debug_viewer:subscribed` 并看到同一份 snapshot，不再出现“第二个网页因为第一个还开着就永久 detached”的旧行为。
  - runtime revoke 路径现在会额外发出 `AgentPlayerUnbound` 虚拟事件；当前 hosted presence monitor 仍主要靠下一轮短连 snapshot 收敛 active 集合，但这条事件真值已为后续更低延迟的 presence bus 预留好协议基础。
  - runtime-live 的 player binding 现在会在“同一 agent 从旧 player 改绑到新 player”时显式发出 `AgentPlayerUnbound(old) -> AgentPlayerBound(new)` 事件序列，而不是只发新 `Bound`；当前 hosted reconcile 仍以周期性 snapshot 为准，但后续 rebind/operator handoff 若升级到更实时的 presence 通道，不必再重做解绑语义。
  - runtime-live 的 `session_register` 现已把 `force_rebind` 纳入 auth payload；只有当玩家对 `requested_agent_id + force_rebind` 一并签名时，runtime 才会解绑旧 agent 并把同一 `player_id` 改绑到新 agent，避免把实体切换继续留在未落地设计里。
  - `viewer::auth` 现已补 `session_register` 定向回归，要求篡改 `requested_agent_id` 或 `force_rebind` 都会触发签名校验失败，避免 explicit rebind 退化成“字段在协议里存在，但不在签名约束里”。
  - hosted admission 的 `world_full` 现不再只看 issuer active slot：`HostedPlayerSessionAdmissionSnapshot` 新增 `effective_player_sessions/runtime_only_player_sessions`，会把 runtime 当前已绑定但不在 issuer 内的 runtime-only occupancy 一并计入有效占用，避免 host restart / issuer 漂移后继续超发 player session。
  - hosted issuer 现已把“刚 issue 但还没 register”的 pending slot 与正常在线 slot 分开处理：未完成 runtime register 的 slot 只享有更短的 `pending_registration_ttl_ms`，不会继续按完整 lease TTL 长时间占位。
- 已完成的 `TASK-P2P-041-D` hosted strong-auth preview 收口:
  - `oasis7_web_launcher` 在 `deployment_mode=hosted_public_join` 下会显式拒绝 `POST /api/chain/transfer`，返回结构化 `strong_auth_required`，不再让 public join 路径继续借用 trusted-local signer bootstrap。
  - `oasis7_game_launcher` 的 public player plane 现已新增通用 `/api/public/strong-auth/grant`：会先校验 `player_id + release_token` 仍对应有效 hosted player session，再要求后端 `approval_code` 正确，最后用服务端环境变量中的 signer 生成短期 `HostedStrongAuthGrant`。
  - `oasis7_game_launcher -> oasis7_viewer_live -> runtime-live` 现已透传 hosted deployment mode；在 `hosted_public_join` 下，`prompt_control preview/apply/rollback` 不再一律拒绝，而是要求“玩家本地签名的 `player_session` proof + backend-signed grant”同时成立，缺失时返回 `strong_auth_required`，篡改/过期/错 signer 时返回 `strong_auth_grant_invalid`。
  - 真实联调现已确认这条 preview lane 的 blocker 不再是 transport/并发：伪造的 env signer 会被显式拒绝为 `hosted strong-auth signer public key does not match private key`；换成真实匹配的 Ed25519 keypair 后，不带 `--with-llm` 时会正确停在 `llm_mode_required`，带 `--with-llm` 时则进一步暴露当前真实主 blocker `release_token does not map to an active player slot`。
  - 上述 `release_token` 错误发生时，浏览器与 runtime 侧仍可同时显示 `authRegistrationStatus=registered`、`authRuntimeStatus=registered` 与 `authBoundAgentId=agent-0`，但 public admission 已漂移到 `active_player_sessions=0`、`runtime_bound_player_sessions=1`、`runtime_only_player_sessions=1`、`released_players_total=1`；当前判断是 issuer 在 runtime 仍持有绑定时提前释放了 active slot，属于 hosted session issuer / release-token 生命周期竞争，而不是 grant route、viewer attach 或 signer 校验本身未打通。
  - 针对上述竞争，`HostedPlayerSessionIssuer::observe_runtime_active_players()` 现已改为“先用当前 runtime probe snapshot 给仍在 runtime 里活跃的 active slot 续租，再执行过期清理”，不再在本轮 probe 明明已经看到该 player 仍绑定时，先因历史 `last_seen` 过期把 release token 剪掉。新增回归 `hosted_player_session_runtime_probe_refreshes_runtime_bound_slot_before_expiry_prune` 已冻结这一修复。
  - `software_safe.js` 现会在 hosted public join 的 `prompt_control` lane 显示 `Backend Approval Code`，并改走同源通用 strong-auth grant route；`__AW_TEST__.getState()` 也会回出 `strongAuthApprovalCodeConfigured/strongAuthLastGrant*` 供 QA 取证。
  - `software_safe.js` 的 `authSurface.capabilities` 与页面 badge 现会显式导出 `main_token_transfer`，不再继续用 `strong_auth_actions` 这类代理概念代指真实资产动作；即便前端仍未开放资产操作，QA 也能直接看到真实 action_id 的 hosted verdict。
  - viewer summary 现已新增可读的 `Hosted Action Matrix` 面板，并把同一份结果同步暴露到 `__AW_TEST__.getState().hostedActionMatrix`；QA 不必再手抄 `hostedAccess.action_matrix` JSON 或靠按钮状态倒推 hosted verdict。
  - 交互区现已新增独立的 `Asset / Governance Lane` 面板：会单独展示 `main_token_transfer` 的 `required_auth/availability`、当前阻断原因，并给出禁用 CTA，避免资产动作仍只是一行 badge 或 buried 在 action matrix 里。
  - viewer summary 现还会在 `session_revoked/session_not_found/本地 release` 后显示独立 `Hosted Recovery` 面板，并把同一份派生结果暴露到 `__AW_TEST__.getState().hostedRecoveryHint`；玩家和 QA 都能直接看见“为什么掉回 guest、下一步该重新获取什么”。
  - runtime-live 的 `authoritative_recovery_ack/error` 现已补结构化 `revoke_reason/revoked_by`；`software_safe.js` 会在 remote revoke / operator kick 后保留这两项元数据，`Hosted Recovery` 面板与 `__AW_TEST__.getState().authRevokeReason/authRevokedBy` 也会直接显示“谁撤销了会话、撤销原因是什么”，不再只剩模糊的 `session_revoked` 字符串。
  - `/api/public/state` 的 `hosted_access` contract 现已导出动态 `action_matrix`：若 `OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY/PRIVATE_KEY/APPROVAL_CODE` 已配置，则 `prompt_control_*` 会从 `blocked_until_strong_auth` 升为 `public_player_plane_with_backend_reauth_preview`；`main_token_transfer` 仍保持 `blocked_until_strong_auth`。
  - 当前 grant route 虽已泛化到 `action_id` 维度，但 allowlist 仍只放行 `prompt_control_*`；若请求 `main_token_transfer`，public player plane 会显式返回 `strong_auth_action_not_enabled`，避免 route 泛化后被误读成 hosted 资产动作已可用。
  - 这条 hosted `prompt_control` strong-auth lane 仍明确属于 preview-grade backend reauth：后端 signer 当前只支持 env 托管 + approval code，不是 production signer custody，也不代表资产动作已具备 hosted-ready 安全级别。
- 2026-04-24 follow-up: `hosted-public-join-player-session-gate` 已把 launcher 真值也补齐到同一边界：`oasis7_client_launcher`、`oasis7_web_launcher` 与 `oasis7_game_launcher` 在 `deployment_mode=hosted_public_join` 下都会强制停用本地 `oasis7_chain_runtime`，不再把 public join 继续建模成 shared-devnet 节点 bootstrap；viewer/public snapshot 现会显式暴露 `local_chain_runtime=blocked_for_public_player_plane` 与 `node_admission=operator_managed_node_onboarding_only`。 Trace: `.pm/tasks/task_21dfffe808a24221a70fa5fe3fa895aa.yaml`
- 已完成的 `TASK-P2P-041-E` abuse suite 收口:
  - runtime-live 现已补 hosted `prompt_control` abuse 定向测试，至少覆盖 `expired grant -> strong_auth_grant_invalid`、`replayed player auth nonce -> auth_nonce_replay` 与 `session revoked after grant issuance -> session_revoked` 三条高风险签名，证明 preview-grade backend reauth 不能单靠 grant 穿透 session 生命周期与 nonce 防重放。
  - `oasis7_web_launcher::server` 现已补 remote private-control-plane matrix 回归，覆盖 `/api/state`、`/api/start|stop`、`/api/chain/start|stop`、`/api/gui-agent/*` 与 `/api/ui/schema` 的远端拒绝路径，要求统一返回 `operator_plane_only` 且只携带 public snapshot，避免误分享 operator URL 时把私有控制面状态直接暴露给公网访客。
  - `oasis7_hosted_access` 现已补 capability bypass 定向测试：即使 `OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY/PRIVATE_KEY/APPROVAL_CODE` 全部就绪、`prompt_control_*` 已升到 `public_player_plane_with_backend_reauth_preview`，`main_token_transfer` 仍必须保持 `blocked_until_strong_auth`，不能被 prompt lane 的 preview reauth 环境顺带打开。
  - `oasis7_web_launcher::server` 现已补 public snapshot 组合态回归：在同一份 env-ready snapshot 里，`prompt_control_apply` 必须显示 `public_player_plane_with_backend_reauth_preview`，而 `main_token_transfer` 仍必须显示 `blocked_until_strong_auth`，确保对外 contract 不会导出自相矛盾的 hosted verdict。
  - 现已新增浏览器侧证据 `doc/testing/evidence/hosted-world-browser-auth-surface-2026-03-26.md`：通过真实 `agent-browser` 会话确认 `Hosted Action Matrix`、`Asset / Governance Lane`、`Hosted Recovery` 与 `pending_registration_ttl_ms/release_token` 绑定都能在页面上稳定复现；同时验证 detached/agentless 页面下 `prompt_control_*` 仍不会误签发 grant、`main_token_transfer` 仍返回 `strong_auth_action_not_enabled`。
  - 现已新增并发接入证据 `doc/testing/evidence/hosted-world-browser-concurrency-2026-03-27.md`：在显式重编 `oasis7_viewer_live` sibling bin 后，用两份独立 `agent-browser` session 实测同一 `web_bind`，确认两个页面都能稳定进入 `debug_viewer:subscribed` 并同时看到 seeded agents，不再复现第二页长期 `detached`。
  - 现已新增真实 strong-auth 成功证据 `doc/testing/evidence/hosted-world-browser-strong-auth-success-2026-03-27.md`：在真实 signer + `--with-llm` 的 hosted 栈上，用浏览器本地临时 key、approval code 与未绑定的 `agent-1` 实测 `prompt_control preview/apply`，确认 `strongAuthLastGrantError = null` 且最终拿到 `preview_ack/apply_ack`，不再复现 `release_token does not map to an active player slot`。
  - 现已新增 remote revoke 浏览器证据 `doc/testing/evidence/hosted-world-browser-revoke-recovery-2026-03-27.md`：通过真实 `agent-browser` 页面 + 外部 `oasis7_pure_api_client revoke-session`，确认下一轮 hosted heartbeat 会把页面收敛到 `guest_session`，并同时显示 `authRevokeReason/authRevokedBy` 与带操作者/原因的 `Hosted Recovery` 文案。
  - 本轮已新增 `doc/testing/evidence/hosted-world-abuse-suite-matrix-2026-03-27.md`，把 replay / expiry / revocation / operator-public URL confusion / admission limit / capability bypass 的 required/full 证据和实跑命令汇总成统一矩阵。
- 已完成的 `TASK-P2P-041-F` liveops/runbook 收口:
  - 已新增 `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`，冻结 hosted operator 的最小执行法：区分 `public join URL / private control plane / signer path`，并明确分享前检查、误分享后的第一响应、incident 最小记录字段与 public claims freeze 边界。
  - 已新增 `doc/testing/templates/hosted-world-operator-incident-template.md`，把误分享 operator URL / private control plane 暴露的 incident 记录字段统一成可复用模板，避免 liveops/QA 各写各的事故摘要。
  - runbook 现已补 `session revoke` 实操步骤：明确 `player_id/session_pubkey/revoke_reason` 的收集来源、推荐 `oasis7_pure_api_client revoke-session` 命令、预期 `session_revoked` ack 字段，以及执行后应如何在浏览器侧确认 `Hosted Recovery` 与 `authRevokeReason/authRevokedBy` 已回流到玩家面。
  - 已新增 `doc/testing/templates/hosted-world-share-correction-template.md`，统一“错误 URL 更正 / revoke 后重新获取 hosted player session / preview 口径不升级”的对外文案骨架，避免不同 operator 在纠正消息里混入过度承诺或再次暴露错误入口。
  - runbook 现已补远程 operator tunnel / reverse proxy 最低策略，要求公网只暴露玩家 join 面，operator/control 面继续留在 loopback、VPN 或人工 tunnel 内。
  - 已新增 `doc/testing/templates/hosted-world-share-announcement-template.md`，统一首轮对外分享、重复提醒与 preview claims 边界文案，补齐“分享 / 误分享 / 撤销 / 事故通报”四类 liveops 输出物。
- 后续增强议题（不阻断本专题完成定义）:
  - hosted handoff / batch kick / operator kick 的产品化操作流仍可继续做得更顺手，但不影响当前 `player_id -> agent` 单 owner、revoke 和 rebind 规则已经成立。
  - host restart / rollback 后当前仍要求重新注册 session；若后续要做持久化 resume registry，应另立专题处理，而不是回退到浏览器长期 signer bootstrap。
  - 当前 hosted strong-auth 仍是 preview-grade backend reauth，后续若要升级到 production custody / wallet 插件 / externalized signer，应另立更高等级 strong-auth 专题；这不影响本专题“浏览器不持有长期 signer、prompt-control 走 strong-auth、main_token_transfer 继续阻断”的完成判定。
  - 更细粒度的 hosted action matrix 和更高等级 proof 仍可继续演进，但不影响当前 PRD 所需的三档 session ladder、敏感动作分级和 QA/LiveOps 边界已经冻结。

## 依赖
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.runbook.md`
- `doc/testing/evidence/hosted-world-browser-auth-surface-2026-03-26.md`
- `doc/testing/evidence/hosted-world-browser-concurrency-2026-03-27.md`
- `doc/testing/evidence/hosted-world-browser-strong-auth-success-2026-03-27.md`
- `doc/testing/evidence/hosted-world-browser-revoke-recovery-2026-03-27.md`
- `doc/testing/evidence/hosted-world-abuse-suite-matrix-2026-03-27.md`
- `doc/testing/templates/hosted-world-operator-incident-template.md`
- `doc/testing/templates/hosted-world-share-announcement-template.md`
- `doc/testing/templates/hosted-world-share-correction-template.md`
- `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
- `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `testing-manual.md`

## 验收命令（TASK-P2P-041-A P0 实装）
- `rg -n "public player plane|private control plane|signer plane|guest session|player session|strong auth|invite-only|gui-agent/action|admission control|specified_not_implemented" doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.design.md doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md doc/p2p/prd.md doc/p2p/project.md`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher --bin oasis7_game_launcher`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前状态: completed
- 下一步: 如需继续提升 hosted-world 安全等级，应另立 production custody / wallet / hosted handoff hardening 子专题；不再占用 `TASK-P2P-041` 的完成定义。
- 最近更新: 2026-03-27
