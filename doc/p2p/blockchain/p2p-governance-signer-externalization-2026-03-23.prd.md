# oasis7 治理 signer 外部化与轮换门禁（2026-03-23）

- 对应设计文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/p2p-governance-signer-externalization-2026-03-23.project.md`

审计轮次: 1
## 1. Executive Summary
- Problem Statement: oasis7 已经把 execution world 内的 `governance_finality_signer_registry` / `governance_main_token_controller_registry` 接成 runtime 启动/恢复时的真值入口，但治理 signer 仍缺一条正式的 validator / finality signer 准入、激活、轮换和撤销流程。若新增节点仍靠人工改 env、本地 `NodePosConfig` 或口头审批，就仍不具备 production governance source of truth。
- Proposed Solution: 建立 `MAINNET-2` 专题 PRD，把治理 finality signer 与 controller signer 的外部化目标、source-of-truth 边界、failover、rotation、revocation、operator ownership，以及 validator / finality signer 准入流程一次性冻结下来，并明确生产真值直接上链，而不是继续停留在本地配置或链下 registry。
- Success Criteria:
  - SC-1: 明确覆盖两类治理 signer：`governance finality signer` 与 `main token controller signer`。
  - SC-2: 明确 deterministic local seed 与 `NodeConfig` 本地 signer policy 只能算 preview/local truth，不算 production governance truth。
  - SC-3: 给出治理 signer 外部化的最小完成定义：`on-chain source of truth`、`rotation`、`revocation`、`failover`、`operator ownership`。
  - SC-4: 形成可执行任务链，至少拆出 `inventory/source-of-truth/failover-policy/QA gate` 四个切片。
  - SC-5: 在本专题完成前，readiness 阶段仍保持 `crypto-hardened preview`。
  - SC-6: 冻结 validator / finality signer 的目标准入流程，至少覆盖 `apply -> approved_candidate -> probation_ready -> active -> rotate/revoke`。
  - SC-7: 明确外部开放的是 validator / finality signer 准入路径，controller signer 仍属于治理内部 appointment，不走公开申请。

## 2. User Experience & Functionality
- User Personas:
  - `producer_system_designer`：需要把“治理 signer 还差什么”从口头 TODO 变成正式门禁。
  - `runtime_engineer`：需要知道 finality signer 与 controller signer 各自的长期真值该落在哪里。
  - `qa_engineer`：需要把 failover/rotation/revocation 变成可阻断项。
  - `liveops_community`：需要知道 production 环境是否还允许 local governance signer path。
  - 治理/金库维护者：需要明确谁拥有治理 signer、谁负责轮换与失效恢复。
  - validator 候选运营者：需要知道要提交哪些材料、何时能进入正式 validator set，以及什么情况下会被拒绝或撤销。
- User Scenarios & Frequency:
  - 每次准备提升治理安全口径时执行。
  - 每次变更 finality signer 或 treasury/genesis controller signer 时复核。
  - 每次准备进入创世 freeze/ceremony 前执行。
- User Stories:
  - PRD-P2P-GOVSIGN-001: As a `producer_system_designer`, I want a formal inventory of all governance signer truths, so that local convenience paths are not mistaken for production governance.
  - PRD-P2P-GOVSIGN-002: As a `runtime_engineer`, I want externalized source-of-truth boundaries for finality and controller signers, so that future implementation has a single target.
  - PRD-P2P-GOVSIGN-003: As a `qa_engineer`, I want failover/rotation/revocation to be explicit gate conditions, so that governance signer readiness is testable.
  - PRD-P2P-GOVSIGN-004: As a 治理维护者, I want operator ownership and policy authority defined, so that signer updates are auditable.
  - PRD-P2P-GOVSIGN-005: As a validator 候选运营者, I want a formal apply/approve/activate workflow, so that becoming a validator/finality signer no longer depends on ad hoc env edits or verbal approval.
- Critical User Flows:
  1. Flow-P2P-GOVSIGN-001: `盘点 finality/controller signer 当前来源 -> 标记 local/config truth -> producer 冻结 production source-of-truth 目标`
  2. Flow-P2P-GOVSIGN-002: `runtime 定义 on-chain signer source -> 把 local seed/config 退出 production path -> QA 复核 failover/rotation/revocation`
  3. Flow-P2P-GOVSIGN-003: `治理 signer 发生 compromise/人员调整/设备替换 -> 按 revocation/rotation/failover 执行 -> 审计留痕`
  4. Flow-P2P-GOVSIGN-004: `准备进入创世 ceremony -> 检查 governance signer gate 是否已过 -> 未过则直接阻断`
  5. Flow-P2P-GOVSIGN-005: `候选 validator 提交 node identity/finality signer/public manifest -> producer/runtime/QA 审核 -> candidate 演练通过 -> activation epoch 冻结 -> governance registry 激活 -> runtime 通过 world-state registry 恢复有效 validator membership`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算/判定规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Governance signer inventory | `governance_scope/current_source/source_of_truth/verdict` | 盘点 finality/controller signer 真值来源 | `unknown -> inventoried -> classified` | deterministic seed 与 `NodeConfig` 本地 policy 只能判为 preview/local | `producer_system_designer` 收口 |
| Externalized truth target | `governance_scope/target_truth_store/signer_registry/update_authority` | 冻结生产级长期真值落点 | `classified -> target_defined` | 若 production truth 仍依赖 local seed/config 则不通过；当前选定目标为 `on-chain/world-state registry` | `runtime_engineer` 牵头 |
| Rotation policy | `rotation_owner/trigger/approval_path/evidence_id` | 定义 signer 更换与过渡规则 | `undefined -> planned -> gated` | 无 approval path 或无 evidence sink 视为未完成 | producer/runtime 联审 |
| Revocation policy | `revocation_trigger/disable_path/recovery_path` | 定义 compromise、离岗、节点失效时的停用与恢复 | `undefined -> planned -> gated` | 无快速 disable path 则 block | `runtime_engineer` 牵头 |
| Failover policy | `failover_scope/activation_rule/rejoin_rule` | 定义 signer 集合降级、恢复与最小阈值行为 | `undefined -> planned -> gated` | 不能在 signer 失效时保持治理可持续性，则不通过 | `qa_engineer` 联审 |
| Operator ownership | `operator_group/change_authority/audit_sink` | 冻结谁能改 signer policy、谁能批准轮换 | `draft -> enforced` | 无明确 owner 的治理 signer 不得进入 production | `producer_system_designer` 拍板 |
| Validator / finality signer admission | `candidate_id/node_id/finality_signer_public_key/operator_owner/public_manifest/activation_epoch` | 受理申请、审核、试运行并决定是否激活进入 validator set | `applied -> approved_candidate -> probation_ready -> active` | 只有进入 world-state registry 且 activation 生效后才能算正式 validator；手工 env 改动不算正式准入 | producer/runtime/QA 联审 |
| Controller signer appointment boundary | `slot_id/controller_scope/appointment_authority/public_manifest/audit_sink` | 明确 controller signer 只走治理内部 appointment，不走公开 validator 准入 | `draft -> enforced` | 若把 controller slot 当成公开 validator 申请入口，则直接判设计越界 | `producer_system_designer` 拍板 |
- Acceptance Criteria:
  - AC-1: 专题必须明确列出当前两类真实治理 signer 路径的 `registry-first + local fallback` 结构：finality 走 `governance_finality_signer_registry -> deterministic local seed fallback`，controller 走 `governance_main_token_controller_registry -> NodeConfig.main_token_controller_binding.controller_signer_policies fallback`。
  - AC-2: 必须明确其中 local fallback 只允许作为 local/preview 真值，不得作为 production governance truth。
  - AC-3: 必须分别为 `finality signer` 与 `controller signer` 写出长期 source-of-truth 目标与更新 authority；当前统一选定为 `on-chain/world-state registry`。
  - AC-4: 必须定义 `rotation`、`revocation`、`failover`、`operator ownership` 四类 gate；任一缺失则 `MAINNET-2` 不通过。
  - AC-5: 必须明确 production 环境禁止 deterministic local seed 参与治理 finality signer。
  - AC-6: 必须明确 production 环境禁止仅靠 `NodeConfig` 本地 policy 维护 controller signer 真值。
  - AC-7: 必须输出 `GOVSIGN-1~4` 任务链与 owner/test tier 映射。
  - AC-8: 模块主 PRD/project/index/README 与 readiness project 必须接入本专题。
  - AC-9: 必须冻结 validator / finality signer 的准入状态机、申请材料、审核角色与 activation 规则；`world-state registry` 生效前不得把候选节点算作正式 validator。
  - AC-10: 必须明确 controller signer 不属于公开 validator 申请路径，只能走治理内部 appointment / freeze / ceremony 流程。
- Non-Goals:
  - 本轮不直接实现外部治理 signer 服务或 world-state 治理存储。
  - 本轮不直接执行创世 ceremony。
  - 本轮不重复定义 signer custody/keystore，那属于 `MAINNET-1`。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 当前治理 signer 风险分成两层。runtime 现在已经支持在 execution world 存在 registry 时优先读取 `governance_finality_signer_registry` 与 `governance_main_token_controller_registry`，并覆盖本地 validator membership / signer binding / controller policy；但新增 validator/finality signer 仍缺少正式准入和 activation 流程。producer 当前已选定 `直接上链`，即长期 governance truth 以 `on-chain/world-state registry` 为目标，而不是链下 registry。`MAINNET-2` 的目标不是一次写完工程实现，而是先冻结长期治理真值、更新 authority、候选准入流程与失效恢复门禁。
- Integration Points:
  - `crates/oasis7/src/runtime/world/governance.rs`
  - `crates/oasis7_node/src/types.rs`
  - `crates/oasis7_node/src/node_runtime_core.rs`
  - `crates/oasis7/src/consensus_action_payload.rs`
  - `doc/p2p/blockchain/p2p-mainnet-grade-readiness-hardening-2026-03-23.prd.md`
  - `doc/p2p/blockchain/p2p-production-signer-custody-keystore-2026-03-23.prd.md`
  - `doc/p2p/token/mainchain-token-signed-transaction-authorization-2026-03-23.prd.md`
  - `testing-manual.md`
- Edge Cases & Error Handling:
  - 若 finality signer 在无 registry 场景下仍由固定 seed label 推导，只能视为 local/test convenience，不得记为外部化完成。
  - 若 controller signer policy 仍完全依赖单机 `NodeConfig`，即使 threshold proof 可用，也只能记为 `partial`。
  - 若治理真值仍停留在链下 registry 或单机配置，而不是最终链上状态，也不能记为完成当前选定方案。
  - 若 rotation 有定义但无 revocation/disable path，仍必须判为未完成。
  - 若 failover 会破坏最小阈值或导致治理停摆，则必须 block。
  - 若 operator ownership 不明确，则任何 signer 更新都不得进入 production。
  - 若候选 validator 把 node identity key 与 finality signer key 混用，必须在审核阶段直接拒绝。
  - 若 candidate 节点未通过 reachability / sync / registry drill 就直接写入 active registry，必须判为流程越界。
  - 若有人尝试把 controller signer slot 作为公开 validator 申请入口，必须回退到治理 appointment 流程而不是继续执行。
- Non-Functional Requirements:
  - NFR-P2P-GOVSIGN-1: production governance signer truth 不得由 deterministic local seed 或单机本地配置单独承担；当前选定目标要求最终 truth 直接上链。
  - NFR-P2P-GOVSIGN-2: finality/controller signer 必须各自定义 rotation、revocation、failover 与 operator ownership。
  - NFR-P2P-GOVSIGN-3: 在本专题完成前，公开安全口径仍保持 `crypto-hardened preview`。
  - NFR-P2P-GOVSIGN-4: 文档与日志只能记录 signer scope、policy、公钥与审计摘要，不得记录私钥、seed 或助记词。
  - NFR-P2P-GOVSIGN-5: validator / finality signer 准入必须通过 governance registry 的候选/激活流程生效，不能把本地 env 或 `--node-validator*` 参数修改当作长期 admission 机制。
- Security & Privacy: 本专题只定义治理 signer 的长期真值和治理流程边界；禁止把任何真实 seed、私钥或生产签名材料落入仓库或证据文档。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 完成 governance signer inventory、source-of-truth 目标与 operator ownership 冻结。
  - v1.1: 定义 failover/rotation/revocation gate。
  - v1.2: readiness project 标记 `MAINNET-2` 完成，并把下一步交给 `MAINNET-3`。
- Technical Risks:
  - 风险-1: 如果 local seed/config 没有先退出 production path，后续 ceremony 会基于错误真值。
  - 风险-2: 如果 failover 没有明确阈值与恢复规则，治理 signer 故障时可能直接停摆。
  - 风险-3: 如果 operator ownership 不清，治理 signer 变更无法审计。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-GOVSIGN-001 | GOVSIGN-0/1 | `test_tier_required` | 代码真值盘点、governance signer inventory、模块主追踪回写 | 治理 signer 来源与真值边界 |
| PRD-P2P-GOVSIGN-002 | GOVSIGN-1/2 | `test_tier_required` | externalized source-of-truth、update authority 与禁止项冻结 | finality/controller signer 长期治理目标 |
| PRD-P2P-GOVSIGN-003 | GOVSIGN-2/3 | `test_tier_required` | failover/rotation/revocation gate 定义与 QA 审核清单 | 治理 signer 失效恢复能力 |
| PRD-P2P-GOVSIGN-004 | GOVSIGN-3/4 | `test_tier_required` | operator ownership、release/public-claims 依赖链核对 | 治理运营和变更审计 |
| PRD-P2P-GOVSIGN-005 | GOVSIGN-5 | `test_tier_required` | validator/finality signer admission 状态机、材料清单、activation 规则与角色边界冻结 | 新 validator 准入路径、world-state registry 激活边界与 controller appointment 分层 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-P2P-GOVSIGN-001 | 将 finality signer 与 controller signer 一起纳入治理外部化专题 | 只处理 finality signer，把 controller signer 留在 token topic | 两者共同构成治理真值，拆开会继续造成 source-of-truth 分裂。 |
| DEC-P2P-GOVSIGN-002 | deterministic local seed 与 `NodeConfig` policy 都只能算 preview/local truth | 接受其中一条作为生产过渡方案 | 任何依赖 local seed/config 的路径都缺少正式治理更新与审计能力。 |
| DEC-P2P-GOVSIGN-003 | 治理长期真值选定为 `on-chain/world-state registry` | 先走链下 external registry 过渡 | 当前制作人决策已经明确要求治理真值直接上链，因此后续实现必须围绕链上 source-of-truth 展开。 |
| DEC-P2P-GOVSIGN-004 | validator / finality signer 准入采用“申请 -> 审核 -> candidate/probation -> activation -> revoke/rotate”的治理状态机 | 继续依赖人工改 env、改 `NodePosConfig` 或直接把节点拉进活跃集合 | 这更接近主流公链“协议内注册 + 激活边界 + 审计留痕”的通用模式，也与当前 world-state registry 真值方向一致。 |
| DEC-P2P-GOVSIGN-005 | controller signer 保持治理内部 appointment，不和外部 validator 准入混成一条流程 | 把 treasury/controller slot 也当成公开 validator 申请的一部分 | controller signer 权限更高，职责也不同；若混合流程，会把治理/金库风险错误地下放到节点准入路径。 |
