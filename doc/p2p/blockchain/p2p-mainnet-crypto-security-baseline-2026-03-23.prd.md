# oasis7 主链/共识密码学安全基线评估（2026-03-23）

- 对应设计文档: `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.project.md`

审计轮次: 1
## 1. Executive Summary
- Problem Statement: oasis7 已经落地 `ed25519`、`HMAC-SHA256`、P2P writer allowlist 与 viewer 签名鉴权，但尚未形成“当前密码学安全到底处于什么等级、能否对标主流公链、哪些缺口会阻断创世与对外口径”的统一评估。若继续只凭局部实现感受决策，容易把“原语可用”误判成“系统已达 mainnet-grade”。
- Proposed Solution: 建立一份 producer-owned 密码学安全基线评估 PRD，对签名原语、交易授权、账户地址模型、节点/复制鉴权、密钥托管、治理 signer、创世执行控制七个面向做统一分级，并输出当前 verdict、硬阻断项与 mainnet-ready 必做项。
- Success Criteria:
  - SC-1: 评估矩阵完整覆盖 `signing primitives / transaction authorization / account model / network authorization / key custody / governance signer / genesis execution` 七个面向。
  - SC-2: 当前总 verdict 明确写成 `not_mainnet_grade`，且给出可追溯代码/文档依据，不允许口头留白。
  - SC-3: 至少固定 4 个 release blocker：主链 Token 转账未签名授权、生产级 keystore 缺失、治理 finality signer 仍含本地固定 seed 路径、创世 recipient/controller 仍未完成真实绑定。
  - SC-4: 形成 producer 可直接执行的优先级路线：`P0 签名交易模型 -> P1 signer/keystore 硬化 -> P2 创世 signer ceremony + QA gate`。
  - SC-5: 对外口径边界明确：在上述 P0 blocker 收口前，不得宣称“对标主流公链安全”或 `mainnet-grade`.

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要决定当前项目能否使用“主流公链级安全”口径，以及创世前的真正优先级。
  - `runtime_engineer`：需要知道哪些密码学能力已经够用，哪些仍停留在 preview-grade。
  - `qa_engineer`：需要把安全结论变成可验证的 `pass/block` 门禁，而不是主观判断。
  - 治理/金库维护者：需要知道当前创世地址、控制账户和 signer ceremony 的风险等级。
  - `liveops_community`：需要知道对外能说到哪里，哪些安全口径不能提前承诺。
- User Scenarios & Frequency:
  - 创世前安全评审：每次准备推进创世地址绑定、Token mint 或对外强化安全口径时执行。
  - 版本阶段升级：从 `limited playable technical preview` 向更高可信度阶段升级前执行。
  - 对外口径复核：每次出现“能否对标主流公链”类问题时复用。
- User Stories:
  - PRD-P2P-CRYPTO-001: As a `producer_system_designer`, I want one explicit security baseline verdict, so that external claims and internal priorities are grounded in the same truth.
  - PRD-P2P-CRYPTO-002: As a `runtime_engineer`, I want a gap matrix between current implementation and mainstream public-chain expectations, so that hardening work can be sequenced correctly.
  - PRD-P2P-CRYPTO-003: As a `qa_engineer`, I want blocker conditions and audit evidence rules, so that security readiness can be turned into a release gate instead of a subjective impression.
- Critical User Flows:
  1. Flow-P2P-CRYPTO-001: `收集代码真值 -> 按七个安全面向归类 -> 形成 green/yellow/red 结论 -> producer 给出总 verdict`
  2. Flow-P2P-CRYPTO-002: `有人提出“可对标主流公链” -> 对照基线文档检查 blocker -> 若 blocker 未清零则直接驳回该口径`
  3. Flow-P2P-CRYPTO-003: `准备进入创世地址绑定 -> 先检查签名交易模型与 keystore 状态 -> 若仍为 not_mainnet_grade 则先转入 P0 安全专题`
  4. Flow-P2P-CRYPTO-004: `runtime/QA 完成一个安全硬化专题 -> 回填矩阵状态 -> 重新评估总 verdict 是否可升级`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 原语层盘点 | `primitive_name/scope/source_of_truth/status` | 盘点 `ed25519`、`HMAC-SHA256`、threshold receipt signer 等现状 | `unknown -> inventoried` | 仅按仓库代码真值判定，不按口头设计补齐 | `runtime_engineer` 提供真值，producer 收口 |
| 交易授权评估 | `action_surface/auth_required/auth_present/verdict` | 检查 `TransferMainToken`、`ClaimMainTokenVesting`、创世动作是否具备签名交易模型 | `inventoried -> pass/block` | 若资产动作缺签名授权，则该面向直接 `block` | producer/QA 联审 |
| 账户地址模型评估 | `account_id_model/address_derivation/wallet_parity/verdict` | 检查账户是否为内部字符串、是否已有成熟钱包地址体系 | `inventoried -> pass/risk` | `internal string only` 不得判为主流公链同级 | producer 判级 |
| 网络/复制鉴权评估 | `writer_allowlist/request_signature/enforcement_mode/verdict` | 检查 replication writer allowlist 与 fetch request 签名路径 | `inventoried -> pass/risk` | 具备签名 + allowlist 可给 `preview_pass`，但不代表整体 mainnet-grade | `runtime_engineer` 提供真值 |
| 密钥托管评估 | `key_storage/location/plaintext/external_signer_support/verdict` | 检查节点/治理/创世 signer 是否仍走本地明文 hex | `inventoried -> pass/block` | 明文 `config.toml` 不得判为生产级 keystore | producer/QA 联审 |
| 治理 signer 评估 | `signer_origin/rotation/revocation/seed_source/verdict` | 检查是否存在固定 seed、本地内建 signer 或不可审计轮换路径 | `inventoried -> pass/block` | deterministic local seed 仅能算 local/test convenience，不算 production governance signer | producer 判级 |
| 创世执行控制评估 | `recipient_binding/controller_binding/signer_policy/qa_status` | 检查 freeze sheet 是否仍有 `TBD_BEFORE_MINT` 或 `pending_binding` | `draft -> conditional/block/pass` | 任一创世 slot 未绑定则不得宣称 ready for mint | producer/QA 联审 |
| 对外口径门禁 | `claim_phrase/allowed_until_status/reject_reason` | 约束能否使用 `mainnet-grade`、`mainstream public chain security` 等说法 | `unchecked -> allowed/rejected` | 只要任一 P0 blocker 未关，就必须拒绝高级安全口径 | `liveops_community` 执行，producer 拍板 |
- Acceptance Criteria:
  - AC-1: 专题文档必须明确给出当前总 verdict：`not_mainnet_grade`。
  - AC-2: 必须把“原语层可用”与“系统级安全可对标主流公链”明确拆开，不允许混写。
  - AC-3: 必须列出当前至少 4 个硬阻断项，并为每项给出对应代码或现有文档依据。
  - AC-4: 必须明确 `viewer` 鉴权与 `replication` 鉴权属于局部正向信号，但不能覆盖 `main token` 交易授权缺失这一系统级 blocker。
  - AC-5: 必须明确 `oc:pk:<public_key_hex>` 当前仍属于 runtime 内部账户派生语义，不等同于成熟外部钱包地址体系。
  - AC-6: 必须明确本地固定 seed governance signer 与明文 `config.toml` keypair 不得计为主流公链生产级 signer/keystore。
  - AC-7: 必须输出一个 mainnet-ready 路线图，且第一个优先级必须是“主链 Token 资产动作签名交易模型”。
  - AC-8: 必须给出对外口径边界：在 P0 blocker 清零前，只能使用 `limited playable technical preview` 级别的安全表述，不得宣称“对标主流公链安全”。
- Non-Goals:
  - 不在本专题内直接实现签名交易模型、keystore 或 HSM/KMS。
  - 不替代第三方正式密码学审计或智能合约审计。
  - 不把经济模型、代币分配比例或运营激励重新定义一遍。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题是代码/文档真值评估，不涉及 AI 模型能力变更）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题不是新增协议，而是把现有实现按七个安全面向做真值归档。评估入口以 `runtime` 资产动作、`viewer` 玩家鉴权、`node`/`replication` 签名鉴权、治理 signer 与创世 freeze sheet 为主，最后由 producer 把“局部通过”收束成系统级 verdict。
- Integration Points:
  - `crates/oasis7/src/viewer/auth.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api.rs`
  - `crates/oasis7/src/consensus_action_payload.rs`
  - `crates/oasis7/src/runtime/events.rs`
  - `crates/oasis7/src/runtime/main_token.rs`
  - `crates/oasis7/src/runtime/world/governance.rs`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs`
  - `crates/oasis7_node/src/replication.rs`
  - `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若某条链路使用了 `ed25519`，但签名没有绑定到账户所有权或交易提交入口，则该链路只能判定为“原语存在”，不能判定为“资产安全闭环完成”。
  - 若某个专题文档已写“治理绑定/多签/地址绑定”，但 freeze sheet 仍是 `TBD_BEFORE_MINT`，则创世 readiness 只能给 `conditional` 或 `block`。
  - 若 replication/fetch request 已启用签名鉴权，但主链 transfer submit 仍接收未签名 JSON，则整体 verdict 必须保持 `not_mainnet_grade`。
  - 若 signer 来源依赖 deterministic local seed，仅允许在 local/test/preview 语境下计入“可运行”，不得在 mainnet-grade 评估中加分。
  - 若密钥仍明文保存在本地配置文件，即使能正常签名，也必须在 keystore 面向判 `block`。
  - 若后续补了签名交易模型，但未补 keystore/rotation/revocation/创世 signer ceremony，则最多只能升级到 `crypto-hardened preview candidate`，不能直接升级到 `mainnet_candidate`。
- Non-Functional Requirements:
  - NFR-P2P-CRYPTO-1: 所有 verdict 必须带来源依据；每个 red/yellow 项至少引用 1 个代码或正式文档真值。
  - NFR-P2P-CRYPTO-2: 资产动作安全评估必须覆盖 `InitializeMainTokenGenesis`、`ClaimMainTokenVesting`、`TransferMainToken` 与 treasury distribution 执行路径。
  - NFR-P2P-CRYPTO-3: 只要资产动作仍缺统一签名交易模型，总 verdict 必须保持 `not_mainnet_grade`。
  - NFR-P2P-CRYPTO-4: 只要生产 signer 仍依赖本地明文私钥配置，总 verdict 不得高于 `crypto-safe-enough-for-preview`。
  - NFR-P2P-CRYPTO-5: 对外口径门禁必须是 hard gate，而不是建议项；命中 blocker 时只能拒绝，不允许“酌情通过”。
  - NFR-P2P-CRYPTO-6: 安全路线图必须包含可执行优先级、owner role、验收层级与阻断原因，避免停留在泛泛而谈。
- Security & Privacy: 本专题只记录实现层安全能力与缺口，不暴露真实创世私钥、签名材料或敏感账户数据；若后续需要落真实 signer ceremony，只允许记录公钥、账户标识、阈值与审计摘要，不落私钥明文。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 完成安全基线评估、形成统一 verdict 与 blocker 列表。
  - v1.1: 补“主链 Token 资产动作签名交易模型”专题，并把 transfer/claim/genesis/treasury path 收口到统一交易授权。
  - v2.0: 补生产级 keystore / signer rotation / governance signer externalization / 创世 signer ceremony QA gate，再决定是否升级到 `mainnet_candidate`。
- Technical Risks:
  - 风险-1: 若只看 `ed25519`/allowlist 等局部实现，很容易误判整体安全等级，导致对外过度承诺。
  - 风险-2: 若创世地址绑定先行、而签名交易模型滞后，后续会把“地址已生成”误读成“资产权限已安全闭环”。
  - 风险-3: 若继续依赖本地明文密钥与 deterministic governance signer，任何“主流公链级安全”口径都会在审计时被直接打回。
  - 风险-4: 若把本专题当作审计替代品，会忽略实现整改与独立安全审计仍然是后续必需项。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-CRYPTO-001 | CRYPTO-0/1/2 | `test_tier_required` | 代码真值盘点、专题 PRD/design/project 建档、模块入口映射、文档门禁 | 安全基线 verdict 与口径治理 |
| PRD-P2P-CRYPTO-002 | CRYPTO-1/2/3 | `test_tier_required` | 交易授权/账户模型/keystore/governance signer/gensis freeze sheet 差距矩阵核对 | mainnet-ready 路线与工程优先级 |
| PRD-P2P-CRYPTO-003 | CRYPTO-2/3 | `test_tier_required` | blocker 列表、口径门禁、下一专题优先级与模块主追踪回写 | QA/producer 阻断与对外表述 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-CRYPTO-001 | 明确判定当前为 `not_mainnet_grade` | 用“接近主流公链”模糊表述带过 | 当前最关键的资产动作签名授权仍缺失，模糊结论会误导优先级与对外口径。 |
| DEC-P2P-CRYPTO-002 | 将“签名原语存在”与“系统级安全达标”拆成两个层次 | 只要用了 `ed25519` 就视为足够安全 | 主流公链安全差距主要在交易模型、托管与治理执行层，而不是原语名称。 |
| DEC-P2P-CRYPTO-003 | 优先补主链 Token 签名交易模型，再推进创世地址 ceremony | 先继续生成/绑定创世地址，再晚点补交易授权 | 若资产动作仍未收口统一签名交易模型，地址 ceremony 不是当前第一优先级。 |
| DEC-P2P-CRYPTO-004 | 把 deterministic local governance signer 和明文 config keypair 视作 preview convenience，不计入生产级加分 | 把它们作为生产 signer 基线接受 | 这类路径缺少正式托管、轮换、吊销与分权治理能力。 |
