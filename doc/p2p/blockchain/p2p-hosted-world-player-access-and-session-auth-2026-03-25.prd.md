# oasis7 hosted world 玩家访问与会话鉴权（2026-03-25）

- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md`

审计轮次: 1
## 1. Executive Summary
- Problem Statement: oasis7 当前 Web hosted 路径把 host/operator/player/node signer 混在一起处理。公开网页可直达管理接口，浏览器还会收到从 `config.toml` 或环境变量导出的长期私钥，这让“某个玩家部署服务，另一个玩家直接打开网页游玩”在安全边界上仍是不成立的。
- Proposed Solution: 冻结一套 producer-owned 的 hosted world 访问与会话鉴权方案，拆分 `public player plane / private control plane / signer plane` 三层平面，并引入 `guest session -> player session -> strong auth` 的鉴权梯度，让浏览器只持有可过期的会话/能力，不再持有长期 signer 私钥；同时把 `/api/gui-agent/action` 从共享控制面拆成 player-safe surface 与 operator-only surface，并为公开 join URL 增加 admission control。
- Success Criteria:
  - SC-1: hosted world 正式方案明确禁止把 `node` 或治理 signer 的长期私钥注入浏览器 HTML/JS/bootstrap。
  - SC-2: hosted world 对外访问明确区分 public join URL 与 private operator/control URL，避免共享玩家入口与管理入口复用同一信任面。
  - SC-3: 远程玩家最少可完成 `打开 join URL -> 建 guest/player session -> 绑定玩家实体 -> 重连恢复` 的闭环定义。
  - SC-4: Web 侧敏感动作至少区分三档权限：普通游玩、受限交互、强鉴权资产/治理动作，且 `/api/gui-agent/action` 不再作为 shared hosted public surface 保留。
  - SC-5: 公开 join URL 必须具备最小 admission control，至少冻结 `max_guest_sessions/max_player_sessions/issue_rate_limit/world_full_policy`。
  - SC-6: `doc/p2p/project.md` 建立可执行任务链，覆盖 `runtime_engineer`、`viewer_engineer`、`agent_engineer`、`qa_engineer`、`liveops_community` 的 owner 和测试层级。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要把“玩家可访问 hosted world”从模糊想法收敛成可交付架构，而不是继续用 preview bootstrap 混过去。
  - hosted world host / operator：需要公开一个可分享的 join URL，但不想把 world 控制权和 node signer 一起暴露给访客。
  - 远程玩家：需要点开网页就能进入世界，先看后玩，必要时再做更强鉴权，而不是先拿到 host 私钥。
  - `viewer_engineer`：需要明确网页登录、会话状态、敏感按钮降级和错误反馈怎么做。
  - `qa_engineer` / `liveops_community`：需要知道共享访问、误分享、滥用、撤销和对外口径该怎么验。
- User Scenarios & Frequency:
  - 玩家把自己部署的 hosted world 分享给朋友试玩时，每次都要走一次 join/session 流程。
  - 运营或 QA 复核 hosted world 可公开访问边界时，每个候选版本至少做一次。
  - 发生链接泄露、会话劫持、误把管理口开放到公网时，必须立即触发回滚/撤销/runbook。
- User Stories:
  - PRD-P2P-023-A: As a `producer_system_designer`, I want hosted world access split into explicit trust planes, so that public multiplayer access stops depending on preview-only trust shortcuts.
  - PRD-P2P-023-B: As a remote player, I want to join a hosted world through the web without receiving the host's long-lived signer, so that I can play safely.
  - PRD-P2P-023-C: As a host operator, I want a public join flow separated from admin/control operations, so that sharing the game URL does not hand out control authority.
  - PRD-P2P-023-D: As a `viewer_engineer`, I want a session ladder with explicit UI capability states, so that login, reconnect and disabled actions are predictable.
  - PRD-P2P-023-E: As a `qa_engineer` / `liveops_community`, I want abuse cases, revocation flow and public-claims boundaries frozen, so that hosted access can be validated and operated.
- Critical User Flows:
  1. Flow-P2P-023-001: `host 启动 hosted world -> 生成 public join URL + private operator URL -> 仅公开 join URL`
  2. Flow-P2P-023-002: `远程访客打开 join URL -> 获得 guest session -> 浏览/观战/读取世界快照`
  3. Flow-P2P-023-003: `访客点击开始游玩 -> 建立 player session -> 绑定 player_id / entity slot / capability set -> 开始低风险交互`
  4. Flow-P2P-023-004: `玩家尝试高风险动作 -> 系统要求 strong auth -> 鉴权通过后才放行资产/治理相关动作`
  5. Flow-P2P-023-005: `session 过期/被撤销/host 重启 -> 客户端收到明确错误 -> 走 reconnect 或重新登录`
  6. Flow-P2P-023-006: `host 或 operator 误把管理面暴露到公网 -> QA/LiveOps runbook 判定越界 -> 立即 freeze public claims 并要求整改`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Public player plane | `join_url/world_id/public_ws/public_http/session_issuer_ref` | 对外仅提供 join、只读状态、player session 建立入口 | `draft -> published -> revoked` | join URL 只承载公开接入信息，不携带长期 signer 真值 | 任何互联网玩家可访问；不得包含 operator/control 能力 |
| Private control plane | `admin_bind/admin_origin/operator_identity/control_actions` | 启停世界、改配置、人工封禁、应急恢复 | `private_only -> active -> frozen` | 与 public plane 分离；默认不可由 join URL 推导 | 仅 host/operator 可访问 |
| Signer plane | `signer_type/custody_backend/public_key_ref/policy_ref` | 为强鉴权动作签名或验签 | `preview_bootstrap -> delegated -> externalized` | 浏览器只见公钥或能力声明，不见长期私钥 | signer 只存在于受控后端/离线托管 |
| Guest session | `session_id/world_id/expires_at/device_hint` | 允许打开网页、观战、查看世界基本状态 | `issued -> active -> expired/revoked` | 默认时效有限；不能转义为资产权限 | 未登录访客可获得 |
| Player session | `session_id/player_id/entity_id/capability_set/resume_token` | 允许移动、普通交互、低风险聊天和玩家内玩法输入 | `pending_bind -> active -> suspended/revoked` | 必须完成 `player_id -> entity` 绑定后才可写入世界输入 | 仅登录后的玩家会话可持有 |
| Strong auth | `session_id/auth_level/challenge_id/proof_ref/proof_expiry` | 对资产转账、治理、敏感 prompt/control 类动作做二次鉴权 | `required -> challenged -> authorized -> expired` | 强鉴权必须独立于普通 player session；失效后自动回落 | 仅被授权的玩家或 operator 可触发 |
| Join admission control | `max_guest_sessions/max_player_sessions/issue_rate_limit/world_full_policy/kick_policy` | 对 guest/player session 签发、占位、满员和踢出做准入判定 | `open -> rate_limited/world_full -> reopened` | 公开 join 默认不是无限制发 session；超限时必须返回结构化拒绝 | 由 host/operator 配置，runtime 执行 |
| Entity 绑定与断线恢复 | `player_id/entity_id/resume_token/last_seen_tick/revoke_epoch` | 重连时恢复到原玩家实体或显式拒绝 | `unbound -> bound -> resumed/rebound` | `resume_token` 失效或 `revoke_epoch` 变化时必须重新登录 | 玩家只可恢复自己的实体 |
| Agent action surface split | `action_surface_id/action_class/player_safe/operator_only` | 将 `gui-agent` 入口拆成 player-safe 子集和 operator-only 子集 | `legacy_shared -> split_enforced` | 在拆分完成前，`/api/gui-agent/action` 默认视为 private control plane | 玩家只能命中显式 allowlist 的 player-safe action |
| Web 敏感动作降级 | `action_id/action_class/required_auth/ui_state/reject_reason` | 前端根据能力显示、隐藏或禁用按钮 | `hidden -> disabled -> enabled` | `agent chat`、`prompt control`、`main token transfer` 至少三档区分 | 缺能力时前后端都必须拒绝 |
- Acceptance Criteria:
  - AC-1: 本专题必须明确 hosted world 的三层平面：`public player plane`、`private control plane`、`signer plane`。
  - AC-2: 本专题必须明确 hosted world 的会话梯度：`guest session`、`player session`、`strong auth`，并说明每档可做/不可做的动作。
  - AC-3: 本专题必须明确当前 Web 侧敏感动作分类，至少覆盖 `agent chat`、`prompt control`、`main token transfer` 三类。
  - AC-4: 本专题必须明确：hosted world 公网 join 场景下，浏览器不得再接收 `node.private_key`、治理 signer 私钥或任何长期 signer 真值。
  - AC-5: 本专题必须明确：`/api/start`、`/api/stop`、`/api/chain/start`、`/api/chain/stop` 这类控制面动作不得继续作为 public player origin 的默认可达入口；`/api/gui-agent/action` 要么拆分，要么整体留在 private control plane。
  - AC-6: 本专题必须明确 guest/player session 的 admission control，至少覆盖发放速率、世界满员、会话上限和踢出策略。
  - AC-7: 本专题必须明确 invite-only 不是当前 base requirement；基础方案以 `public join + session/capability` 为主，不把 allowlist/invite 当成安全替代。
  - AC-8: 本专题必须明确 hosted world 当前 verdict 只能落在 `trusted_local_only_preview`、`hosted_public_join_blocked_until_strong_auth` 或 `hosted_public_join_strong_auth_preview` 等受限状态；不会单靠建档就升级 `hosted_ready`、`limited playable technical preview` 或 `crypto-hardened preview` 口径。
  - AC-9: `doc/p2p/project.md` 必须建立 `TASK-P2P-041` 任务链，并拆出 runtime/viewer/agent/QA/LiveOps 的后续 owner。
- Non-Goals:
  - 不在本专题内直接实现钱包接入、第三方 OAuth 或完整账户系统。
  - 不在本专题内继续支持 invite-only 作为首版 hosted world 访问前提。
  - 不在本专题内把 preview-only signer custody 直接升级成 production custody。
  - 不替代 `mainchain-token-signed-transaction-authorization`、`p2p-production-signer-custody-keystore`、`p2p-mainnet-public-claims-policy` 等既有专题。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: hosted world 正式形态需要把“公开玩游戏”“私有控制世界”“安全持有 signer”拆成三个平面。`public player plane` 负责 join URL、公开静态资源、WebSocket/world 输入与 session issue；`private control plane` 负责 world 启停、配置和应急操作；`signer plane` 只为强鉴权动作提供签名或验签。浏览器只能持有短期 session、能力声明和必要公钥，不再承载长期私钥。hosted v1 的最小 player-session 形态可以是“同源 public issue endpoint 签发 opaque `player_id + release_token`，浏览器本地生成/持久化临时 Ed25519 key，再通过 `register_session/reconnect_sync` 接到 runtime one-player-one-agent 规则；退出时通过 release route 归还 active slot，在线期间通过 refresh route 续租 lease，让 stale slot 能自动过期回收”。其中 lease/release 归还必须同时校验 `player_id + release_token` 绑定，不能退化成 token-only 即可操作 slot；`game_launcher` 的同源 public player plane 还应维持一条独立的 runtime presence 常驻连接，优先消费 `AgentPlayerBound/AgentPlayerUnbound` 增量事件，再用周期性 `RequestSnapshot` 做纠偏，把“曾经已在 runtime 绑定、现在又从 runtime 消失”的玩家 slot 回收掉，避免 operator kick / remote revoke 长时间卡住 `world_full`。`world_full` 的 admission 判定不能只看 issuer active slot，还必须把 runtime 当前已绑定但不在 issuer 内的 runtime-only occupancy 计入有效占用，避免 host restart / issuer 漂移后继续超发 player session；与此同时，刚 issue 但尚未完成 runtime register 的 pending slot 也不能无限占位，必须使用短于正常 lease TTL 的 pending-registration TTL 自动回收。在当前 preview slice 中，同源 public player plane 需要提供通用的 `strong-auth grant` endpoint，并通过 `action_id` 区分具体动作；其中仅 `prompt_control_preview/apply/rollback` 允许走 preview-grade hosted strong-auth lane：浏览器先用本地临时 key 完成 `player_session` 签名，再带 `approval_code` 向该 endpoint 换取短期 backend-signed `HostedStrongAuthGrant`，runtime 必须同时校验两者后才放行；该 lane 明确不等价于 production custody，且 `main_token_transfer` 继续保持 `blocked_until_strong_auth`，即便 grant endpoint 已泛化，也不得被误判为 hosted-ready。若 host restart / rollback 让 runtime session 状态丢失，浏览器必须重新注册，不得回退到长期 signer bootstrap。
- Integration Points:
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若 public join origin 仍能直接命中 world 启停或链控制接口，则必须判定为 architecture fail，而不是“部署时自己注意”。
  - 若 HTML bootstrap、JS 全局对象或任意 `/api/*` 返回体中仍出现长期私钥、助记词或 signer 原文，则 hosted world 直接 `block_security_boundary`。
  - 若 guest session 尝试发送 player-only 输入，必须返回明确的 `auth_level_insufficient`，同时前端按钮保持禁用。
  - 若 player session 尝试执行 `main token transfer` 或其他强鉴权动作但没有额外 proof，必须返回 `strong_auth_required`，不得静默降级到 node signer 代签。
  - 若 hosted `prompt_control` 的 `approval_code` 不匹配，或 backend-signed grant 与 `player_id/public_key/agent_id/action_id` 不匹配、已过期、或 signer 不在 allowlist 中，必须返回结构化 `approval_code_invalid` 或 `strong_auth_grant_invalid`，不得退化成仅凭 `player_session` 放行。
  - 若公开 join URL 的 guest/player session 发放超过 `issue_rate_limit`、已达 `max_guest_sessions/max_player_sessions` 或 world 已满，必须返回结构化 `rate_limited/world_full`，而且 `world_full` 判定必须计入 issuer active slot 与 runtime-only occupancy 的有效并集，不能因为 issuer 漂移而继续无界签发 session；未完成 register 的 pending slot 也必须在更短 TTL 后自动回收，避免恶意或失败 issue 长时间卡满世界。
  - 若浏览器试图用不匹配的 `player_id + release_token` 组合去 refresh/release active slot，public player plane 必须返回结构化 `player_id_required/player_id_mismatch/release_token_invalid`，不能只凭 token 命中别人的 slot。
  - 若 host 重启 world、撤销玩家、或 rotate session secret，旧 `resume_token` 必须立即失效并要求重新登录。
  - 若同一 `player_id` 从多个浏览器并发恢复同一实体，必须有单一 owner 规则；冲突者进入 `suspended` 或重绑流程。
  - 若 host 错把 operator URL 当 join URL 分享，runbook 必须要求立即 rotate bind/secret、撤销会话并回收 public claims。
- Non-Functional Requirements:
  - NFR-P2P-023-1: hosted world public player plane 在任何 HTML/JS/bootstrap/API 响应中都不得暴露长期私钥或 signer seed。
  - NFR-P2P-023-2: public player origin 默认不得暴露 world start/stop、chain start/stop、GUI operator action 等管理入口。
  - NFR-P2P-023-3: guest/player/strong-auth 三档能力必须在前端和后端双重生效，不能只做 UI 隐藏。
  - NFR-P2P-023-4: session 必须可过期、可撤销、可重连；默认 session TTL 必须是有限值，且不得等同于长期 signer 生命周期。
  - NFR-P2P-023-5: 所有 hosted world 敏感拒绝路径必须返回可归类错误码，如 `auth_level_insufficient`、`session_revoked`、`strong_auth_required`、`operator_plane_only`。
  - NFR-P2P-023-6: hosted world 公开 join 面必须有有界 admission control；未达到上限前可签发，会话达到上限或 world 满员时必须显式拒绝并可审计。
  - NFR-P2P-023-7: 在 hosted world 方案真正实现前，对外口径不得声称“玩家可安全把网页公开给任何人并共玩”，统一只能描述为受限 preview/blocking verdict，不得描述为 `hosted_ready`。
  - NFR-P2P-023-8: QA required 套件必须覆盖 session replay、expired token、revocation、admin/public URL 混淆、admission limit 和敏感按钮降级。
  - NFR-P2P-023-9: preview-grade hosted `strong_auth` 只允许把短期 backend-signed grant 暴露给浏览器；backend signer 私钥必须继续留在服务端受控环境中，不得回流到 HTML/JS/bootstrap。
- Security & Privacy: 浏览器应被视为不可信执行面。hosted world 的根安全原则是“浏览器拿 session，不拿长期 signer；玩家入口管游玩，不管运维；强鉴权单独升级，不依赖 host node key 代签”。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结三层平面、会话梯度、动作能力矩阵和 claims boundary。
  - v1.1: 落地 `guest session` + `player session`，把浏览器长期私钥 bootstrap 从 hosted-world 路径移除。
  - v1.2: 落地 runtime capability enforcement、entity bind/reconnect/revoke。
  - v2.0: 落地 `strong auth` 与 hosted-world runbook/QA abuse suite，并与 shared network 轨道衔接。
- Technical Risks:
  - 风险-1: 如果继续让 public player plane 和 control plane 共用同一入口，任何部署细节失误都会直接变成 world 控制泄露。
  - 风险-2: 如果只把私钥从 UI 隐掉但后端仍默认代签，仍然只是“看不见私钥”的假隔离。
  - 风险-3: 如果不把 `agent chat`、`prompt control`、`main token transfer` 分级，hosted world 很容易把创作者能力、玩家能力和资产能力继续混在一起。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-023-A | TASK-P2P-041-A | `test_tier_required` | plane split、`gui-agent` surface split 策略与 hosted verdict 冻结 | hosted world 架构边界、部署口径 |
| PRD-P2P-023-B | TASK-P2P-041-A/B/C | `test_tier_required` + `test_tier_full` | 浏览器 bootstrap 脱敏、guest/player session、entity bind/reconnect/revoke 回归 | Web join、session 生命周期、玩家身份 |
| PRD-P2P-023-C | TASK-P2P-041-A/E | `test_tier_required` + `test_tier_full` | public/control plane 隔离、admission control、operator/public URL 拒绝与 runbook 验证 | 运维安全、公开分享边界 |
| PRD-P2P-023-D | TASK-P2P-041-B/D | `test_tier_required` | viewer login/session UX、按钮降级和错误反馈回归 | Web 体验、敏感功能显隐 |
| PRD-P2P-023-E | TASK-P2P-041-E/F | `test_tier_required` + `test_tier_full` | abuse suite、撤销/runbook、claims review 与 shared-host evidence | QA 阻断、LiveOps 事故处理、对外口径 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-023-001 | 把 hosted world 拆成 `public player plane / private control plane / signer plane` | 继续让 join、control、signer 共享单一 web launcher 入口 | 玩家公开访问和 operator 管理属于不同信任域，不拆平面就无法稳定支持他人通过网页进入。 |
| DEC-P2P-023-002 | 浏览器持有短期 session / capability，不持有长期 signer 私钥 | 继续通过 HTML/bootstrap 注入 `node.private_key` 或治理 signer | 浏览器是最弱信任面，长期 signer 一旦进入 HTML/JS，就不再具备 hosted-world 可分享前提。 |
| DEC-P2P-023-003 | 采用 `guest -> player -> strong auth` 梯度 | 要么所有动作都不登录，要么所有动作都要求强钱包鉴权 | 游玩、社交、资产、治理风险不同，梯度式鉴权才能兼顾可玩性和风险隔离。 |
| DEC-P2P-023-004 | base hosted-world 方案不依赖 invite-only，先做公开 join + session/capability | 把 invite/allowlist 当成第一版安全替代 | invite-only 只能缩小访问面，不能替代私钥隔离、平面拆分和能力校验。 |
| DEC-P2P-023-005 | `/api/gui-agent/action` 在 hosted-ready 前必须拆成 player-safe surface 与 operator-only surface，未拆分前默认算 private control plane | 继续把 `gui-agent/action` 视为单一共享入口，靠调用方自觉区分 | 这条接口同时承载玩家语义和 operator 语义，不先拆分就会把 hosted-world 权限边界重新混回去。 |
