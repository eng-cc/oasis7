# oasis7 Runtime：生产级区块链 + P2P FS 路线图

- 对应设计文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md`

审计轮次: 5

> 现行状态：本文是历史 P2PFS 路线图与追溯入口，保留用于理解链式哈希、结算签名和执行链路的早期收口。当前链上大世界状态底座的聚合闭环、测试层级与 claim boundary 以 `testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. Executive Summary
- Problem Statement: 将当前“可演示的区块链 + P2P FS + 节点收益”实现推进到可生产部署形态。
- Proposed Solution: 在保持现有可运行能力的同时，优先收敛三类高风险缺口：
- Success Criteria:
  - SC-1: 共识哈希语义弱（`block_hash` 非交易承诺）；
  - SC-2: 奖励结算网络消息缺少独立传输签名；
  - SC-3: 节点主循环与执行主链路仍是桥接模式，缺共识内生执行。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：生产级区块链 + P2P FS 路线图 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: **PRG-A1：共识区块哈希链式化**
  - AC-2: 将 `oasis7_node` 的提案 `block_hash` 从字符串占位升级为链式哈希（包含 `parent_hash`）。
  - AC-3: 持久化引擎最新已提交哈希，确保重启后哈希链连续。
  - AC-4: **PRG-A2：奖励结算 Envelope 传输签名**
  - AC-5: 为 `RewardSettlementEnvelope` 增加签名字段与验签流程。
  - AC-6: 在消费侧增加“签名验真 + signer 身份绑定一致性”前置校验。
- Non-Goals:
  - **PRG-B：共识内生执行（替代执行桥接）**
  - 将 `RuntimeWorld` 执行与状态根写入合并到 `NodeRuntime` 主循环。
  - **PRG-C：跨节点 DistFS 挑战与证明网络化**
  - 从本地 challenge/self-answer 迁移到跨节点 challenge-response。
  - **PRG-D：系统预算池主导收益模型（固定）**
  - 保持协议预算池主导，不引入需求侧订单撮合交易市场。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.prd.md`
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-roadmap.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) 节点区块哈希载荷（新增语义）
```rust
BlockHashPayload {
  version: u8,
  world_id: String,
  height: u64,
  slot: u64,
  epoch: u64,
  proposer_id: String,
  parent_block_hash: String,
}
```
- `block_hash = blake3(cbor(BlockHashPayload))`。
- `parent_block_hash`：已提交高度 `h-1` 的 `block_hash`，创世使用常量 `genesis`。

### 2) 奖励结算网络消息签名（新增字段）
```rust
RewardSettlementEnvelope {
  version: u8,
  world_id: String,
  epoch_index: u64,
  signer_node_id: String,
  signer_public_key_hex: String,
  report: EpochSettlementReport,
  mint_records: Vec<NodeRewardMintRecord>,
  emitted_at_unix_ms: i64,
  signature: String, // rewardsett:v1:<sig_hex>
}
```
- 签名算法：ed25519。
- 验签口径：
  1. envelope 签名有效；
  2. `signer_node_id -> public_key` 与世界状态绑定一致；
  3. 再进入既有 `ApplyNodePointsSettlementSigned` 状态校验。

## 5. Risks & Roadmap
- Phased Rollout:
  - **PRG-M0（本轮）**：路线图文档与任务拆解。
  - **PRG-M1（本轮）**：链式 `block_hash` + 持久化兼容。
  - **PRG-M2（本轮）**：结算 Envelope 签名/验签 + 消费前置校验。
  - **PRG-M3（本轮）**：`test_tier_required` 回归与文档/devlog 收口。
  - **PRG-M4（后续）**：共识内生执行。
  - **PRG-M5（后续）**：DistFS 证明网络化。
  - **PRG-M6（删除）**：需求侧交易化支付市场（已移除，维持系统预算池主导）。
- Technical Risks:
  - 链式哈希切换后，旧测试或历史快照若依赖占位字符串口径，可能出现兼容差异。
  - 结算 envelope 双层签名（envelope + mint record）增加排障复杂度，需要清晰日志。
  - 后续 PRG-M4/5 需要跨 crate 同步演进，若缺少统一验收标准会造成阶段回退。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-056-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-056-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
