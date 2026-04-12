# oasis7 主链 Token 分配与发行机制（已实现）

- 对应设计文档: `doc/p2p/token/mainchain-token-allocation-mechanism.design.md`
- 对应项目管理文档: `doc/p2p/token/mainchain-token-allocation-mechanism.project.md`

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 建立主链 Token 经济闭环：创世分配、解锁领取、epoch 增发、费用销毁、治理参数更新。
- Proposed Solution: 将 `NodePoints/PowerCredit` 与主链 Token 账本解耦，通过可审计桥接事件接入 `node_service_reward` 分配。
- Success Criteria:
  - SC-1: 保证快照回放一致性与参数治理可追溯性。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 主链 Token 分配与发行机制（已实现） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: 主链 Token 配置、供应、账户、创世桶、epoch 发行记录、金库余额、治理延迟队列。
  - AC-2: 动作闭环：
  - AC-3: `InitializeMainTokenGenesis`
  - AC-4: `ClaimMainTokenVesting`
  - AC-5: `ApplyMainTokenEpochIssuance`
  - AC-6: `SettleMainTokenFee`
- Non-Goals:
  - 跨链桥、DEX/CEX 上线、自动做市与兑换市场。
  - 法币入口、KYC/合规主体配置。
  - 复杂税收与链下清结算系统。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
  - `doc/p2p/token/mainchain-token-allocation-mechanism.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) 主配置与边界
```rust
MainTokenConfig {
  symbol: String,                // 默认 "OC"
  decimals: u8,                  // 默认 9
  initial_supply: u64,
  max_supply: Option<u64>,
  inflation_policy: MainTokenInflationPolicy,
  issuance_split: MainTokenIssuanceSplitPolicy,
  burn_policy: MainTokenBurnPolicy,
}
```
- 边界校验入口：`validate_main_token_config_bounds(&MainTokenConfig) -> Result<(), String>`。
- 关键约束：
  - `issuance_split` 总和必须 `10000 bps`；
  - `burn_policy` 各项 `<= 10000 bps`；
  - `min_rate_bps <= base_rate_bps <= max_rate_bps`；
  - `epochs_per_year > 0`；
  - `max_supply >= initial_supply`（若设置）。
  - 当前链上代币的正式产品命名固定为“绿洲币 / Oasis Coin”；`symbol` 字段当前承接 runtime symbol/ticker 语义，现行默认值为 `OC`；公钥派生账户前缀同步为 `oc:pk:`。

### 2) 动作与事件
```rust
Action::InitializeMainTokenGenesis { allocations }
Action::ClaimMainTokenVesting { bucket_id, beneficiary, nonce }
Action::ApplyMainTokenEpochIssuance { epoch_index, actual_stake_ratio_bps }
Action::SettleMainTokenFee { fee_kind, amount }
Action::UpdateMainTokenPolicy { proposal_id, next }
Action::ApplyNodePointsSettlementSigned { report, signer_node_id, mint_records }
```

```rust
DomainEvent::MainTokenGenesisInitialized { total_supply, allocations }
DomainEvent::MainTokenVestingClaimed { bucket_id, beneficiary, amount, nonce }
DomainEvent::MainTokenEpochIssued { epoch_index, inflation_rate_bps, issued_amount, ... }
DomainEvent::MainTokenFeeSettled { fee_kind, amount, burn_amount, treasury_amount }
DomainEvent::MainTokenPolicyUpdateScheduled { proposal_id, effective_epoch, next }
DomainEvent::NodePointsSettlementApplied {
  ...,
  main_token_bridge_total_amount,
  main_token_bridge_distributions,
}
```

### 3) 状态与查询
- `WorldState` 主链 Token 相关字段：
  - `main_token_config`
  - `main_token_supply`
  - `main_token_balances`
  - `main_token_genesis_buckets`
  - `main_token_epoch_issuance_records`
  - `main_token_treasury_balances`
  - `main_token_claim_nonces`
  - `main_token_scheduled_policy_updates`
  - `main_token_node_points_bridge_records`
- `World` 查询入口（节选）：
  - `main_token_config()`
  - `main_token_supply()`
  - `main_token_account_balance(account_id)`
  - `main_token_treasury_balance(bucket_id)`
  - `main_token_epoch_issuance_record(epoch_index)`
  - `main_token_scheduled_policy_update(effective_epoch)`
  - `main_token_node_points_bridge_record(epoch_index)`

### 4) 已落地关键语义
- 创世分配：
  - 分桶比例和必须为 `10000`；
  - `initial_supply` 在创世时一次性写入 `total_supply`；
  - 初始进入 `vested_balance`，需通过 `ClaimMainTokenVesting` 释放到 `liquid_balance`。
- epoch 增发：
  - `effective_rate = clamp(base + gain * (target - actual) / 10000, min, max)`；
  - `issued = floor(circulating_supply * rate / epochs_per_year / 10000)`；
  - 分配到 `staking/node_service/ecosystem/security`（余数归并到 `security_reserve`）。
- 费用结算：
  - 按 `fee_kind` 的 burn bps 计算 `burn_amount`；
  - 剩余进入对应 treasury bucket；
  - 同步更新 `total_supply/circulating_supply/total_burned`。
- 治理更新：
  - `UpdateMainTokenPolicy` 走参数边界校验；
  - 固定延迟 `2` 个 epoch 生效（`MAIN_TOKEN_POLICY_UPDATE_DELAY_EPOCHS = 2`）；
  - 增发与费用结算均按“目标 epoch 的有效配置”计算。
- NodePoints 桥接占位：
  - 预算来源：同 epoch `MainTokenEpochIssuanceRecord.node_service_reward_amount`；
  - 分配：按 `awarded_points` 比例做确定性分配，余数按排序回填；
  - 执行：扣减 `node_service_reward_pool`，增加节点主链 Token `liquid_balance` 与 `circulating_supply`；
  - 审计：写入 `main_token_node_points_bridge_records[epoch]`；
  - 当前账户映射占位策略：`account_id = node_id`。

### 5) 运行手册补充
- 主链 Token / NodePoints 桥接定向回归：
```bash
./scripts/main-token-regression.sh required
./scripts/main-token-regression.sh full
```
- 核心审计检查点：
  - 增发记录：`main_token_epoch_issuance_record(epoch)`；
  - 治理延迟：`main_token_scheduled_policy_update(effective_epoch)`；
  - 桥接记录：`main_token_node_points_bridge_record(epoch)`；
  - 供应守恒：`main_token_supply().total_supply = initial + issued - burned`。

## 5. Risks & Roadmap
- Phased Rollout:
  - **TAM-M0**：设计文档/项目文档建档（完成）。
  - **TAM-M1**：状态模型与快照字段落地（完成）。
  - **TAM-M2**：创世分配 + vesting 领取落地（完成）。
  - **TAM-M3**：epoch 增发与分配落地（完成）。
  - **TAM-M4**：费用销毁与金库记账落地（完成）。
  - **TAM-M5**：治理边界 + 生效延迟 + 审计事件落地（完成）。
  - **TAM-M6**：NodePoints 桥接占位接线与结算路径落地（完成）。
  - **TAM-M7**：`test_tier_required/full` 回归矩阵与脚本落地（完成）。
  - **TAM-M8**：文档回写、发布说明与运行手册补充（完成）。
- Technical Risks:
  - 参数风险：通胀/分配配置不当可能导致激励不足或稀释过快。
  - 治理风险：提案频繁变更会增加运行理解成本，需保持提案审计与发布节奏。
  - 桥接风险：当前 `node_id -> account_id` 为占位映射，后续身份体系升级时需平滑迁移。
  - 并行经济体风险：`NodePoints/PowerCredit` 与主链 Token 并行阶段仍需持续防重复激励。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-112-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-112-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
