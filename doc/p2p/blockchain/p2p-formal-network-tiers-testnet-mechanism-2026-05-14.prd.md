# oasis7 正式网络分层与 testnet 机制（2026-05-14）

- 对应设计文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.project.md`

审计轮次: 2
## 1. Executive Summary
- Problem Statement: oasis7 现在已有 `shared_devnet/staging/canary` 的 shared release-train 最小机制，也已有 mainnet-grade readiness 文档，但两者之间还缺一层正式的公共主链式网络分层。结果是 `shared_devnet` 容易被误读成 testnet，而 `mainnet` 又容易被误读成“等 mainnet gates 文档齐了就能直接上线”。
- Proposed Solution: 冻结一份 producer-owned 的正式网络分层 PRD，明确 `local_devnet -> shared_devnet -> public_testnet -> mainnet` 的层级、各层 network manifest 真值、faucet/reset/validator/claims policy 边界，以及 repo-owned script/config skeleton，让后续 runtime、QA、liveops 都围绕同一套 tier contract 接线。
- Success Criteria:
  - SC-1: 明确冻结四层网络模型：`local_devnet`、`shared_devnet`、`public_testnet`、`mainnet`，并写清各层目标、访问方式、价值语义与允许 claims。
  - SC-2: `public_testnet` 必须拥有一套正式 manifest 字段集合，至少覆盖 `network_id/chain_id/genesis_ref/release_candidate_bundle_ref/bootstrap_peer_ref/public_rpc/explorer/faucet/reset_policy/validator_admission`。
  - SC-3: `mainnet` 必须明确绑定 `no faucet + no reset + frozen genesis + governance registry + MAINNET readiness gates`，不得与 `public_testnet` 共用“可随时重置”的语义。
  - SC-4: 仓库内必须落地 repo-owned skeleton：network-tier manifest create/validate 脚本、smoke、example manifests 与 `testing-manual` 入口。
  - SC-5: 本专题必须明确当前结论仍是 `specified_skeleton_only`；建档与脚本骨架不等于已存在 live `public_testnet` 或 live `mainnet`。
  - SC-6: 本专题必须补一份 repo-owned `public_testnet` live-candidate companion checklist/runbook，把 seven-lane readiness gate、owner、最小 evidence、canonical 命令与 claim boundary 冻结成可执行入口，避免“还差什么”只停留在聊天结论。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要把“什么时候叫 shared devnet，什么时候才叫 public testnet / mainnet”冻结成正式口径。
  - `runtime_engineer`：需要知道一个网络 tier 至少要固定哪些字段，后续 runtime/config 才能围绕单一 manifest 接线。
  - `qa_engineer`：需要知道 testnet/mainnet 的 promotion gate、reset/faucet 边界和 current verdict，避免把 rehearsal 当上线。
  - `liveops_community`：需要知道哪些 tier 可以公开给外部访问，哪些还只能作为团队内部共享轨道。
  - 外部测试者 / validator 候选：需要知道 public testnet 能做什么，不能把 testnet 资产、faucet 或节点准入误认为 mainnet 承诺。
- User Scenarios & Frequency:
  - 每次准备把 `shared_devnet` 之外的环境对外开放时执行一次。
  - 每次准备冻结公共 RPC、explorer、faucet 或 testnet reset policy 时执行一次。
  - 每次出现“这个环境算 testnet 还是 mainnet”类争议时复用。
- User Stories:
  - PRD-P2P-TIER-001: As a `producer_system_designer`, I want one formal network-tier model, so that shared release-train and mainnet readiness no longer float as disconnected concepts.
  - PRD-P2P-TIER-002: As a `runtime_engineer`, I want one machine-readable network-tier manifest schema, so that environment bootstrap and release pinning can wire to one source of truth.
  - PRD-P2P-TIER-003: As a `qa_engineer`, I want explicit public-testnet and mainnet gates, so that promotion decisions can block on missing faucet/reset/governance evidence.
  - PRD-P2P-TIER-004: As a `liveops_community`, I want claims boundaries tied to tier state, so that external communication stays behind execution truth.
- Critical User Flows:
  1. Flow-P2P-TIER-001: `producer` 先为某一网络层生成 `network_tier_manifest`，固定 tier、network id、genesis、candidate bundle、endpoint policy 与 claims boundary。
  2. Flow-P2P-TIER-002: `runtime_engineer` 以 `shared_devnet pass` 和 pinned bundle 为输入，提出 `public_testnet` manifest，声明 public RPC/explorer/faucet/reset/validator admission。
  3. Flow-P2P-TIER-003: `qa_engineer` 对照 manifest 与 gate 结论决定该 tier 是 `planned/specified_skeleton_only/rehearsal/live` 中哪一档，并决定是否允许 public claims。
  4. Flow-P2P-TIER-004: `mainnet` 申请时，系统必须同时检查 `public_testnet exit review + MAINNET-1~4 + frozen genesis + no-reset commitment`；任一缺失即阻断。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Network tier manifest | `tier/status/network_id/chain_id/release_candidate_bundle_ref/genesis_ref/bootstrap_peer_ref/endpoint_policy/validator_policy/token_policy/claims_policy/promotion_policy` | 为一个网络层冻结机器可读真值 | `draft -> validated -> referenced` | 字段缺失或 tier 语义冲突时不得通过 validate | `producer_system_designer` 冻结语义，`runtime_engineer` 负责技术接线 |
| `shared_devnet` boundary | `visibility=shared_operator`、`value_semantics=preview`、`faucet_mode=none|operator_grant` | 继续作为 shared release-train 轨道，不对外冒充 public testnet | `planned -> rehearsal -> partial/pass` | 即便已有 shared access，也不自动等于 public RPC/faucet/testnet claim | `runtime_engineer`/`qa_engineer` 维护 |
| `public_testnet` policy | `visibility=public`、`value_semantics=testnet`、`faucet_mode=guarded_testnet_faucet`、`reset_policy=resettable`、`validator_admission=allowlist_or_governed_candidate` | 对外开放 public testnet，并明确它仍可重置、资产无 mainnet 价值承诺 | `planned -> specified_skeleton_only -> rehearsal -> live` | 没有 public RPC/explorer/faucet/reset policy 时不得叫 public testnet | `liveops_community` 执行公开面，producer 审批 |
| `mainnet` policy | `visibility=public`、`value_semantics=production`、`faucet_mode=none`、`reset_policy=frozen`、`validator_admission=governance_registry_only` | 宣告正式价值网络 | `planned -> gated -> live` | 只有 `MAINNET-1~4`、frozen genesis、no-reset commitment 全部具备时才允许 | `producer_system_designer` 审批，`qa_engineer` 阻断 |
| Claims gate | `allowed_claims/denied_claims/required_gates` | 根据 tier 决定允许哪些公开口径 | `draft -> enforced` | `shared_devnet` 与 `public_testnet` 默认 deny `mainnet-grade live network` | `liveops_community` 执行，producer 审批 |
- Acceptance Criteria:
  - AC-1: 本专题必须落地 PRD / design / project，并接入 `doc/p2p/prd.md`、`doc/p2p/project.md`、`doc/p2p/prd.index.md` 与 `testing-manual.md`。
  - AC-2: 本专题必须明确 `local_devnet -> shared_devnet -> public_testnet -> mainnet` 四层模型，且明确 `shared_devnet != public_testnet`。
  - AC-3: `public_testnet` 的最小 manifest 字段必须至少包含 `network_id`、`chain_id`、`release_candidate_bundle_ref`、`genesis_ref`、`bootstrap_peer_ref`、`rpc_ref`、`explorer_ref`、`faucet_ref`、`reset_policy`、`validator_admission`、`allowed_claims` 与 `denied_claims`。
  - AC-4: `mainnet` 的最小 manifest 字段必须至少包含 `network_id`、`chain_id`、`release_candidate_bundle_ref`、`genesis_ref`、`bootstrap_peer_ref`、`rpc_ref`、`reset_policy=frozen`、`faucet_mode=none`、`validator_admission=governance_registry_only`，并把 `MAINNET-1~4` 写入 required gates。
  - AC-5: 仓库内必须新增 repo-owned `scripts/network-tier-manifest.sh` 与 smoke，并提供 `shared_devnet/public_testnet/mainnet` 三份 example manifests。
  - AC-6: `testing-manual.md` 必须新增正式 network-tier skeleton 入口，并明确当前仍无 live `public_testnet` / `mainnet`。
  - AC-7: 本专题必须明确 `public_testnet` 的资产与 faucet 只用于 rehearsal/test surface，不得被写成 `OC` 的 mainnet 价值承诺。
  - AC-8: 本专题必须明确当前 verdict 为 `specified_skeleton_only`；本轮不部署真实公共 testnet，不提升 public claims，不宣称 mainnet ready。
  - AC-9: 必须新增 companion runbook，至少列出 `shared_devnet_pass/public_rpc_ready/explorer_public_ready/faucet_guard_ready/reset_policy_announced/runtime_bootstrap/claims_boundary_review` 七条 lane 的 owner、最小 evidence、阻断条件、建议执行顺序与 canonical 命令，并明确在真实证据补齐前仍只能维持 `specified_skeleton_only`。
- Non-Goals:
  - 不在本专题内直接部署 live `public_testnet` 或 live `mainnet`。
  - 不在本专题内落 runtime 的多 network profile 切换实现。
  - 不在本专题内修改共识、经济模型或交易协议。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 正式网络分层位于现有 `shared network / release train minimum` 之上。`shared_devnet/staging/canary` 继续承担内部共享编排与候选验证；新的 `public_testnet` 作为第一个可公开访问、可 reset、带 faucet 的 rehearsal 网络；`mainnet` 作为 frozen genesis、no-reset、no-faucet、受治理准入约束的正式价值网络。四层通过同一 `network_tier_manifest` schema 固定 tier 语义，而不是继续把环境名称写散在 README、运行手册和口头说明中。
- Integration Points:
  - `doc/p2p/blockchain/p2p-mainstream-public-chain-testing-benchmark-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-shared-network-release-train-minimum-2026-03-24.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-mainnet-public-claims-policy-2026-03-23.prd.md`
  - `doc/p2p/network/p2p-mainnet-private-reachability-architecture-2026-04-01.prd.md`
  - `doc/p2p/blockchain/p2p-formal-network-tiers-testnet-mechanism-2026-05-14.runbook.md`
  - `testing-manual.md`
  - `scripts/release-candidate-bundle.sh`
  - `scripts/shared-network-track-gate.sh`
- Edge Cases & Error Handling:
  - 若一个环境只有 shared operator access、没有 public RPC/explorer/faucet，则最多仍是 `shared_devnet`，不能叫 `public_testnet`。
  - 若 `public_testnet` 没有 reset policy 或默认宣称“永不重置”，则必须判定 schema 语义冲突，避免与 mainnet 边界重叠。
  - 若 `mainnet` manifest 仍带 `faucet_mode != none`、`reset_policy != frozen` 或缺 `MAINNET-*` gates，则 validate 必须失败。
  - 若 `public_testnet` 允许 claim phrase 包含 `mainnet live`、`production OC settlement` 之类语句，则必须判为 policy 越界。
  - 若 tier manifest 引用的 candidate bundle、genesis 或 bootstrap ref 为空，则该 tier 只能停留在 `draft`，不得进入 release review。
- Non-Functional Requirements:
  - NFR-P2P-TIER-1: network tier manifest 必须保持 ASCII JSON，可在 repo 中追踪并可由脚本 create/validate。
  - NFR-P2P-TIER-2: `public_testnet` 与 `mainnet` 的 validate 必须显式检查 tier 语义冲突，不能只做字段存在性校验。
  - NFR-P2P-TIER-3: 任何 `mainnet` manifest 都不得允许 `resettable`、`guarded_testnet_faucet` 或 `value_semantics=testnet`。
  - NFR-P2P-TIER-4: 在 live `public_testnet` 尚未建立前，对外文档不得把示例 manifest、skeleton script 或 `shared_devnet` rehearsal 当成 public availability 证据。
  - NFR-P2P-TIER-5: tier manifest 中只允许记录 public refs、bundle refs 与政策字段，不得落私钥、助记词或 operator 私密登录信息。
- Security & Privacy: 本专题只冻结网络层级真值与验证约束，不新增密钥管理方案。凡涉及 signer、custody、genesis freeze 的敏感部分，继续由既有 `MAINNET-*` 主题约束，禁止把任何私密凭据写入 manifest 示例或测试文档。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 冻结四层网络模型、manifest schema、claims boundary 与 repo-owned skeleton。
  - v1.1: 把 `shared_devnet pass -> public_testnet planned` 的 promotion gate 与 runbook 正式接线。
  - v1.2: 落第一轮 public testnet rehearsal，补齐 public RPC/explorer/faucet/reset evidence。
  - v2.0: 在 `public_testnet` exit review 与 `MAINNET-1~4` 全绿后，再讨论 mainnet manifest 激活。
- Technical Risks:
  - 风险-1: 如果继续把 `shared_devnet` 和 `public_testnet` 混用，团队会在 claims、faucet、reset 承诺上持续越界。
  - 风险-2: 如果 `mainnet` 没有单独 schema 约束，后续很容易把 preview/testnet 运行习惯直接带进正式网络。
  - 风险-3: 如果只有文档没有 manifest skeleton，后续 runtime/liveops 会再次把 tier 真值散落到各自脚本里。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-TIER-001 | TIER-0 | `test_tier_required` | 专题 PRD/design/project 建档并回写模块主追踪 | p2p 模块正式网络口径 |
| PRD-P2P-TIER-002 | TIER-1 | `test_tier_required` | `network-tier-manifest.sh` create/validate + example manifests + smoke | network tier config skeleton |
| PRD-P2P-TIER-003 | TIER-2/3 | `test_tier_required` | public testnet / mainnet gate 字段、claims boundary 与 `testing-manual` 入口冻结 | QA / liveops promotion 边界 |
| PRD-P2P-TIER-004 | TIER-3/4 | `test_tier_required` | current verdict、promotion prerequisites 与 deny claims 回写 | 对外口径与后续 mainnet 进入条件 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-TIER-001 | 保留 `shared_devnet`，在其之上新增 `public_testnet` 与 `mainnet` 正式层 | 直接把 `shared_devnet` 改名为 `testnet` | `shared_devnet` 当前是团队共享 integration/release 轨，不等于公共 testnet。 |
| DEC-P2P-TIER-002 | 先冻结 repo-owned manifest schema 和示例，再推进 runtime/ops 接线 | 先做 live public testnet，之后再补 schema | 没有 manifest 真值，后续 public tier 很容易继续靠口头描述推进。 |
| DEC-P2P-TIER-003 | `mainnet` 必须单独校验 `no faucet + frozen reset + MAINNET-* gates` | 让 mainnet 复用 public testnet schema，只靠额外注释区分 | 正式价值网络不能依赖“大家知道这是 mainnet”的口头约定。 |
