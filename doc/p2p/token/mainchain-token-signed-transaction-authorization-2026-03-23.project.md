# oasis7 主链 Token 签名交易鉴权（项目管理文档）

- 对应设计文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.design.md`
- 对应需求文档: `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`

审计轮次: 3
## 任务拆解（含 PRD-ID 映射）
- [x] STRAUTH-0 (PRD-P2P-TXAUTH-001/002/003) [test_tier_required]: 新建“主链 Token 签名交易鉴权”专题 PRD / design / project，并接入 `doc/p2p` 模块主追踪。
- [x] STRAUTH-1 (PRD-P2P-TXAUTH-001/002) [test_tier_required]: 由 `runtime_engineer` 为 `POST /v1/chain/transfer/submit` 实现 `public_key + signature` 鉴权、`oc:pk:` 账户绑定、控制面请求结构同步与定向回归。
- [x] STRAUTH-2 (PRD-P2P-TXAUTH-001/003) [test_tier_required]: 由 `runtime_engineer` 继续把 `ClaimMainTokenVesting / InitializeMainTokenGenesis / DistributeMainTokenTreasury / UpdateRestrictedStarterClaimAdminRegistry / TopUpRestrictedStarterClaimLiveopsPool` 纳入统一 signed transaction envelope。
  - [x] STRAUTH-2A [test_tier_required]: 为 `ConsensusActionPayloadEnvelope` 增加主链 Token auth proof，并让 `NodeRuntime` 对 transfer/claim/genesis/treasury/restricted-admin-registry 在提交层强制验签。
  - [x] STRAUTH-2B [test_tier_required]: 将 controller-bound 资产动作的治理控制从“signed metadata”推进到正式 controller slot binding，并继续保留 signer allowlist / ceremony 后续任务。
    - [x] STRAUTH-2B1 [test_tier_required]: 为 genesis/treasury 建立正式 controller slot registry，并在 `NodeRuntime` 提交层按 `action/bucket` 绑定 `auth.account_id`。
    - [x] STRAUTH-2B2 [test_tier_required]: 将 controller slot 继续收口到本地配置 signer allowlist / threshold enforcement，并明确 ceremony / external signer 仍待后续专题。
    - [x] STRAUTH-2B3 [test_tier_required]: 将 `UpdateRestrictedStarterClaimAdminRegistry` 绑定到 `ecosystem_pool` treasury controller slot，并在 `NodeRuntime`/runtime 两侧统一要求 controller account 匹配、signer allowlist / threshold policy 通过。
    - [x] STRAUTH-2B4 [test_tier_required]: 将 `TopUpRestrictedStarterClaimLiveopsPool` 同样绑定到 `ecosystem_pool` treasury controller slot，使 dedicated liveops pool top-up 继续复用高权限 `2-of-3` signer policy，而不把 daily restricted grant CLI 混进 controller payload 细节。
- [x] STRAUTH-3 (PRD-P2P-TXAUTH-002/003) [test_tier_required + test_tier_full]: 由 `viewer_engineer` + `qa_engineer` 补齐 Web/native 转账签名提交流程、失败提示与更完整回归证据。
  - [x] STRAUTH-3A [test_tier_required]: 为 `oasis7_client_launcher` 的 Web/native 转账窗口补 signed request builder、本地 signer bootstrap 读取，以及 `oasis7_web_launcher` HTML bootstrap 注入。
  - [x] STRAUTH-3B [test_tier_full]: 产出 Web-first 闭环证据，至少覆盖一次 signed transfer 尝试与一次 signer/bootstrap 失败提示路径。

## 当前切片结论
- 已收口:
  - `TransferMainToken` 公开 HTTP submit 不再接受未签名请求。
  - `from_account_id` 已绑定到 `oc:pk:<public_key_hex>`。
  - `oasis7_web_launcher` 代理请求结构已同步到新字段集合。
  - `ConsensusActionPayloadEnvelope` 已支持 shared main-token auth proof，`NodeRuntime` 提交层已对 transfer/claim/genesis/treasury/restricted-admin-registry/liveops-pool-top-up 统一做 signed payload gating。
  - `InitializeMainTokenGenesis / DistributeMainTokenTreasury` 已进入正式 controller slot registry，submit-layer 不再接受任意 controller label。
  - `InitializeMainTokenGenesis / DistributeMainTokenTreasury` 已进入代码级 controller signer allowlist / threshold enforcement，submit-layer 会拒绝 policy missing、allowlist miss 与 threshold 不达标的 proof。
  - `UpdateRestrictedStarterClaimAdminRegistry` 已从模拟内 proposal proposer 模式重构为正式 controller-account 模式：submit-layer 与 world apply 都要求 `controller_account_id` 命中 `ecosystem_pool` treasury controller slot，并通过相同的 signer allowlist / threshold policy。
  - `TopUpRestrictedStarterClaimLiveopsPool` 已与 restricted admin registry update 一样绑定 `ecosystem_pool` treasury controller slot：submit-layer 与 world apply 都要求 `controller_account_id` 命中同一高权限 slot，并通过相同的 signer allowlist / threshold policy。
  - `oasis7_client_launcher` 的 Web/native 转账窗口已在提交前本地产出 `public_key/signature`，不再发送裸 transfer 请求。
  - `oasis7_web_launcher` 已在服务 HTML 时注入本地 signer bootstrap，使 wasm 端能够按同一协议产签。
  - `STRAUTH-3B` 已完成 Web-first 证据闭环：agent-browser 通过 wasm test hook 驱动 canvas-only 转账窗口，已验证一次页面侧 signed submit -> runtime `action_id=1` / tracked status `confirmed`，以及一次 bootstrap 缺失 -> 本地 `转账签名失败` 且无 transfer POST 的失败提示路径。
  - `viewer_engineer` 已修复两类真实 Web blocker：wasm `SystemTime/process::id()` 平台 panic，以及 wasm transfer canonical JSON 字节顺序与 runtime helper 不一致导致的 `invalid_signature`。
- 仍待完成:
  - genesis/treasury/restricted-admin-registry 仍缺 ceremony freeze、external signer、HSM/KMS 与更长期的 world-state / governance source of truth。
  - 生产级 keystore / signer rotation / external signer 专题。

## 依赖
- `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
- `doc/p2p/prd.md`
- `doc/p2p/project.md`
- `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_tests.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`
- `crates/oasis7/src/bin/oasis7_web_launcher/viewer_auth_bootstrap.rs`
- `crates/oasis7_client_launcher/src/transfer_auth.rs`
- `crates/oasis7_client_launcher/src/launcher_test_hook_web.rs`
- `crates/oasis7_client_launcher/src/transfer_window.rs`
- `crates/oasis7_client_launcher/src/transfer_window_web.rs`
- `crates/oasis7/src/consensus_action_payload.rs`
- `crates/oasis7/src/runtime/main_token.rs`
- `crates/oasis7_node/src/node_runtime_core.rs`
- `crates/oasis7_node/src/tests_action_payload.rs`
- `testing-manual.md`
- `doc/testing/evidence/mainchain-token-signed-transfer-web-validation-2026-03-23.md`

## 验收命令（本轮）
- `env -u RUSTC_WRAPPER cargo test -p oasis7 transfer_submit --bin oasis7_chain_runtime`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 submit_chain_transfer_remote --bin oasis7_web_launcher`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_node tests_action_payload -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_entry -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher transfer_auth -- --nocapture`
- `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher`
- `env -u RUSTC_WRAPPER cargo check -p oasis7_client_launcher --target wasm32-unknown-unknown`
- `env -u NO_COLOR trunk build --dist /tmp/oasis7-strauth3b-web-launcher-4`
- `env -u RUSTC_WRAPPER cargo check -p oasis7 --lib`
- `env -u RUSTC_WRAPPER cargo check -p oasis7 --bin oasis7_chain_runtime --bin oasis7_web_launcher`
- `env -u RUSTC_WRAPPER cargo check -p oasis7_node`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime::tests::governance::update_restricted_claim_admin_registry_ -- --nocapture`
- `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime::tests::agent_claims::controller_registry_update_can_enable_restricted_grant_admin_before_issue -- --nocapture`
- `./scripts/doc-governance-check.sh`
- `git diff --check`

## 状态
- 当前阶段: active
- 下一步: `STRAUTH-3` 已收口；另开专题把 ceremony / external signer / keystore 收成长期治理真值，并把当前本地 controller signer policy 升级为更长期的治理 source of truth，覆盖 genesis/treasury/restricted-admin-registry 全部 controller-bound 动作。
- 最近更新: 2026-03-23
