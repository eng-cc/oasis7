# oasis7 主链 mainnet-grade readiness 硬化路线（2026-03-23）

- 对应设计文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.project.md`

审计轮次: 1
## 1. Executive Summary
- Problem Statement: `STRAUTH-3` 已经收口公开 transfer submit、shared signed payload 与 Web-first QA 证据，但 oasis7 仍缺生产级 keystore / signer custody、治理 finality signer 外部化与创世 signer ceremony freeze gate。当前若不把剩余 `P1/P2` blocker 单独立项，团队容易把“签名交易已做完”误判成“整体已接近主流主链生产级安全”。
- Proposed Solution: 新建 producer-owned 的 mainnet-grade readiness 硬化路线 PRD，把 `P1/P2` 剩余缺口拆成可执行的 readiness gate，明确每一项的 owner、输入、完成定义、验证方式与对外口径约束，并固定在全部闭环前 oasis7 仍维持 `not_mainnet_grade` verdict。
- Success Criteria:
  - SC-1: 明确当前阶段定义为 `crypto-hardened preview`，且在本专题内不得把 `STRAUTH-3` 完成误写成 `mainnet-grade`。
  - SC-2: 形成 4 个正式 readiness gate：`production keystore/signer custody`、`governance finality signer externalization`、`genesis recipient/controller binding freeze`、`signer ceremony + QA/public-claims gate`。
  - SC-3: 每个 gate 都具备可验证的 `current_state/target_state/blocked_by/owner/test_tier` 定义，不允许继续停留在口头 TODO。
  - SC-4: 明确 public claims policy：在四个 readiness gate 全部通过前，只允许使用 `limited playable technical preview` 与 `crypto-hardened preview` 口径。
  - SC-5: 输出 producer 可直接推进的执行顺序：`P1 keystore/signer custody -> P1 governance signer externalization -> P2 genesis freeze/ceremony -> P2 final re-evaluation`。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要把“还差什么才能靠近主流主链生产级安全”从聊天判断变成正式 release gate。
  - `runtime_engineer`：需要知道当前 signed transaction model 之后，下一阶段到底补 keystore 还是继续补治理 signer。
  - `qa_engineer`：需要把 signer custody / ceremony / public-claims 变成可执行的阻断清单，而不是模糊口径。
  - `liveops_community`：需要知道对外哪些安全说法仍禁止使用，避免过度承诺。
  - 治理/金库维护者：需要知道何时才能冻结创世 recipient/controller/signer 真值，以及 freeze 之后的 QA gate 是什么。
- User Scenarios & Frequency:
  - `STRAUTH-3` 收口后立即执行一次 readiness 复盘，防止错误升级安全阶段。
  - 每次准备推进创世参数 freeze、controller binding 或 signer ceremony 前执行一次 readiness gate 复核。
  - 每次出现“现在是不是已经对标主流主链”类判断时复用。
- User Stories:
  - PRD-P2P-MAINNET-001: As a `producer_system_designer`, I want the remaining post-STRAUTH blockers turned into formal readiness gates, so that the roadmap after signed transactions is executable.
  - PRD-P2P-MAINNET-002: As a `runtime_engineer`, I want production signer custody and governance signer externalization to have separate done definitions, so that implementation work is not conflated.
  - PRD-P2P-MAINNET-003: As a `qa_engineer`, I want genesis freeze and signer ceremony to have explicit pass/block evidence rules, so that mint readiness is auditable.
  - PRD-P2P-MAINNET-004: As a `liveops_community`, I want public security claims tied to readiness gate status, so that the project does not overclaim before the hard blockers are gone.
- Critical User Flows:
  1. Flow-P2P-MAINNET-001: `STRAUTH-3 收口 -> producer 复核剩余 blocker -> 拆成 readiness gate -> 进入主模块 project 跟踪`
  2. Flow-P2P-MAINNET-002: `runtime 设计生产 signer custody -> 定义 key source/storage/sign flow -> QA 对照 gate 决定 pass/block`
  3. Flow-P2P-MAINNET-003: `治理 signer 从 local seed/config 迁出 -> 建立 validator/finality signer 准入/激活流程与 rotation/revocation/failover 规则 -> 通过后才能进入创世 ceremony`
  4. Flow-P2P-MAINNET-004: `冻结 genesis recipient/controller/signer sheet -> 执行 ceremony checklist -> QA 审核证据 -> producer 再做安全阶段重评估`
  5. Flow-P2P-MAINNET-005: `有人提出 mainnet-grade/public-chain-grade 口径 -> liveops 对照 readiness gate -> 任一未过即拒绝`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Readiness 总门禁 | `gate_id/current_state/target_state/owner/blocker_class` | 统一记录四个剩余 gate 的状态 | `identified -> scoped -> active -> qa_gated -> passed` | 任一 gate 未 `passed` 即保持 `not_mainnet_grade` | `producer_system_designer` 维护总 verdict |
| 生产级 signer custody | `signer_scope/key_source/storage_backend/sign_method/rotation_status` | 定义玩家/节点/治理 signer 的正式托管边界 | `local_bootstrap -> managed_signer -> audited` | 明文 `config.toml` 或页面注入 bootstrap 仅能算 preview path，不算 production | `runtime_engineer` 牵头，producer/QA 联审 |
| 治理 finality signer 外部化 | `governance_scope/signer_origin/rotation/revocation/failover_status` | 去掉 deterministic local seed 与本地固定 signer 依赖 | `local_seed -> externalized -> rotated -> revocable` | 若 rotation/revocation 任一缺失，则该 gate 不得通过 | `runtime_engineer` 牵头 |
| 创世 freeze 真值表 | `slot_id/recipient/controller_account/signer_policy/freeze_status` | 冻结 `recipient/controller/signer policy` 最终值 | `draft -> bound -> frozen -> ceremony_ready` | 仍含 `TBD_BEFORE_MINT` 或 `pending_binding` 时必须阻断 | `producer_system_designer` 牵头 |
| Ceremony + QA gate | `ceremony_run_id/participants/evidence_bundle/qa_verdict/public_claim_status` | 执行 signer ceremony、收证据、出 QA 结论 | `planned -> executed -> evidenced -> qa_passed -> claim_ready` | 无 evidence bundle 或 QA block 时，`claim_ready` 必须为 `false` | `qa_engineer` 持有 pass/block |
| Public claims policy | `claim_phrase/min_gate_status/reject_reason` | 对照 gate 决定是否允许某种口径 | `draft -> enforced` | `mainnet-grade/mainstream public-chain-grade` 需要四个 gate 全绿 | `liveops_community` 执行，producer 审批 |
- Acceptance Criteria:
  - AC-1: 专题必须明确写出：当前 verdict 仍为 `not_mainnet_grade`，当前阶段表述只能是 `limited playable technical preview` 与 `crypto-hardened preview`。
  - AC-2: 专题必须明确写出：`STRAUTH-3` 的完成只关闭了 signed transaction model 这一类 blocker，不代表 keystore、governance signer 与 genesis ceremony 已完成。
  - AC-3: 必须定义四个 readiness gate，并给出每个 gate 的 `owner/current_state/target_state/test_tier_required`。
  - AC-4: 必须明确生产级 signer custody 的最小完成定义至少包含 `external signer or managed keystore`、`rotation`、`revocation`、`audit trail` 四项。
  - AC-5: 必须明确治理 finality signer 外部化的最小完成定义至少包含 `no deterministic local seed in production path`、`validator/finality signer admission + activation rule`、`failover`、`rotation`、`revocation`。
  - AC-6: 必须明确创世 freeze 真值表在 `recipient/controller/signer policy` 三列全部冻结前，不得进入 mint-ready 判定。
  - AC-7: 必须明确 signer ceremony 的 evidence bundle 至少包含 `public keys/account bindings/threshold policy/operator checklist/QA verdict`，不得记录私钥明文。
  - AC-8: 必须定义 public claims gate，并明确 `mainnet-grade`、`mainstream public-chain-grade security`、`production mint ready` 三类说法在当前阶段全部禁止。
  - AC-9: 必须把下一阶段执行顺序固定为 `MAINNET-1 -> MAINNET-2 -> MAINNET-3 -> MAINNET-4`，避免团队回到“先做 ceremony 再补 custody”的错误顺序。
  - AC-10: 模块主 PRD/project/index/README 必须接入本专题，形成 `PRD-P2P-016 / TASK-P2P-034` 追踪链路。
- Non-Goals:
  - 不在本专题内直接实现 keystore、external signer service、HSM/KMS 或 ceremony automation。
  - 不把当前安全 verdict 升级为 `mainnet-grade` 或 `mint_ready`。
  - 不重新定义代币分配比例、贡献奖励台账或经济模型。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题位于 `p2p-mainnet-crypto-security-baseline` 与 `mainchain-token-signed-transaction-authorization` 之后，负责把剩余系统级安全 blocker 收成四个 readiness gate。它不新增链上协议，而是定义 `signer custody -> governance signer -> genesis freeze -> ceremony/public claims` 的硬化顺序、依赖和完成定义。
- Integration Points:
  - `doc/p2p/blockchain/p2p-mainnet-crypto-security-baseline-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-genesis-parameter-freeze-sheet-2026-03-22.md`
  - `crates/oasis7/src/bin/oasis7_chain_runtime/node_keypair_config.rs`
  - `crates/oasis7/src/runtime/world/governance.rs`
  - `crates/oasis7_node/src/types.rs`
  - `crates/oasis7_node/src/node_runtime_core.rs`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若项目已完成 signed transfer/claim/genesis/treasury gating，但 signer 仍来自本地 env/config bootstrap，则 readiness 只能记为 `crypto-hardened preview`，不得升级口径。
  - 若治理 signer 从 deterministic seed 切到 threshold allowlist，但真值仍停留在本地 `NodeConfig`，则只能记为 `partial`，不能视为长期治理 source of truth。
  - 若 freeze sheet 已填完 recipient/controller，但 signer policy 或 threshold 仍未冻结，则 `genesis_freeze` gate 仍为 `block`。
  - 若 ceremony 已执行但证据 bundle 不完整、缺 QA block/pass 结论或记录了敏感私钥材料，则必须判失败并重做。
  - 若 liveops/public docs 使用了高于当前 gate 状态允许的安全表述，必须按 release-risk 记录并回滚口径。
- Non-Functional Requirements:
  - NFR-P2P-MAINNET-1: readiness gate 的判定必须是 hard gate，不允许 `insufficient_data` 或“口头通过”。
  - NFR-P2P-MAINNET-2: 每个 gate 必须映射到明确 owner、依赖文档和 `test_tier_required`；ceremony gate 还必须定义 `test_tier_full`。
  - NFR-P2P-MAINNET-3: 在 `managed signer custody` 与 `governance signer externalization` 未完成前，`oasis7` 对外安全口径不得高于 `crypto-hardened preview`。
  - NFR-P2P-MAINNET-4: freeze sheet 与 ceremony 文档只允许沉淀公钥、账户绑定、阈值和审计摘要，不得落私钥、助记词或明文签名材料。
  - NFR-P2P-MAINNET-5: readiness 复评结果必须可追溯到 `PRD-ID -> Task -> QA verdict -> public claim policy`。
- Security & Privacy: 本专题只定义正式 signer custody、治理外部化和创世 ceremony 的治理边界；所有真实私钥、助记词、seed 与生产签名材料必须留在受控托管系统之外，不进入仓库或 PRD/devlog。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 建立 mainnet-grade readiness 硬化路线文档与模块主追踪映射。
  - v1.1: 完成 `MAINNET-1` 生产级 signer custody / keystore 规格收口。
  - v1.2: 完成 `MAINNET-2` 治理 finality signer 外部化与 rotation/revocation 规格收口。
  - v2.0: 完成 `MAINNET-3` 创世 freeze/ceremony/QA gate，进入 `MAINNET-4` 最终 re-evaluation 与 public claims policy 复核。
- Technical Risks:
  - 风险-1: 团队可能把 signed transaction model 完成误当作整体 mainnet-grade 候选，导致优先级错序。
  - 风险-2: 如果先做 ceremony、后补 custody/governance signer，创世真值会基于不稳定 signer 源再次返工。
  - 风险-3: 若治理 signer 外部化没有把 rotation/revocation 一起定义，生产故障或 key compromise 时没有正式恢复路径。
  - 风险-4: 若 public claims gate 没有单独冻结，社区与市场侧可能继续提前使用过高安全表述。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MAINNET-001 | MAINNET-0/1 | `test_tier_required` | readiness gate 文档建档、四个 gate 状态矩阵与主模块追踪回写 | producer 阶段判断与优先级治理 |
| PRD-P2P-MAINNET-002 | MAINNET-1/2 | `test_tier_required` | signer custody / governance signer externalization 完成定义、依赖与 blocker 梳理 | runtime signer 生产路径与治理可信度 |
| PRD-P2P-MAINNET-003 | MAINNET-3 | `test_tier_required` + `test_tier_full` | genesis freeze sheet 完整性、ceremony checklist、QA block/pass 模板 | mint readiness 与创世执行风险 |
| PRD-P2P-MAINNET-004 | MAINNET-4 | `test_tier_required` | public claims policy、阶段重评估与对外口径门禁回写 | liveops/public communication 风险 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-MAINNET-001 | 把 post-STRAUTH 剩余缺口单独立成 readiness hardening 主题 | 继续沿用安全基线/签名交易专题里的零散后续事项 | 当前 blocker 已经从“有没有签名交易”转成“如何达生产治理与托管”，需要独立 owner 和 gate。 |
| DEC-P2P-MAINNET-002 | 执行顺序固定为 custody -> governance signer -> genesis freeze/ceremony -> re-evaluation | 先做创世 ceremony，再回头补生产 signer 来源 | 如果 signer 来源不稳定，ceremony 结果不具备长期可信度。 |
| DEC-P2P-MAINNET-003 | 当前阶段口径固定为 `limited playable technical preview` + `crypto-hardened preview` | 以“接近 mainnet-grade”模糊描述替代 | 当前 P1/P2 blocker 仍实质存在，模糊口径会误导制作决策与外部预期。 |
