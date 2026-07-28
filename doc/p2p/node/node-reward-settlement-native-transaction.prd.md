# oasis7 Runtime：奖励结算切换到网络共识主路径原生交易

- 对应设计文档: `doc/p2p/node/node-reward-settlement-native-transaction.design.md`
- 对应项目管理文档: `doc/p2p/node/node-reward-settlement-native-transaction.project.md`

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 将当前 reward runtime 的“旁路直写结算（`apply_node_points_settlement_mint_v2` 直接调用）”切换为世界主执行路径可消费的原生 `Action` 交易。
- Proposed Solution: 让奖励结算和兑换一样进入 `submit_action -> step -> event -> state apply` 闭环，统一审计与回放语义。
- Success Criteria:
  - SC-1: 保持现有签名治理（`mintsig:v2`、`RewardSignatureGovernancePolicy`）与账本字段兼容，不破坏历史快照读取。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：奖励结算切换到网络共识主路径原生交易 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: **NSTX-1：新增奖励结算原生交易动作**
  - AC-2: 新增 `Action::ApplyNodePointsSettlementSigned`，将 epoch 结算报告与签名后的 mint 记录作为交易负载。
  - AC-3: 该动作走既有 action 主路径，不再通过 runtime 旁路直接改状态。
  - AC-4: **NSTX-2：新增领域事件与状态应用**
  - AC-5: 新增 `DomainEvent::NodePointsSettlementApplied`，作为奖励结算主路径事件。
- AC-6: 在 `WorldState::apply_domain_event` 中实现 mint 账本写入、系统订单池预算扣减、幂等与守恒校验。
- Non-Goals:
  - 跨节点奖励调度器与链上提案器（多节点共识下自动发起奖励结算交易）。
- 将 NodePoints 采样器直接并入 `oasis7_node` 主循环并跨节点同步。
  - 奖励市场化定价、可转账代币化和完整清算系统。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/node/node-reward-settlement-native-transaction.prd.md`
  - `doc/p2p/node/node-reward-settlement-native-transaction.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 新增 Action（草案）
```rust
Action::ApplyNodePointsSettlementSigned {
  report: EpochSettlementReport,
  signer_node_id: String,
  mint_records: Vec<NodeRewardMintRecord>,
}
```

### 新增 DomainEvent（草案）
```rust
DomainEvent::NodePointsSettlementApplied {
  epoch_index: u64,
  signer_node_id: String,
  settlement_hash: String,
  minted_records: Vec<NodeRewardMintRecord>,
}
```

### 状态应用语义（草案）
- 对 `minted_records` 逐条应用：
  - `node_asset_balances[node].power_credit_balance += minted_power_credits`
  - `node_asset_balances[node].total_minted_credits += minted_power_credits`
  - `reward_mint_records.push(record)`
- 预算池（若存在）按记录扣减：
  - `remaining_credit_budget -= minted_power_credits`
  - `node_credit_allocated[node] += minted_power_credits`
- 幂等与一致性：
  - 同 `epoch_index + node_id` 已存在记录时拒绝重复应用。

### 当前 collector 接线边界
- `oasis7_chain_runtime` 的 collector 只在满足本地 readiness 条件后提交 `Action::ApplyNodePointsSettlementSigned { report, signer_node_id, mint_records }`，再通过 `submit_action -> step -> DomainEvent::NodePointsSettlementApplied -> WorldState::apply_domain_event` 落账。
- 该路径的签名、守恒、预算与重复记录校验都在 action/event 主链路拒绝；collector 不得直接改写奖励资产状态。
- 这不是多节点自动调度器、共识提案器或 `aw.*` topic/envelope 协议。历史观察轨迹、sequencer 广播和 UDP 表述不构成当前接口或传输权威。

## 5. Risks & Roadmap
- Phased Rollout:
  - **NSTX-M0**：设计文档 + 项目管理文档。
  - **NSTX-M1**：Action/DomainEvent/状态应用落地。
  - **NSTX-M2**：reward runtime 接线切换到原生交易。
  - **NSTX-M3**：`test_tier_required` 回归、文档状态回写、devlog 收口。
- Technical Risks:
  - 结算交易负载包含 `EpochSettlementReport` 与 `mint_records`，单笔 payload 体积可能较大；后续需考虑压缩或分片。
  - 若预算校验与状态扣减语义不一致，可能出现“校验通过但应用失败”；需以单测锁定规则。
  - 从旁路改主路径后，任何字段不兼容都会反映为 action reject，需要明确错误日志与运维排障口径。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-101-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-101-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
