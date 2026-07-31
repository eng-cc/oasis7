# oasis7 Runtime：共识代码统一收敛到 oasis7_consensus

- 对应设计文档: `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.design.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 将 `oasis7_node` 中 PoS 共识核心状态机（proposal/attestation/decision）迁移到 `oasis7_consensus`，避免同语义双实现长期漂移。
- Proposed Solution: 保持 `oasis7_node` 专注于节点运行时职责（网络收发、复制、执行 hook、快照桥接），共识规则核心改为复用 `oasis7_consensus`。
- Success Criteria:
  - SC-1: 在不破坏现有 runtime 行为的前提下，分阶段完成“代码位置统一 + 运行语义不回退”。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：共识代码统一收敛到 oasis7_consensus 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: 在 `crates/oasis7_consensus` 新增 node 可复用的 PoS 核心模块：
  - AC-2: attestation 结构
  - AC-3: pending proposal 结构
  - AC-4: decision 结构
  - AC-5: proposal 构造、attestation 插入、状态推进函数
  - AC-6: `crates/oasis7_node` 改为调用上述核心模块，不再本地维护同构结构与核心推进算法。
- Non-Goals:
  - 一次性迁移 `oasis7_node` 全部共识相关网络处理代码（gossip/libp2p endpoint 适配仍留在 node）。
  - 改写 `oasis7_consensus::PosConsensus` 到 `oasis7_node` 运行主路径（本轮先做核心抽取，不做大规模替换）。
  - Fork-choice/finality/BLS 等完整以太坊信标链语义升级。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/consensus/consensus-code-consolidation-to-oasis7-consensus.prd.md`
  - GitHub Issue / GitHub Project
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) oasis7_consensus 新增 node_pos 核心模块
- `NodePosAttestation`
- `NodePosPendingProposal<TAction, TStatus>`
- `NodePosDecision<TAction, TStatus>`
- `NodePosStatusAdapter`（用于 node 自定义状态枚举映射）
- 核心函数：
  - `propose_next_head(...)`
  - `advance_pending_attestations(...)`
  - `insert_attestation(...)`
  - `decision_from_proposal(...)`

### 2) oasis7_node 适配
- `PosNodeEngine` 保留运行时状态与网络接线。
- 本地同构结构替换为 `oasis7_consensus::node_pos` 类型别名。
- 错误映射统一为 `NodeError::Consensus { reason }`。

### 3) 依赖分层修正（避免循环依赖）
- 为支持 `oasis7_node -> oasis7_consensus` 单向依赖，`oasis7_consensus` 内聚 `distributed_dht` / `distributed_net` 抽象与内存实现，不再反向依赖 `oasis7_net`。
- 该调整不改变 PoS/成员治理语义，仅收敛 crate 边界，确保后续共识代码可持续集中在 `oasis7_consensus`。

### 4) 第二阶段（尽量一步到位）收口目标
- 将 `oasis7_node` 中残留的共识纯逻辑（action root 计算/校验、共识消息签名验签、共识消息结构定义）迁移到 `oasis7_consensus`，`oasis7_node` 仅保留运行时接线与错误映射。
- 对 `oasis7_consensus` 内部 PoS 双实现关系进行收敛：`node_pos` 作为 node 主链路推进核心，`pos` 复用同一推进核心，避免两套独立推进逻辑长期漂移。
- 保持 `oasis7_node` 外部接口和现有闭环测试口径不回退。

## 5. Risks & Roadmap
- Phased Rollout:
  - CCG-0：设计与项目文档建档。
  - CCG-1：抽取 PoS 核心状态机到 `oasis7_consensus::node_pos` 并接线 `oasis7_node`。
  - CCG-2：回归测试（node + viewer live 定向）与文档/devlog 收口。
  - CCG-3：扩展文档，定义第二阶段“共识代码全收口 + PoS 单链路化”任务。
  - CCG-4：迁移 `oasis7_node` 残留共识纯逻辑到 `oasis7_consensus` 并完成接线。
  - CCG-5：完成 PoS 内部单链路收敛、定向回归与文档/devlog 终态收口。
- Technical Risks:
  - 泛型化抽取若边界定义不清，可能导致类型复杂度上升，影响可读性。
  - 抽取过程中若状态更新顺序变化，可能引发边界行为回归（如 pending -> committed 时机）。
  - 后续若不继续推进网络层抽取，仍会存在“规则已统一、接线分散”的中间状态，需要后续阶段继续收口。
  - 第二阶段若迁移边界过大，可能导致 `oasis7_node` 与 `oasis7_consensus` 接口短期震荡，需通过分层回归测试兜底。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-058-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-058-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。

## 7. 有序动作载荷与 committed replay 合同

- proposal/commit/replication 携带版本化、有序动作 envelope；action root 对完整有序列表做确定性承诺，签名、复制和执行上下文必须绑定同一 root 与 payload。
- 接收方在提交或回放前验证 payload hash、action root、顺序与 envelope 版本；未知的非 runtime payload 可按版本化边界跳过或拒绝，但不得使已知 committed runtime action 静默丢失。
- viewer/live/simulator 只消费已提交动作并按提交顺序回放；submit acceptance、pending proposal 或空 committed batch 都不等于世界推进。
- 共识层拥有动作完整性与提交顺序，不拥有市场价格、WASM 生命周期、LLM 决策或 runtime state apply 规则。
