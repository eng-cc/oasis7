# oasis7 主网安全、治理与创世就绪度

- 对应设计文档：`doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.design.md`
- 对应GitHub Issue/Project task truth：GitHub Issue / GitHub Project

## 目标与权威边界

本专题是 P2P 专业域关于系统级安全、治理 signer、创世与主网进入条件的稳定 authority。它吸收 2026-03 的 crypto baseline、MAINNET readiness、production signer custody、governance signer externalization 与 genesis ceremony 专题；那些历史三件套不再作为现行入口。

本专题不宣布主网、公开发售、production settlement、mint readiness、production custody 或 public launch。当前公开状态仍以根 `README.md` 为准；网络层级以 `formal-network-tiers-testnet-mechanism.*` 为准；玩家承诺不在本专题中定义。

## 范围

覆盖系统级安全判定、signer custody、registry-first governance、genesis/ceremony/QA 阻断条件及其专业验证入口；不覆盖代码实现、实际运维执行、公开承诺或玩家产品设计。

## 接口 / 数据

`governance_finality_signer_registry`、`governance_validator_admissions`、`governance_main_token_controller_registry`、validator membership/governed stake/finality signer binding、genesis binding status 与 QA evidence verdict 是本专题的专业合同字段；私钥、seed 与助记词禁止写入。

## 当前安全与阶段判定

- 当前系统级判定保持 `not_mainnet_grade`；可运行的局部签名、allowlist、registry 或测试证据不能单独升级该判定。
- `public_testnet` 是 controlled、resettable、non-mainnet 的测试层级；它不承诺 production settlement、mainnet 或公开发行。
- 创世仍为 `not_mint_ready`：`logic_frozen_address_binding_pending`、`TBD_BEFORE_MINT`、`pending_binding`、`ready_pending_address_binding` 任一状态，未完成 ceremony evidence，或缺少 QA `pass` 都必须阻断。
- 主网或 mint 口径仅能在相应技术、治理、QA 与公开口径权威同时给出证据后重新评估；本专题不替代该决定。

## 现行合同

### 资产与密码学基线

- `POST /v1/chain/transfer/submit` 要求 normalized ed25519 `public_key` 和 `signature`；`from_account_id` 必须等于 `oc:pk:<normalized_public_key_hex>`。缺失/无效签名或账户不匹配必须在余额、nonce 和 consensus submit 前拒绝。
- `MainTokenActionAuthProof` 同时进入 `ConsensusActionPayloadEnvelope`，并在 shared `NodeRuntime` submit layer 对受保护 main-token actions 重验。有效请求仍执行 amount、same-account、balance 与 nonce-replay 规则。
- `oasis7_client_launcher` 的 native 与 wasm 转账入口必须先按同一 canonical `TransferMainToken` payload 与域前缀生成 ed25519 `public_key/signature`，再提交；wasm 通过 `oasis7_web_launcher` 服务 HTML 时注入的 `window.__OASIS7_VIEWER_AUTH_ENV` 读取受信本地 signer bootstrap。
- bootstrap 缺失或仅有 public/private key 任一侧时，客户端必须在本地明确失败并阻止 transfer POST；不得静默降级为 unsigned submission。该 local env/config/HTML bootstrap 仅是便利路径，不构成钱包托管、生产 keystore 或 custody。
- 上述完成态不等于 custody 或 mainnet readiness，也不替代系统级多证据判定。
- Node/replication 的签名与 allowlist 是局部防护信号；系统级评估仍须同时检查 custody、治理、创世和验证证据。
- 所有系统级 verdict 必须能回溯到当前代码、专业文档或测试证据；不得以历史专题的完成复选框替代当前核验。

### Signer custody 与治理真值

- local/dev/preview 的 config、env、HTML bootstrap 或 deterministic seed 只能作为便利路径，不能被描述为 production custody。
- production 目标要求受控 signer material、明确的 rotation、revocation、审计留痕与环境策略；私钥、seed 和助记词不得进入仓库或本专题。
- runtime 以 world-state governance registry 为治理 finality signer、validator membership、governed stake 与 controller policy 的优先真值。registry 存在时不得退回 local seed/config 作为 production truth。
- validator/finality admission 保持 `apply -> approved_candidate -> probation_ready -> active -> revoke/rotate` 生命周期。准入、activation、stake-weighted quorum 与 finality signer binding 必须由 world-state 及其治理动作验证。
- genesis import 只适用于空 execution world；已有 snapshot/journal world 必须走专用 registry migration/import，不得覆盖或伪造恢复状态。

### 创世、ceremony 与 QA

- 创世 freeze 必须同时满足 slot registry、bucket execution、recipient/controller binding、signer policy、ceremony checklist 和 QA evidence bundle。
- evidence 只能记录公钥、账户绑定、threshold、审批与 QA verdict；不得记录任何私密 signer material。
- `conditional_draft_only`、缺失 evidence 或未通过 QA 均为阻断，不能用历史规格 gate 完成态绕过。

## 主网进入条件与非目标

主网进入至少需要：受控 custody、registry-first governance、可审计的 rotation/revocation/failover、冻结创世与 QA pass、formal network-tier 的 mainnet prerequisites，以及独立的产品/公开口径复核。任何单一 topic 或 rehearsal 成功都不构成主网进入授权。

本专题不定义 network-tier manifest、public endpoint、faucet、release procedure、实际 mint、密钥托管后端或运行时实现。它也不替代 QA 放行、LiveOps 对外表述或产品真值。

## 里程碑

- M1：吸收历史 security/governance/genesis authority，保留负向 readiness 标记。
- M2：由对应专业角色持续验证 custody、governance、genesis 与 network-tier 前置条件。

## 风险

- 将规格完成态误报为 production/mainnet/mint 完成态。
- 用 local seed/config 或 rehearsal 证据替代 registry、ceremony、QA 或 tier gate。

## 验证与追溯

- 文档迁移后应通过 `./scripts/doc-governance-check.sh`、`./scripts/readme-link-check.sh` 与旧源路径零引用扫描。
- 语义回归至少抽检 asset authorization、governance registry drill 与 network-tier manifest smoke；这些是合同核验，不能被解释为 mainnet readiness pass。
- 历史完成过程由 Git history 与 GitHub task evidence 追溯；本稳定专题只承载当前合同、阻断条件和后续验证入口。
