# oasis7 主链 Token 签名交易鉴权（2026-03-23）

- 对应设计文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.design.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.project.md`

审计轮次: 3
## 1. Executive Summary
- Problem Statement: 当前虽然 `TransferMainToken` 的公开 HTTP 入口与 shared payload 层已经完成签名化，但 Web/native 的 `oasis7_client_launcher` 转账入口仍然只会提交裸字段，无法自己产出 `public_key + signature`，导致前端闭环仍然断在 signer/bootstrap 层。
- Proposed Solution: 在保留 transfer HTTP 鉴权与 shared payload submit-layer gating 的前提下，把主链 Token 资产动作的签名模型继续接到 `oasis7_client_launcher` 的 Web/native 转账窗口；native 复用现有 Rust signer helper，wasm 复刻同一份 transfer canonical payload 签名协议，并让 `oasis7_web_launcher` 在受信本地环境把 viewer auth bootstrap 注入到 HTML，确保浏览器端也能产出与 runtime 契约一致的 signed request。
- Success Criteria:
  - SC-1: `/v1/chain/transfer/submit` 不再接受未签名请求；缺少 `public_key` 或 `signature` 的请求必须被拒绝。
  - SC-2: 只有当 `from_account_id == oc:pk:<normalized_public_key_hex>` 且 `ed25519` 签名校验通过时，转账请求才允许继续进入余额/nonce 预检与共识提交。
  - SC-3: `ConsensusActionPayloadEnvelope` 必须支持主链 Token auth proof，且 `NodeRuntime` 对 transfer/claim/genesis/treasury/restricted-admin-registry 提交时强制要求对应 proof。
  - SC-4: claim/genesis/treasury/restricted-admin-registry 即使没有公开 HTTP submit surface，也必须在共享提交层具备“无 proof 直接拒绝”的门禁。
  - SC-5: `genesis/treasury/restricted-admin-registry/liveops-pool-top-up` 的 `auth.account_id` 必须绑定到正式 controller slot；其中 `UpdateRestrictedStarterClaimAdminRegistry` 与 `TopUpRestrictedStarterClaimLiveopsPool` 都固定绑定 `ecosystem_pool` treasury controller slot，不能继续接受任意命名 controller label。
  - SC-6: `InitializeMainTokenGenesis / DistributeMainTokenTreasury / UpdateRestrictedStarterClaimAdminRegistry / TopUpRestrictedStarterClaimLiveopsPool` 必须支持 `threshold_ed25519` controller proof，并要求 proof 的 `threshold` 与 controller signer policy 一致。
  - SC-7: `genesis/treasury/restricted-admin-registry/liveops-pool-top-up` 在 `STRAUTH-2B2/2B3/2B4` 完成后，submit-layer 必须拒绝 signer 不在 allowlist、唯一签名数未达 threshold、或 controller signer policy 缺失的 payload。
  - SC-8: `STRAUTH-2B2` 只完成代码级 signer allowlist / threshold enforcement，不得误写成 ceremony / HSM / external signer 已完成。
  - SC-9: `oasis7_client_launcher` 的 Web/native 转账窗口必须在提交前自行产出 `public_key + signature`，不再发送裸 `from/to/amount/nonce`。
  - SC-10: `oasis7_web_launcher` 必须把本地 signer bootstrap 注入所服务的 HTML，使 wasm 端在 trusted local deployment 下也能生成与 runtime 契约一致的 transfer 签名。

## 2. User Experience & Functionality
- User Personas:
  - `runtime_engineer`：需要先关闭当前最暴露的公开资产提交面，并把后续资产动作收口到共享提交层。
  - `qa_engineer`：需要把签名缺失、签名错误、账户不匹配、payload 层无 proof 变成可验证的阻断用例。
  - `producer_system_designer`：需要看到“transfer HTTP 已完成，payload 层开始统一，但 genesis/treasury 的治理绑定仍未完成”的真实阶段。
  - `viewer_engineer`：需要把 Web/native 转账入口补到“本地产签再提交”，而不是继续发送裸 JSON。
  - 治理/金库维护者：需要知道 `genesis/treasury/restricted-admin-registry` 现在至少要携带签名化 controller 元数据，而不是继续 unsigned 提交。
- User Scenarios & Frequency:
  - 链上转账提交：每次玩家或运营侧通过公开 runtime/control-plane 入口提交主链 Token 转账时触发。
  - 链内资产动作提交：每次任何 submit surface 经由 `NodeRuntime::submit_consensus_action_payload*` 提交主链 Token 资产动作时触发。
  - 安全回归：每次修改资产提交接口、payload envelope 或转账交互时触发。
  - 创世前安全推进：每次 producer 复核 `not_mainnet_grade` blocker 是否有实质收口时触发。
- User Stories:
  - PRD-P2P-TXAUTH-001: As a `runtime_engineer`, I want transfer submit to require a valid signature and shared payload auth, so that公开资产入口和共享提交层都不再信任裸 JSON 或无 proof payload。
  - PRD-P2P-TXAUTH-002: As a `qa_engineer`, I want invalid/mismatched auth to fail with stable error codes, so that release gate can block unsigned asset surfaces.
  - PRD-P2P-TXAUTH-003: As a `producer_system_designer`, I want this专题明确写出“transfer HTTP 已完成，payload 层开始统一，但治理 signer 绑定仍待完成”， so that阶段判断不会把 P0 的推进误判成全部资产动作已安全闭环。
- Critical User Flows:
  1. Flow-P2P-TXAUTH-001: `客户端构造 transfer request -> 使用 ed25519 对 canonical payload 签名 -> 提交 public_key/signature -> runtime 验证 -> 通过后继续余额/nonce 预检 -> 写入 signed consensus payload`
  2. Flow-P2P-TXAUTH-002: `请求缺少签名或签名格式非法 -> runtime 直接返回 invalid_request/invalid_signature -> 不进入余额检查与提交流程`
  3. Flow-P2P-TXAUTH-003: `public_key 签名有效但 from_account_id 不等于 oc:pk:<public_key_hex> -> runtime 返回 account_auth_mismatch -> 不允许冒用其他账户`
  4. Flow-P2P-TXAUTH-004: `caller 构造 claim/genesis/treasury/restricted-admin-registry runtime action -> 在 consensus payload envelope 附带 auth proof -> NodeRuntime 先验签再入队 -> 未附 proof 时直接拒绝`
  5. Flow-P2P-TXAUTH-005: `producer 复核专题状态 -> 看到 transfer HTTP 与 payload 层都已签名化 -> 仍保留 genesis/treasury controller binding 与 signer ceremony 为后续任务 -> 继续维持 not_mainnet_grade 口径`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Transfer submit 鉴权请求 | `from_account_id/to_account_id/amount/nonce/public_key/signature` | runtime 解析 JSON 并校验字段完整性 | `raw_request -> parsed_request` | `amount > 0`、`nonce > 0`、字段不可空、账户 ID 仍遵守现有格式约束 | 任何公开调用方都必须显式提供签名材料 |
| Transfer canonical signing payload | `version/operation/from_account_id/to_account_id/amount/nonce/public_key` | 以固定字段顺序编码为 canonical JSON，并加固定域前缀后验签 | `parsed_request -> auth_verified/auth_rejected` | 签名域固定为 transfer submit，避免与 viewer/chat auth 混用 | 仅持有对应私钥的请求方可生成有效签名 |
| 账户所有权绑定 | `derived_from_account_id = oc:pk:<public_key_hex>` | runtime 比对 `request.from_account_id` 与派生账户 | `auth_verified -> bound/rejected` | 若不相等则返回 `account_auth_mismatch` | 请求人只能提交自己公钥派生的主链账户 |
| Shared consensus auth envelope | `auth.scheme/auth.account_id/auth.public_key/auth.signature` | caller 将主链 Token auth proof 附加到 `ConsensusActionPayloadEnvelope` | `unsigned_payload -> signed_payload -> runtime_verified/rejected` | transfer/claim/genesis/treasury/restricted-admin-registry 进入 `NodeRuntime` 前必须带 proof；proof 验签失败直接拒绝 | 所有未来 submit surface 统一走该提交层校验 |
| Claim / Genesis / Treasury / Restricted Registry 扩展位 | `action_surface/account_binding/controller_binding/status` | claim 绑定 beneficiary；genesis/treasury/restricted-admin-registry 从 slot registry 继续升级到 threshold controller signer policy | `planned -> payload_signed -> slot_bound -> signer_policy_enforced -> ceremony_pending` | claim 若为 `oc:pk:` 需可推导；controller-bound 动作的 `auth.account_id` 必须命中正式 controller slot，且 proof signer 必须命中 allowlist / threshold | 后续仍需 producer/QA 收口外部 signer 与 ceremony |
- Acceptance Criteria:
  - AC-1: `oasis7_chain_runtime` 的 `ChainTransferSubmitRequest` 必须新增 `public_key` 与 `signature` 字段，并在缺失时拒绝请求。
  - AC-2: runtime 必须验证 `ed25519` 签名，且 `from_account_id` 必须严格等于 `oc:pk:<normalized_public_key_hex>`。
  - AC-3: 签名无效、签名格式非法、公钥格式非法、账户绑定不匹配必须返回结构化错误，不得继续进入 `preflight_validate_transfer_request`。
  - AC-4: 有效签名请求仍必须保留现有 `same account / amount / nonce / insufficient balance / nonce replay` 行为和错误语义。
  - AC-5: `oasis7_web_launcher` 的控制面请求结构、序列化与代理测试必须同步更新到新字段集合。
  - AC-6: `ConsensusActionPayloadEnvelope` 必须新增可选主链 Token auth proof；`NodeRuntime` 对 `TransferMainToken / ClaimMainTokenVesting / InitializeMainTokenGenesis / DistributeMainTokenTreasury / UpdateRestrictedStarterClaimAdminRegistry` 提交时必须强制校验该 proof。
  - AC-7: 定向 required 回归必须覆盖 transfer 有效签名成功、缺签名拒绝、错误签名拒绝、`from_account_id` 与 `public_key` 不匹配拒绝。
  - AC-8: 定向 required 回归必须覆盖 claim/genesis/treasury/restricted-admin-registry 在 payload 层“有 proof 可入队、缺 proof 拒绝”的提交门禁。
  - AC-9: `InitializeMainTokenGenesis` 在 payload submit 层必须只接受正式 genesis controller slot（当前为 `msig.genesis.v1`），不得接受任意 controller label。
  - AC-10: `DistributeMainTokenTreasury` 在 payload submit 层必须按 treasury `bucket_id` 绑定到正式 controller slot（例如 `ecosystem_pool -> msig.ecosystem_governance.v1`），不得接受任意 controller label。
  - AC-11: `MainTokenActionAuthProof` 必须支持 `threshold_ed25519`，包含 `threshold` 与 `participant_signatures`，用于 genesis/treasury controller proof。
  - AC-12: `NodeRuntime` 必须对 genesis/treasury/restricted-admin-registry 读取正式 controller signer policy，并拒绝 allowlist 为空、signer 不在 allowlist、proof.threshold 与 policy 不一致、或唯一签名数未达 threshold 的 payload。
  - AC-13: 定向 required 回归必须覆盖 genesis/treasury/restricted-admin-registry 的 allowlist miss、threshold not met、policy missing 拒绝，以及合法 threshold proof 通过。
  - AC-14A: `UpdateRestrictedStarterClaimAdminRegistry` 的 `auth.account_id` 必须与 `ecosystem_pool` treasury controller slot 当前绑定的 controller account 一致；即使 action 字段自带 `controller_account_id`，若与 slot 真值不一致也必须拒绝。
  - AC-14: 专题文档必须明确 `STRAUTH-2B2/2B3` 只完成代码级 signer allowlist / threshold enforcement，ceremony / HSM / external signer 仍是后续任务。
  - AC-15: 专题文档必须接入 `doc/p2p/prd.md`、`doc/p2p/project.md`、`doc/p2p/prd.index.md` 与 `doc/p2p/README.md`。
  - AC-16: `oasis7_client_launcher` 的 native 转账窗口必须在提交前生成合法 transfer signature，并把 `public_key/signature` 带入 `/api/chain/transfer`。
  - AC-17: `oasis7_client_launcher` 的 wasm 转账窗口必须复刻相同 canonical payload 与签名前缀，并在 signer bootstrap 可用时生成与 native 同契约的 signed request。
  - AC-18: `oasis7_web_launcher` 服务静态 HTML 时必须注入 `__OASIS7_VIEWER_AUTH_ENV` bootstrap，使 wasm 端能读取本地 signer 公私钥。
  - AC-19: 当前 signer 来源仅限本地 env/config bootstrap；本轮不得误写成已实现钱包托管、助记词、HSM/KMS 或生产级 keystore。
  - AC-20: `test_tier_required` 必须覆盖 signed request builder、launcher transfer request 序列化，以及 `wasm32` 编译通过。
- Non-Goals:
  - 本轮不实现生产级 keystore、HSM/KMS、硬件钱包或外部 signer 服务。
  - 本轮不完成外部 signer 服务、HSM/KMS、硬件钱包或 ceremony 自动化。
  - 本轮不重做 Web/native 转账窗口整体交互，也不引入助记词/钱包管理体验；只把现有表单升级成 trusted local signer bootstrap 提交。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: `oasis7_client_launcher` 的 native 转账窗口通过本地 signer helper 读取 env/config 中的 signer bootstrap，wasm 转账窗口通过 `window.__OASIS7_VIEWER_AUTH_ENV` 读取同一组 signer bootstrap；两端都先对 `TransferMainToken` 构造同一份 canonical auth payload，再以 `ed25519` 生成 action-level proof，并把 `public_key/signature` 写入 `/api/chain/transfer` 请求。`oasis7_chain_runtime` 的 transfer HTTP 入口先完成请求级校验，再把 proof 写进共识 envelope；`NodeRuntime` 则在共享提交层对 transfer/claim/genesis/treasury/restricted-admin-registry 统一验签和阻断，其中 restricted admin registry update 额外绑定 `ecosystem_pool` treasury controller slot。
- Integration Points:
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_tests.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
  - `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs`
  - `crates/oasis7_client_launcher/src/transfer_auth.rs`
  - `crates/oasis7_client_launcher/src/transfer_window.rs`
  - `crates/oasis7_client_launcher/src/transfer_window_web.rs`
  - `crates/oasis7/src/consensus_action_payload.rs`
  - `crates/oasis7/src/runtime/main_token.rs`
  - `crates/oasis7_node/src/node_runtime_core.rs`
  - `crates/oasis7_node/src/tests_action_payload.rs`
  - `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若 `public_key` 不是 32-byte hex，直接按 `invalid_request` 拒绝。
  - 若 `signature` 不带预期前缀或不是 64-byte hex，按 `invalid_signature` 拒绝。
  - 若 `public_key` 大小写混用，允许通过规范化为小写 hex 后参与派生与验签。
  - 若签名通过但 `from_account_id` 不是该公钥派生账户，按 `account_auth_mismatch` 拒绝。
  - 若 Web 端未注入 signer bootstrap，前端必须在本地提示 `transfer signing failed`，而不是继续提交裸请求。
  - 若本地 env/config 只配置了单边 key（只有 public 或只有 private），前端必须在本地提示 signer bootstrap 缺失，不得静默降级成 unsigned submit。
  - 若 claim 的 `beneficiary` 为 `oc:pk:` 账户，则必须可由 `public_key` 推导；若是 `protocol:*` 等命名控制账户，则当前仅要求 proof 中的 `account_id` 与 `beneficiary` 一致并通过签名校验。
  - 若 genesis/treasury/restricted-admin-registry payload 使用单签 proof，但 controller signer policy 的 threshold > 1，则必须拒绝，不能以单签冒充多签治理。
  - 若 genesis/treasury/restricted-admin-registry payload 带有效 threshold proof，但对应 signer list 仍是本地配置而非 ceremony freeze 的最终真值，不得误解为 ceremony 已完成。
  - 若鉴权通过但余额不足或 nonce 回放，继续返回现有业务错误，不改变既有预检规则。
  - 若真实治理 signer 绑定、controller allowlist 或 signer ceremony 尚未完成，本专题不得据此把整体 verdict 提升为 `mainnet_grade`。
- Non-Functional Requirements:
  - NFR-P2P-TXAUTH-1: 公开转账提交面不存在“缺签名也能继续执行余额/nonce 预检”的旁路。
  - NFR-P2P-TXAUTH-2: transfer auth 仅接受 `ed25519` 32-byte public key 与 64-byte signature，编码为 hex 且具备固定版本前缀。
  - NFR-P2P-TXAUTH-3: canonical payload 必须稳定、可回归，同一请求字段在相同顺序下得到完全一致的签名原文。
  - NFR-P2P-TXAUTH-4: 定向 required 回归命令必须在同一提交中给出，且 `git diff --check` 通过。
  - NFR-P2P-TXAUTH-5: transfer/claim/genesis/treasury/restricted-admin-registry 在进入 `NodeRuntime` 前不存在无 proof 旁路。
  - NFR-P2P-TXAUTH-6: 在治理 signer allowlist、external signer 与 signer ceremony 未完成前，模块级安全 verdict 仍保持 `not_mainnet_grade`。
- Security & Privacy: 请求只上传公钥与签名，不上传私钥或助记词；runtime 只基于请求内容验签，不在本轮引入任何本地私钥托管逻辑。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 为 `POST /v1/chain/transfer/submit` 落地签名鉴权、账户绑定与 required 回归。
  - v1.1: 将 `ClaimMainTokenVesting / InitializeMainTokenGenesis / DistributeMainTokenTreasury / UpdateRestrictedStarterClaimAdminRegistry` 接入同一 signed transaction envelope，并在 `NodeRuntime` 强制验签。
  - v1.2: 为 genesis/treasury/restricted-admin-registry 补正式 controller slot binding，并把任意 controller label 收紧到固定 registry。
  - v1.3: 为 `oasis7_client_launcher` 的 Web/native 转账窗口补本地 signer bootstrap 与 signed request builder，并保留 full QA 证据待补。
  - v2.0: 为 genesis/treasury 补外部 signer / ceremony / keystore 托管，并把当前本地配置 allowlist 升级为更长期的治理真值来源。
- Technical Risks:
  - 风险-1: Web/native 现有转账入口若未同步提供签名材料，会在后端收口后直接变成拒绝路径。
  - 风险-2: 若 canonical payload 定义不稳定，后续多端实现会出现签名不兼容。
  - 风险-3: 若 producer 误把“payload 已签名化”解读成“治理 signer 绑定已完成”，会再次高估安全阶段。
  - 风险-4: wasm 若与 native 使用不同 canonical payload 或签名前缀，会出现“浏览器签名永远被 runtime 拒绝”的协议漂移。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-TXAUTH-001 | STRAUTH-0/1/2A | `test_tier_required` | 专题 PRD/design/project 建档、transfer submit 鉴权实现、shared payload auth envelope 与 NodeRuntime 门禁 | 主链 Token 提交层签名化 |
| PRD-P2P-TXAUTH-002 | STRAUTH-1/2A/3 | `test_tier_required` | runtime/control-plane/node 定向测试，覆盖缺签名/错签名/账户不匹配与 payload 层无 proof 拒绝 | 错误语义、阻断门禁与 QA 回归 |
| PRD-P2P-TXAUTH-003 | STRAUTH-2B/3 | `test_tier_required` | project 路线图、devlog 与模块入口回写，确认 controller-bound 资产动作含 restricted admin registry update 的治理 signer allowlist/ceremony 仍待完成 | producer 阶段判断与后续优先级 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-TXAUTH-001 | 先关闭公开 transfer submit 的无签名入口 | 一次性同时改 transfer/claim/genesis/treasury/restricted-admin-registry | 当前公开攻击面首先在 transfer submit；先关最暴露入口能最快实质降低风险。 |
| DEC-P2P-TXAUTH-002 | `from_account_id` 绑定为 `oc:pk:<public_key_hex>` | 继续接受任意字符串账户并只验“有签名即可” | 当前唯一已有明确公钥到账户派生语义的主链账户模型就是 `oc:pk:`。 |
| DEC-P2P-TXAUTH-003 | runtime 请求层先验签，再进入余额/nonce 预检 | 继续沿用“先业务预检，最后再验签” | 未通过签名的请求不应消耗任何资产语义校验路径。 |
| DEC-P2P-TXAUTH-004 | 先把 signed transaction model 上提到 shared payload / NodeRuntime 层，再继续补治理控制绑定 | 为了找不到公开入口而暂缓 claim/genesis/treasury/restricted-admin-registry 的提交层签名化 | 所有未来 submit surface 都会汇合到 `ConsensusActionPayloadEnvelope` 与 `NodeRuntime`，先收口汇合点收益最高。 |
| DEC-P2P-TXAUTH-005 | `genesis/treasury/restricted-admin-registry` 现阶段先要求 signed controller metadata，不假装已完成 governance allowlist | 直接把 signed metadata 视为治理安全闭环完成 | 真实 controller slot、signer list 与 ceremony 仍在 freeze sheet/治理专题里待绑定，不能提前升级口径。 |
| DEC-P2P-TXAUTH-006 | `UpdateRestrictedStarterClaimAdminRegistry` 复用 `ecosystem_pool` treasury controller slot 的正式钱包治理路径，而不是另起模拟内 proposal 真值 | 继续让 restricted grant admin roster 依赖 agent/proposal；或另起一套独立 admin signer 配置 | restricted starter grant 的发放与回收本质上属于主链资产控制面；复用已有 controller slot + signer policy 能避免分裂治理真值，并把 liveops/admin 轮换收回统一的钱包级签名治理。 |
