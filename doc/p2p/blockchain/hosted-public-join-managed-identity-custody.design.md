# oasis7 hosted_public_join 托管身份 / 托管密钥与邮箱登录（设计文档）

- 对应需求文档: `doc/p2p/blockchain/hosted-public-join-managed-identity-custody.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

审计轮次: 1

## 1. 设计目标
- 把 `hosted_public_join` 从“preview player-session + browser local key”推进到“普通玩家可登录、服务端可托管、后续可自托管升级”的正式产品架构。
- 让 runtime、viewer、custody backend、LiveOps 和后续 bridge/asset 系统围绕同一组身份主键工作，而不是继续把 `player_id`、浏览器密钥和 signer 语义混在一起。
- 保持现有 hosted access 平面边界不回退：public player plane 继续公开，private control plane 继续私有，托管密钥进入新的 custody plane，而不是回流到浏览器。

## 2. 当前代码真值
| 维度 | 当前状态 | 设计结论 |
| --- | --- | --- |
| Hosted session issue | `crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session.rs` 已管理 `player_id/device_session_id/release_token`、slot lease、refresh/release，并支持稳定 `player_id` 的复用发放 | public player session 与 device-session recovery 基线已落地，但 `signer_ref`/custody sign lane 仍未进入正式 contract |
| Hosted strong auth | `crates/oasis7/src/bin/oasis7_game_launcher/hosted_strong_auth.rs` 通过 `OASIS7_HOSTED_STRONG_AUTH_*` + `approval_code` 给特定 `action_id` 出 preview grant | 已有 backend reauth 前置，但不是正式 custody sign lane |
| Hosted account persistence backend | `crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_store_backend.rs` 现已把 `hosted_account_id -> player_id` 持久化抽成 `HostedAccountStoreBackend`，支持 `file` 与 `tablestore` 双 backend，并以 `OASIS7_HOSTED_ACCOUNT_STORE_BACKEND=auto|file|tablestore`、`OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` / `ALIYUN_OTS_*` env 决定 hosted 部署行为 | hosted account registry 已不再绑死单机 JSON；生产托管部署可把身份映射落到 Aliyun Tablestore，本地开发仍可保留文件 fallback |
| Legacy bootstrap off for hosted | `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs` 已断言 `hosted_public_join` 不再解析 viewer auth bootstrap | hosted 模式已停止从 `config.toml` 或 env 直注 host key 到浏览器 |
| Browser local persistence | `crates/oasis7_viewer/software_safe_src/viewer_hosted_auth_state_module.js` 现仅持久化 `hostedAccountId/playerId/deviceSessionId/releaseToken/sessionEpoch/issuedAtUnixMs`，旧版 `privateKey` 残留会在读取时清洗掉 | hosted 浏览器已不再把长期私钥写入 `localStorage`；当前剩余缺口是邮件投递与 custody sign，而不是浏览器长期材料 debt |
| Viewer auth bootstrap implementation | `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs` 仍保留从 env / `config.toml` 读取 `node.private_key` 的 trusted-local 路径 | 该能力继续只属于 trusted-local preview，不得回流到 hosted product 默认路径 |

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [AC-1/AC-2，PRD-P2P-029-A/B](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | 保持 `hosted_account_id`、`player_id`、`device_session_id` 与 `signer_ref` 的身份职责分离，并保留邮箱 hosted-account 登录作为玩家入口。 | [目标身份模型](#4-目标身份模型) | identity broker 与 runtime/account registry | 不代表 managed signer、production 邮件投递或真实跨设备恢复已经实现。 |
| [AC-3 / NFR-P2P-029-2](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | hosted 浏览器仅持久化设备会话恢复材料，不持久化长期托管 signer。 | [浏览器存储策略](#6-浏览器存储策略) | viewer auth-state module | 当前验证范围不含历史 `privateKey` 缓存清洗。 |
| [AC-3 / NFR-P2P-029-2](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | 保留旧 `localStorage privateKey` 残留须在读取时清洗的现有要求，并把本设计中的已实现状态与其验证缺口分开。 | [当前代码真值](#2-当前代码真值) | viewer auth-state module；回归测试仍待补齐 | 本文不声称该清洗行为已有精确回归覆盖。 |
| [AC-5/AC-7，NFR-P2P-029-3/4](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | hosted player 的 public、identity、custody 与 private control planes 保持边界；不得据此扩展 node、validator 或 governance signer 的托管范围。 | [目标平面拆分](#3-目标平面拆分) | runtime 与既有 custody/governance authority | 不定义 node、validator 或 governance signer 的生产 custody。 |
| [AC-4，Flow-P2P-029-005](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | `prompt_control_apply`、`prompt_control_rollback` 与 `main_token_transfer` 等高风险动作目标态需经过 step-up、风险判定和 custody sign。 | [风险分级与出签策略](#7-风险分级与出签策略) | runtime 与未来 custody-service delivery | managed custody sign lane 当前未实现；本表不把目标态写成现有能力。 |
| [AC-8，Flow-P2P-029-002](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | 邮箱 challenge 应拒绝无效凭据并施加 resend/burst 限流；有效账户可恢复既有 player identity。 | [推荐组件](#5-推荐组件) | hosted account identity broker | 当前单测不覆盖 production 邮件送达、重复账户合并或完整风控冻结流程。 |
| [AC-8，Flow-P2P-029-003/004/007](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | 已签发的 device session 可刷新、撤销或在恢复后重新签发；runtime 仍是 player/entity 绑定真值。 | [Runtime / Viewer 对接原则](#9-runtime-viewer-对接原则) | runtime session authority 与 viewer | 本地 issuer 单测不证明 hosted 部署或完整设备丢失/账户恢复 runbook。 |
| [AC-6，Flow-P2P-029-006](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | 自托管升级必须通过显式 external-wallet bind 或 transfer-out 路径，并避免把托管私钥回传浏览器。 | [实现顺序](#10-实现顺序) | runtime/account 与资产转移 owners，具体交付 owner 待任务绑定 | self-custody bind、cooldown 与 transfer-out 仍属后续阶段。 |
| [AC-10，NFR-P2P-029-6](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | 对外阶段必须继续表述为 `limited playable technical preview`，不能将设计目标当作已投产 hosted wallet。 | [当前阶段口径](#11-当前阶段口径) | producer/system review 与 claims review | 本设计不授权 production launch 或更改当前阶段。 |
| [NFR-P2P-029-7](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | dev、staging、production 仍须隔离 SMTP、account store、signer/approval secrets、风控阈值和对外 claims；当前阶段记录不能替代分层部署证据。 | [当前阶段口径](#11-当前阶段口径) | hosted identity/custody deployment owner，须由分层交付任务绑定 | 本文没有 staging/production 隔离的 live evidence，也不据此声称环境边界已验证。 |

## 3. 目标平面拆分
- `public player plane`
  - 对外可见: 静态网页、世界只读快照、guest/player session 入口、低风险 gameplay 输入
  - 不可见: operator control API、长期 signer、custody backend
- `identity plane`
  - 负责邮箱登录、OTP、device session、account recovery、rate limit
  - 输出: `hosted_account_id`, `device_session_id`, `player_session`
- `custody plane`
  - 负责 `signer_ref`、托管签名、step-up 授权、风险策略、审计日志
  - 对 public browser 永远只输出签名结果或 challenge state，不输出托管私钥
- `private control plane`
  - 继续承载世界启停、事故处理、运营控制、operator-only GUI actions

## 4. 目标身份模型
| 身份层 | 主键 | 生命周期 | 存储位置 | 用途 |
| --- | --- | --- | --- | --- |
| Hosted account | `hosted_account_id` | 长期 | identity store | 登录与恢复 |
| Player identity | `player_id` | 世界范围长期 | runtime/account registry | 玩家实体和世界内归属 |
| Device session | `device_session_id` | 短期 | browser + identity plane | 当前设备登录态 |
| Managed signer | `signer_ref` | 长期 | custody plane | 托管签名能力 |
| External wallet binding | `external_account_id` | 长期 | account registry | 自托管升级 |

设计规则:
- `hosted_account_id` 不直接等于 `player_id`，以便后续支持跨世界、跨设备与运营账户合并。
- `player_id` 不直接承载托管密钥引用，统一通过账户或 signer binding 间接关联。
- `device_session_id` 可以失效、轮换、冻结；它只服务当前设备，不等于账户长期身份。
- `signer_ref` 是 runtime/asset/custody 的唯一长期签名引用，底层后端可替换，但 API 语义不能漂移。

## 5. 推荐组件
- `hosted-account-service`
  - `start_login(channel, handle)`
  - `complete_login(challenge_id, code_or_link, device_info)`
  - `recover_account(handle, recovery_proof)`
- `player-session-broker`
  - `issue_guest_session(world_id)`
  - `exchange_account_for_player_session(hosted_account_id, device_pubkey)`
  - `refresh_device_session(device_session_id)`
  - `revoke_device_session(device_session_id)`
- `managed-custody-service`
  - `provision_signer(account_id, policy_profile)`
  - `prepare_sign(signer_ref, action_id, payload_digest)`
  - `approve_sign(authz_id, step_up_proof)`
  - `finalize_sign(authz_id)`
- `wallet-transition-service`
  - `bind_external_wallet(account_id, external_account_id, proof)`
  - `request_transfer_out(account_id, target_account_id, scope)`
  - `complete_transfer_out(request_id)`

## 6. 浏览器存储策略
- 允许:
  - `device_session_id`
  - 非导出的 device key handle 或等价短期浏览器材料
  - UI locale、非敏感 feature flags
- 禁止:
  - 托管 signer 明文私钥
  - node signer / governance signer / host key
  - 原始 OTP、长期 step-up token
- 兼容过渡:
  - hosted 浏览器当前只保留设备会话材料与页内临时 key；旧版 `localStorage privateKey` 残留会在读取时被清洗。
  - 后续切到真实邮件 provider / custody sign lane 时，仍应保留旧缓存迁移与提示逻辑，避免历史浏览器状态漂移回长期密钥路径。

## 7. 风险分级与出签策略
| 动作类 | 示例 | 浏览器本地可完成 | 需要 step-up | 需要 custody sign |
| --- | --- | --- | --- | --- |
| `guest_read` | 观战、读状态 | 是 | 否 | 否 |
| `player_gameplay` | 移动、普通玩法输入、低风险 chat | 是 | 否 | 否 |
| `creator_control_preview` | `prompt_control_preview` | 否 | 可选 | 视策略而定 |
| `creator_control_high_risk` | `prompt_control_apply` / `rollback` | 否 | 是 | 是 |
| `asset_transfer` | `main_token_transfer` | 否 | 是 | 是 |
| `governance_admin` | 治理或 treasury 相关动作 | 否 | 是 | 是，但不建议复用 player custody |

规则:
- gameplay 输入继续尽量留在 player session / device session 层，避免每个动作都经过 custody service。
- `main_token_transfer` 的 hosted 目标态不再是永久 `blocked`，而是进入 `step-up + managed custody sign` lane。
- governance/admin 如需浏览器入口，优先走独立更高等级 plane，不与普通 player custody 混用。

## 8. KMS / custody backend 边界
- 上层契约固定为 `signer_ref + sign API + audit trail`。
- 后端可以有两类实现:
  - `KMS/HSM direct key`
    - 适用于算法、吞吐与成本满足时
    - 优点: 私钥不可导出、托管语义清晰
  - `KMS-wrapped sealed key backend`
    - 适用于运行时算法或吞吐不适合直接落到 KMS key API 时
    - 优点: 可以保留上层 trust boundary，同时降低厂商耦合
- 不允许的实现:
  - 浏览器直接拿托管私钥
  - HTML/bootstrap 注入长期 signer
  - 只靠 `approval_code` + env signer 的长期生产运行

## 9. Runtime / Viewer 对接原则
- Runtime
  - 只校验 session、capability、签名证明和 `signer_ref` 绑定
  - 不直接关心邮箱明文
  - `player_id -> entity` 绑定继续由 runtime 真值维护
- Viewer
  - UI 上显示 `Oasis ID`、登录状态、custody mode、设备状态
  - 默认不显示公钥输入框；“外部钱包绑定”单独作为高级入口
  - 对旧 preview 路径要明确提示“这是 trusted-local preview，不是 hosted public join 正式模式”

## 10. 实现顺序
1. `hosted-managed-identity-doc-freeze`
   - 冻结产品和 trust boundary
2. `hosted-account-identity-broker`
   - 增加 hosted account 与登录因子
3. `device-session-and-runtime-binding`
   - 替换浏览器 `privateKey` 持久化，改成设备会话模型
4. `managed-custody-sign-api`
   - 新增 `signer_ref` 和 sign API
5. `step-up-auth-and-risk-policy`
   - 让高风险动作进入可审计的 step-up 体系
6. `external-wallet-bind-and-transfer-out`
   - 提供托管退出与自托管升级
7. `qa-abuse-and-liveops-runbook`
   - 形成运营和风控收口

## 11. 当前阶段口径
- 当前已成立:
  - `hosted_public_join` 已禁止 legacy host key bootstrap 直接进入 hosted mode
  - public player session / preview strong-auth contract 已存在
  - hosted account 邮箱登录 broker 已落地，viewer 正式入口已切到 hosted account login
  - hosted account registry 已支持 `file/tablestore` 双 backend；默认 `auto` 模式下无 OTS 配置走本地文件，有 `OASIS7_HOSTED_ACCOUNT_TABLESTORE_*` 或 `ALIYUN_OTS_*` 时自动切到 Aliyun Tablestore，并支持自动建表
  - hosted 浏览器已切到 `device_session + in-memory ephemeral Ed25519` 恢复模型，不再持久化 hosted player 私钥
- 当前未成立:
  - managed custody sign lane
  - self-custody bind / transfer-out 正式能力
  - 风控冻结与恢复 runbook
- 结论:
  - 当前仍是 `limited playable technical preview`
  - 本文描述的托管身份方向已有第一版实现，但距离生产级 hosted login / managed custody 仍有明显缺口

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [AC-1/AC-2，PRD-P2P-029-A/B](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [目标身份模型](#4-目标身份模型) | 同一邮箱登录应保留稳定 hosted account/player 关联；拒绝非邮箱 login channel。 | [测试分层手册](../../../testing-manual.md)规定 required-tier 选择；现有 ID：`crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity_tests.rs::hosted_account_login_start_rejects_phone_channel`、`::hosted_account_login_complete_reuses_stable_player_id`；本地 Rust 单元层，`test_tier_required`。 | 对应实现任务的 Issue evidence 与冻结候选单测结果。 | 不证明 production 邮件送达、跨世界账户映射或 managed signer 绑定。 |
| [AC-3 / NFR-P2P-029-2](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | [浏览器存储策略](#6-浏览器存储策略) | viewer 持久化 hosted browser session 时只保留身份和设备/session 恢复字段。 | [viewer hosted auth-state tests](../../../crates/oasis7_viewer/software_safe_src/viewer_hosted_auth_state_module.test.js)：`recovers snake_case hosted player session storage and migrates it to the canonical key shape`；本地 Vitest 层，`test_tier_required`。 | 对应实现任务的 Issue evidence 与冻结候选 Vitest 结果。 | 该测试检查 canonical payload；它不直接断言旧 `privateKey` 缓存清洗。 |
| [AC-3 / NFR-P2P-029-2](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | [当前代码真值](#2-当前代码真值) | 旧 `privateKey` 缓存读取时清洗是设计记录的现状声明；需有专门负例才能独立验证。 | N/A; reason=仓内没有直接覆盖历史 privateKey 残留清洗的精确回归测试; scope=旧 viewer localStorage cache migration; owner_role=viewer_engineer; evidence_ref=hosted-public-join-managed-identity-custody.design.md#2-当前代码真值; re-evaluate=补充并运行对应旧缓存回归测试时 | 当前设计状态声明；未来对应实现任务 Issue evidence 与负例结果。 | 不能从 canonical session payload 测试推导旧缓存清洗已验证。 |
| [AC-5/AC-7，NFR-P2P-029-3/4](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [目标平面拆分](#3-目标平面拆分) | hosted public join 不应解析 trusted-local viewer auth bootstrap。 | [测试分层手册](../../../testing-manual.md)规定 required-tier 选择；现有 ID：`crates/oasis7/src/bin/oasis7_game_launcher/launcher_viewer_auth_bootstrap_tests.rs::hosted_public_join_disables_viewer_auth_bootstrap_resolution`；本地 Rust 单元层，`test_tier_required`。 | 对应实现任务的 Issue evidence 与冻结候选单测结果。 | 不验证 KMS/HSM custody、node/governance signer 隔离或 hosted production planes。 |
| [AC-4，Flow-P2P-029-005](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [风险分级与出签策略](#7-风险分级与出签策略) | 高风险资产/控制动作目标态应经过 step-up、policy 与 custody sign；当前设计应继续区分 preview strong-auth 与正式 sign lane。 | N/A; reason=正式 managed custody sign lane 尚未实现且仓内没有该行为的匹配测试; scope=prompt control 高风险出签、main_token_transfer 与生产 custody backend; owner_role=runtime_engineer; evidence_ref=hosted-public-join-managed-identity-custody.design.md#11-当前阶段口径; re-evaluate=managed custody sign lane 首次实现时 | 当前阶段声明；未来实现任务 Issue evidence、单测及集成验证结果。 | 现有 preview strong-auth 不能证明正式 signer_ref 出签或生产风险策略。 |
| [AC-8，Flow-P2P-029-002](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [推荐组件](#5-推荐组件) | email challenge 应拒绝错误 OTP 并执行 resend/burst 限流；有效凭据应复用稳定账户/player identity。 | [测试分层手册](../../../testing-manual.md)规定 required-tier 选择；现有 ID：`crates/oasis7/src/bin/oasis7_game_launcher/hosted_account_identity_tests.rs::hosted_account_login_complete_rejects_wrong_otp`、`::hosted_account_login_start_enforces_burst_rate_limit`；本地 Rust 单元层，`test_tier_required`。 | 对应实现任务的 Issue evidence 与冻结候选单测结果。 | 不证明重复账户合并、冻结复核或运营恢复 runbook。 |
| [AC-8，Flow-P2P-029-003/004/007](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [Runtime / Viewer 对接原则](#9-runtime-viewer-对接原则) | device session refresh/release 与撤销状态必须维持 player-session authority；runtime 继续维护 player/entity binding。 | [测试分层手册](../../../testing-manual.md)规定 required-tier 选择；现有 ID：`crates/oasis7/src/bin/oasis7_game_launcher/hosted_player_session_tests.rs::hosted_player_session_refresh_rotates_registration_grant_for_new_browser_key`、`::hosted_player_session_ledger_recovers_lease_rate_and_revocation_after_restart`；本地 Rust 单元层，`test_tier_required`。 | 对应实现任务的 Issue evidence 与冻结候选单测结果。 | 不证明真实 hosted runtime 的账户恢复流程或 LiveOps device-loss 演练。 |
| [AC-6，Flow-P2P-029-006](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [实现顺序](#10-实现顺序) | transfer-out/self-custody 仅在有 external wallet bind、冷却与审计回归后可视为验证。 | N/A; reason=external wallet bind 与 transfer-out 当前未实现且没有匹配测试或操作手册; scope=self-custody migration、cooldown 与 transfer audit; owner_role=runtime_engineer; evidence_ref=hosted-public-join-managed-identity-custody.design.md#11-当前阶段口径; re-evaluate=external wallet bind 或 transfer-out 首次实现时 | 当前阶段声明；未来对应实现任务 Issue evidence 与测试/审计工件。 | 不证明外部账户绑定、资产迁移或托管退出已可用。 |
| [AC-10，NFR-P2P-029-6](hosted-public-join-managed-identity-custody.prd.md#2-user-experience-functionality) | [当前阶段口径](#11-当前阶段口径) | 文档与对外 claims 应保留当前 preview 阶段语义，不将目标架构描述成已发布能力。 | N/A; reason=阶段与 claims 是本设计的专业文档判定且没有匹配的自动测试或操作手册; scope=仓内及对外 hosted wallet/custody claims; owner_role=producer_system_designer; evidence_ref=hosted-public-join-managed-identity-custody.design.md#11-当前阶段口径; re-evaluate=阶段或 claims 发生变化并进入独立 review 时 | 本设计阶段记录与对应任务的专业 review evidence。 | 文档结构检查不证明真实服务部署或 release readiness。 |
| [NFR-P2P-029-7](hosted-public-join-managed-identity-custody.prd.md#3-technical-requirements) | [当前阶段口径](#11-当前阶段口径) | dev、staging、production 之间的 SMTP、account store、signer/approval secret、风控阈值和 claims 隔离必须由分层环境证据证明。 | N/A; reason=仓内没有覆盖三层 hosted identity/custody 服务隔离的精确测试或手册; scope=staging/production data plane 与独立环境配置; owner_role=blockchain_ops_engineer; evidence_ref=hosted-public-join-managed-identity-custody.design.md#11-当前阶段口径; re-evaluate=分层部署任务提供独立环境验证证据时 | 未来分层部署任务 Issue evidence 与 staging/prod smoke artifact。 | 当前本地单测和文档检查不证明真实环境隔离。 |
