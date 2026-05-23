# oasis7 hosted_public_join 托管身份 / 托管密钥与邮箱登录（2026-05-18）

- 对应设计文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`

审计轮次: 1

## 目标
- 为 `hosted_public_join` 建立一套正式的托管身份 / 托管密钥产品规格，解决“普通玩家如何登录并长期使用”而不是继续停留在 preview 会话与浏览器本地私钥。
- 把邮箱登录、托管 player signer、step-up auth 与自托管升级路径收成统一真值，供 runtime / viewer / QA / LiveOps 继续实现。

## 范围
- 覆盖 hosted player 的账户身份、设备会话、托管签名、风险分级、自托管升级与相关 trust boundary。
- 不覆盖 node / validator / governance signer 的生产托管；这些继续由既有 custody / governance 专题负责。

## 接口 / 数据
- 主文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.prd.md`
- 设计文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.design.md`
- 项目管理文档: `doc/p2p/blockchain/p2p-hosted-public-join-managed-identity-custody-2026-05-18.project.md`
- 关键主键: `hosted_account_id`、`player_id`、`device_session_id`、`signer_ref`

## 里程碑
- M1 (2026-05-18): 专题 PRD / design / project 建档，冻结 hosted account、邮箱登录、device session、托管签名与自托管升级边界。
- M2: 落地 hosted account 与 device session，清退浏览器 `localStorage privateKey` preview debt。
- M3: 落地 custody sign API、step-up auth 与 external wallet bind / transfer-out。

## 风险
- 若把 hosted account 与高风险出签混为一步，玩家体验和风控都可能失衡。
- 若继续让浏览器持有长期 player signer，任何缓存泄露都会放大为账户接管风险。
- 若把 player custody 与 node/governance signer 共用一套 trust domain，会让产品问题和协议级 custody 问题互相绑死。

## 1. Executive Summary
- Problem Statement: `hosted_public_join` 现在已经补上了 hosted account 邮箱登录 broker、`device_session + in-memory session key` 恢复链路，以及 env-configured SMTP 邮件投递；普通玩家不再需要手抄浏览器本地私钥，也不必继续依赖 repo-owned preview code 才能回到同一 `player_id`。但这套实现整体仍停留在 `limited playable technical preview` 边界：`crates/oasis7/src/bin/oasis7_game_launcher/hosted_strong_auth.rs` 依然依赖 `OASIS7_HOSTED_STRONG_AUTH_*` + `approval_code`；`signer_ref`、managed custody sign API、冻结/恢复策略和 self-custody upgrade 还没有进入正式后端 contract。如果不把这些缺口继续收完，`public join` 仍然不能被宣称为生产级 hosted wallet / custody 方案。
- Proposed Solution: 为 `hosted_public_join` 正式冻结一套 producer-owned 的“托管身份 + 托管密钥 + 自托管升级”目标态。默认玩家路径改为 `guest -> hosted account -> managed player signer -> optional self-custody bind/transfer-out`：用户用邮箱验证码登录，浏览器只拿 `player_session` 与设备级短期密钥；长期玩家 signer 留在服务端 custody plane，由 KMS/HSM 或 KMS-wrapped sealed-key backend 保护，并通过 step-up auth + policy engine 出签。
- Success Criteria:
  - SC-1: `hosted_public_join` 必须提供“不输入原始公钥私钥也能开始玩”的正式登录路径，默认支持邮箱登录。
  - SC-2: 浏览器在 `hosted_public_join` 下不得再持有长期资产 signer、node signer 或可长期复用的明文私钥；旧 `localStorage privateKey` 残留必须被清洗而不是恢复到运行态。
  - SC-3: 玩家身份必须拆成 `hosted_account_id`、`player_id`、`device_session_id`、`signer_ref` 四类真值，不再把 `player_id` 直接当成完整账户体系。
  - SC-4: 高风险动作必须支持 `managed custody sign` lane，并由 step-up auth、风险策略和审计日志共同放行；`main_token_transfer` 不得再停留在“blocked，但没有目标方案”的状态。
  - SC-5: 玩家默认不需要保存或输入原始公私钥；如需自托管，必须走显式的 `bind external wallet` 或 `transfer-out to self-custody` 流程，而不是把托管私钥直接回流到浏览器。
  - SC-6: 本专题必须与 `PRD-P2P-023 hosted-world session auth` 和 `PRD-P2P-017 signer custody` 形成清晰边界：只覆盖 hosted player identity/custody，不覆盖 node、validator、governance signer 的生产托管。
  - SC-7: `doc/p2p/prd.md`、`doc/p2p/project.md` 与本专题三件套完成映射，后续实现任务可直接挂到统一真值。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`: 需要把“邮箱登录游戏”和“链上身份/签名”收成一个不会误伤安全边界的正式产品方案。
  - 普通玩家: 希望点开公开 URL 就能试玩，升级到正式玩家时只做常见登录，不维护裸私钥。
  - `runtime_engineer`: 需要知道 runtime 应该信任什么身份断言、什么 sign API、什么 step-up 结果。
  - `viewer_engineer`: 需要知道浏览器本地还能存什么、登录/重连/升级如何表达。
  - `qa_engineer` / `liveops_community`: 需要知道托管账户、托管签名、撤销、恢复和事故处置的 pass/block 标准。
- User Scenarios & Frequency:
  - 新用户第一次打开公开 join URL 时，需要先以 guest 方式进入，再升级为 hosted account player。
  - 老用户换设备或清缓存时，需要通过邮箱重新登录并恢复玩家身份，而不是找回私钥文件。
  - 玩家第一次执行高风险动作时，需要通过 step-up auth 触发托管签名，而不是暴露托管私钥。
  - 玩家后续若要脱离托管模式，需要显式绑定外部钱包或转移到自托管账户。
- User Stories:
  - PRD-P2P-029-A: As a `producer_system_designer`, I want `hosted_public_join` to have a formal hosted account model, so that public onboarding does not depend on users handling raw keys.
  - PRD-P2P-029-B: As a player, I want email login to recover my game identity, so that I can re-enter the world without copying a private key around.
  - PRD-P2P-029-C: As a `runtime_engineer`, I want a managed signer reference and sign API boundary, so that runtime can accept hosted-custody signatures without trusting the browser.
  - PRD-P2P-029-D: As a `viewer_engineer`, I want device-session and step-up auth states to be explicit, so that login, reconnect and sensitive-action UX are predictable.
  - PRD-P2P-029-E: As a `qa_engineer` / `liveops_community`, I want abuse, recovery, revocation and transfer-out rules frozen, so that hosted login can be operated safely.
  - PRD-P2P-029-F: As a player, I want an optional path to bind an external wallet or move to self-custody later, so that managed onboarding does not lock me into browser-only trust.
- Critical User Flows:
  1. Flow-P2P-029-001: `访客打开 hosted_public_join URL -> 先拿 guest session -> 浏览世界 -> 点击开始游玩`
  2. Flow-P2P-029-002: `用户选择邮箱登录 -> 收到 OTP challenge -> hosted account 验证通过 -> 生成或恢复 hosted_account_id`
  3. Flow-P2P-029-003: `account service 为用户恢复 signer_ref 和 player_id -> session broker 签发 player_session + device_session -> runtime 完成 entity bind`
  4. Flow-P2P-029-004: `玩家执行普通玩法输入 -> 浏览器只用 device session key 或 session token -> runtime 校验 capability，不触发 custody sign`
  5. Flow-P2P-029-005: `玩家执行 prompt_control_apply / main_token_transfer 等高风险动作 -> step-up auth -> policy engine 评估 -> custody service 出签 -> runtime 验证并执行`
  6. Flow-P2P-029-006: `玩家要退出托管 -> 绑定外部钱包或发起 transfer-out -> 进入 cooldown / 审计 -> 迁移完成后把 custody_mode 切到 self_custody`
  7. Flow-P2P-029-007: `邮箱失效、设备丢失、账号被盗用或风控命中 -> operator/recovery runbook -> 撤销 device_session 或冻结 managed signer -> 用户重新认证`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Hosted account | `hosted_account_id/login_channel/login_hint/verified_at/status` | 邮箱发码、校验、恢复账户 | `guest -> challenge_issued -> verified -> active/frozen` | 同一登录因子可映射多个设备 session，但只能映射一个当前主账户真值 | `identity plane` 负责 |
| Device session | `device_session_id/player_session_id/device_pubkey/expires_at/risk_level` | 浏览器建立/刷新/撤销短期设备会话 | `issued -> registered -> refreshed -> expired/revoked` | 设备会话必须有限 TTL；浏览器只允许存 `session handle + device key handle`，不存长期 signer 明文私钥 | `public player plane` + `identity plane` |
| Managed signer reference | `signer_ref/custody_mode/algorithm/backend_class/account_binding` | 服务端创建、恢复、停用或迁移托管 signer | `provisioning -> active -> step_up_required -> frozen/migrated` | 运行时只认 `signer_ref` 及签名证明，不认浏览器自报托管私钥 | `custody plane` 负责 |
| Sign authorization | `action_id/policy_class/step_up_result/audit_id/signature_proof` | 高风险动作先授权再出签 | `requested -> challenged -> approved -> signed -> consumed/denied` | `main_token_transfer`、治理与高风险 prompt/control 必须经过 step-up 和风控；失败时返回结构化错误 | `policy engine` + `custody plane` |
| Self-custody upgrade | `custody_mode/target_account_id/transfer_out_request_id/cooldown_until` | 绑定外部钱包或发起迁移 | `managed -> export_pending -> transferred -> self_custody` | MVP 不要求导出原始私钥；至少支持把资产和身份迁到外部账户 | 需 step-up + operator / runtime 复核 |
| Audit / recovery | `risk_event_id/revoke_reason/recovered_by/evidence_ref` | 撤销设备、冻结账户、恢复访问 | `healthy -> challenged -> suspended -> recovered/closed` | 任一异常登录、设备丢失、滥用或风控命中都必须可审计 | `qa_engineer` / `liveops_community` 可追踪 |
- Acceptance Criteria:
  - AC-1: 专题必须明确 `hosted_account_id/player_id/device_session_id/signer_ref` 的职责分离，不再把 `player_id` 既当登录账户又当 signer。
  - AC-2: 专题必须明确邮箱登录是 hosted player 默认入口，而“输入公钥/私钥”不是默认 UX。
  - AC-3: 专题必须明确 hosted 浏览器已经清退 `localStorage privateKey` 持久化，并改用 `device_session + in-memory ephemeral Ed25519` 恢复路径；旧缓存残留必须在读取时被清洗。
  - AC-4: 专题必须明确 `managed custody sign` lane 的动作范围，至少覆盖 `prompt_control_apply` / `prompt_control_rollback` / `main_token_transfer`。
  - AC-5: 专题必须明确托管身份只适用于 player plane；node / validator / governance signer 继续由独立 custody/governance 文档约束。
  - AC-6: 专题必须定义至少一个不回传托管私钥的 self-custody 升级路径。
  - AC-7: 本专题必须显式定义 `identity plane / custody plane / public player plane / private control plane` 的边界。
  - AC-8: 本专题必须包含 recovery、revocation、device loss、rate limit、duplicate account bind、step-up failure 的处理规则。
  - AC-9: 本专题必须在 `doc/p2p/prd.md`、`doc/p2p/project.md` 和 `doc/p2p/prd.index.md` 建立映射。
  - AC-10: 本专题必须保持当前对外阶段口径仍为 `limited playable technical preview`；文档不得把该方案误写成“已经实现”。

## 3. Technical Requirements
- Architecture Overview: 最合适的 hosted_public_join 目标态不是“让浏览器代持真正的玩家私钥”，也不是“强迫用户第一次进游戏就接外部钱包”，而是把登录、设备会话、托管签名和后续自托管升级拆成四层。
  - `identity broker`: 处理邮箱 OTP、rate limit 与登录风险控制。
  - `account registry`: 保存 `hosted_account_id -> player_id/signer_ref/device bindings` 映射；当前 hosted account 第一版实现允许 `file` 与 `Aliyun Tablestore` 双 backend，并约定 hosted 部署默认应优先落到服务端托管表存储而不是单机 JSON。
  - `session broker`: 用已验证账户换发 `player_session` 与设备短期 key challenge，服务于 runtime register/reconnect。
  - `custody service`: 只暴露 `prepare_sign/approve_sign/finalize_sign` 或等价 sign API；底层可以是 KMS/HSM 直管密钥，也可以是 KMS-wrapped sealed-key backend，但浏览器不能直接拿到长期 signer。
  - `policy engine`: 对 action class、设备风险、step-up 结果、冷却期与风控状态做统一判定。
  - `wallet bind / transfer-out gateway`: 处理外部钱包绑定、自托管迁移和托管退出。
- Data Model:
  - `hosted_account`: `hosted_account_id`, `login_channel`, `normalized_login_hint`, `status`, `created_at`, `last_verified_at`
  - `account_factor`: `factor_id`, `account_id`, `factor_type`, `masked_handle`, `verified_at`, `revoked_at`
  - `player_identity_binding`: `account_id`, `player_id`, `world_scope`, `entity_binding_mode`, `status`
  - `managed_signer`: `signer_ref`, `account_id`, `algorithm`, `backend_class`, `custody_mode`, `state`, `created_at`
  - `device_session`: `device_session_id`, `account_id`, `player_session_id`, `device_pubkey`, `issued_at`, `expires_at`, `risk_level`
  - `sign_authorization`: `authz_id`, `signer_ref`, `action_id`, `step_up_method`, `approved_at`, `expires_at`, `audit_id`
  - `custody_audit_log`: `audit_id`, `account_id`, `event_type`, `actor`, `reason`, `evidence_ref`, `created_at`
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_store_backend.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs`
  - `crates/oasis7/src/bin/oasis7_game_launcher/hosted_strong_auth.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs`
  - `crates/oasis7_viewer/software_safe.js`
  - `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.prd.md`
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若邮箱重复绑定到多个 hosted account，必须提供 canonical merge / reject 规则，不能静默创建分叉账户。
  - 若浏览器丢失 device session，但账户因子仍有效，必须允许重新登录恢复，而不是要求用户回忆旧私钥。
  - 若 hosted 部署配置了 `OASIS7_HOSTED_ACCOUNT_STORE_BACKEND=tablestore`，或 `auto` 模式下检测到 `OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` / `ALIYUN_OTS_*`，但 endpoint/AK/table 条件不完整，server 必须在启动期显式报错，而不是静默回退到错误 backend。
  - 若 Tablestore 表不存在，服务端可以在 `auto_create=true` 时自动建表；若关闭自动建表，则必须以明确错误阻断启动，避免把 hosted account 持久化退化成隐式 best-effort。
  - 若 KMS/HSM 不直接支持运行时签名算法，custody backend 可以退化为 KMS-wrapped sealed key，但对上层仍必须保持“只暴露 sign API，不暴露长期私钥”的契约。
  - 若风控判定高风险设备登录，必须把账户状态切到 `step_up_required` 或 `frozen_review`，不得继续自动签发高权限 session。
  - 若托管到自托管迁移尚未完成，managed 与 self-custody 不得并行对同一高风险动作出签，必须有单一 source of truth。
  - 若 hosted account 已存在但 `player_id` 对应实体槽已被 runtime 占用，必须先走 rebind / operator arbitration，不能靠重复创建 signer 绕过 ownership。
- Non-Functional Requirements:
  - NFR-P2P-029-1: 任何 hosted login、step-up、sign authorization、revoke、freeze 与 transfer-out 事件都必须有审计记录。
  - NFR-P2P-029-2: 浏览器端不得持久化托管 signer 明文私钥；hosted 浏览器当前只允许落盘 `device_session` 级恢复材料，旧 `privateKey` 残留必须在读取时被清洗而不是继续恢复到运行态。
  - NFR-P2P-029-3: `identity broker` 与 `custody service` 必须是独立信任面；身份验证通过不等于自动拥有高风险出签能力。
  - NFR-P2P-029-4: `managed custody` 默认适用于 hosted player surface，不得被误扩展为 node/governance/validator signer 方案。
  - NFR-P2P-029-5: 用户默认看到的是 `Oasis ID` / 账户状态 / custody mode，而不是原始公钥命名。
  - NFR-P2P-029-6: 本专题完成前，仓内与对外口径不得声称“任意新用户已经默认拥有安全托管钱包并可直接进行资产动作”。
- Security & Privacy: 邮箱、设备标识和签名授权记录都属于敏感身份数据。本专题允许记录账户 ID、factor 类型、掩码后的联系方式、signer 引用、公钥摘要、step-up 与审计事件；禁止把真实 OTP、原始私钥、seed 或长期签名材料写入仓库、前端 bootstrap、日志与测试证据。

## 4. Risks & Roadmap
- Non-Goals:
  - 不在本专题里为 node / validator / governance signer 设计生产托管体系。
  - 不要求 MVP 支持“导出托管私钥原文”；自托管升级可以先以 `bind external wallet` 或 `transfer-out` 完成。
  - 不把 hosted identity 直接等同于 bridge-service、充值或法币支付体系。
- Phased Rollout:
  - MVP: 冻结 hosted account、device session、managed signer、step-up auth、transfer-out 的正式合同。
  - v1.1: 落地邮箱登录、设备恢复与 runtime 账户绑定。
  - v1.2: 落地 custody sign API、`main_token_transfer` 托管签名 lane 与审计。
  - v2.0: 落地 external wallet bind / self-custody transfer-out，与桥接、资产与高级 creator 能力衔接。
- Technical Risks:
  - 风险-1: 如果把“邮箱登录”和“高风险动作出签”绑死在同一步，玩家体验和风控都会失衡。
  - 风险-2: 如果继续把浏览器本地 `privateKey` 当作 hosted player 正式方案，任何缓存泄露都可能扩大为账户接管。
  - 风险-3: 如果 KMS 选型过早写死到单一厂商 API，后续算法、成本或跨环境迁移会很难收口；因此上层必须先冻结 `signer_ref + sign API` 契约。

## 5. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-029-A | hosted-managed-identity-doc-freeze / hosted-account-identity-broker | `test_tier_required` | 专题 PRD/design/project 建档、模块入口映射、登录与 hosted account contract 冻结 | hosted onboarding、账户真值 |
| PRD-P2P-029-B | hosted-account-identity-broker / device-session-and-runtime-binding | `test_tier_required` + `test_tier_full` | 邮箱登录、guest->player upgrade、恢复与 rebind 回归 | player login、session 生命周期 |
| PRD-P2P-029-C | managed-custody-sign-api | `test_tier_required` + `test_tier_full` | signer_ref、custody backend、sign authorization 与 runtime 验签回归 | high-risk action、资产面签名 |
| PRD-P2P-029-D | device-session-and-runtime-binding / step-up-auth-and-risk-policy | `test_tier_required` | viewer UX、设备会话存储、step-up 状态与错误反馈回归 | 前端登录/重连/敏感动作文案 |
| PRD-P2P-029-E | qa-abuse-and-liveops-runbook | `test_tier_required` + `test_tier_full` | 盗号、设备丢失、重复绑定、风控冻结、撤销与恢复 runbook | 运营事故、QA 阻断 |
| PRD-P2P-029-F | external-wallet-bind-and-transfer-out | `test_tier_required` + `test_tier_full` | self-custody 升级、wallet bind、transfer-out 冷却与审计回归 | 账户迁移、custody exit |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-029-001 | `hosted_public_join` 默认走邮箱 hosted account，而不是用户手填公私钥 | 让新用户自己保存或输入公私钥 | 普通玩家 onboarding 成本过高，且无法支撑恢复、风控和运营收口。 |
| DEC-P2P-029-002 | 浏览器只持有设备级短期会话材料，长期玩家 signer 留在 custody plane | 继续把 `privateKey` 持久化到浏览器 localStorage 作为正式方案 | 这与 hosted-world “浏览器不持有长期 signer” 的既有边界冲突。 |
| DEC-P2P-029-003 | 上层冻结 `signer_ref + sign API` 契约，底层后端允许 `KMS/HSM` 或 `KMS-wrapped sealed-key backend` | 直接把产品方案写死为单一云 KMS 的具体 key API | 产品首先需要稳定的 trust boundary，而不是把运行时算法/成本约束提前硬编码进 PRD。 |
| DEC-P2P-029-004 | 自托管升级先支持 `bind external wallet` / `transfer-out`，不要求 MVP 导出托管私钥原文 | 把“导出原始托管私钥”当成 MVP 必备功能 | 迁移能力重要，但“把私钥直接吐给浏览器”不是 hosted onboarding 的最优安全默认值。 |
| DEC-P2P-029-005 | 本专题只覆盖 hosted player identity/custody，不复用到 node/governance signer | 让一个统一 KMS 同时接管 player、node、governance 全部 signer | trust domain、审计要求与风险级别不同，混用会把 hosted player 产品问题和协议级 custody 问题绑死。 |
