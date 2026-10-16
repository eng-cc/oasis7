# oasis7 主链 Token 分配与发行机制（已实现）

- 对应设计文档: `doc/p2p/token/mainchain-token-allocation-mechanism.design.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 建立主链 Token 经济闭环：创世分配、解锁领取、epoch 增发、费用销毁、治理参数更新。
- Proposed Solution: 将 `NodePoints/PowerCredit` 与主链 Token 账本解耦，通过可审计桥接事件接入 `node_service_reward` 分配。
- Success Criteria:
  - SC-1: 保证快照回放一致性与参数治理可追溯性。

## 2. User Experience & Functionality
- User Personas: 主链 Token 账本/治理维护者与发行审计复核者。
- User Scenarios & Frequency: 创世分配、vesting 领取、epoch 发行、费用结算、治理更新或 NodePoints 桥接时。
- User Stories: 作为账本维护者，需要沿既有动作和治理条件确定性记账并能复算供应守恒，不把文档迁移当作经济机制验证。
- Critical User Flows: 验证 MainTokenConfig -> 创世写入分桶/vested balance -> 按领取动作释放 liquid balance；epoch 增发按目标 epoch 有效配置分配，费用结算按原 burn/treasury 规则；治理提案批准后按既有两 epoch 延迟生效，NodePoints 桥接使用同 epoch node_service_reward 预算并记录审计。
- Functional Specification Matrix（仅承接下文既有专业合同；不新增实现或验收结论）:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 创世/领取 | config、genesis buckets、claim nonce | 创世分配后通过 ClaimMainTokenVesting 领取 | vested -> liquid | 分桶和 10000 bps；沿原边界 | 既有动作合同，不新增公开领取协议 |
| 发行/费用 | epoch records、supply、treasury | 按目标 epoch 有效配置增发/销毁记账 | 供应和 bucket 同步更新 | 原公式、分桶与余数规则 | 不改变参数或费率 |
| 治理/分发/桥接 | proposal_id、distribution_id、node account binding | 批准条件、两 epoch 延迟、幂等分发与桥接 | 审计记录和余额一致 | 桥接用同 epoch node_service_reward | 只限原三个 treasury bucket；内部绑定非 custody |
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
- Architecture Overview: 按下文配置边界、动作事件、WorldState 查询、既有发行/治理/桥接语义组织合同；NodePoints/PowerCredit 与主链 Token 账本保持解耦。
- Integration Points:
  - `doc/p2p/token/mainchain-token-allocation-mechanism.prd.md`
  - GitHub Issue / GitHub Project
  - `testing-manual.md`
- Edge Cases & Error Handling: 非法 split/burn/rate/epoch/supply 配置须按原校验拒绝；治理提案须 Approved/Applied 并遵守两 epoch 延迟；distribution_id 幂等。旧快照默认值只提供既有兼容，不证明外部 custody。
- Non-Functional Requirements: 保留供应守恒、确定性分配、审计幂等和快照回放一致；不改发行比例、账户绑定权限或奖励金额。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
<a id="tam-config"></a>
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
  - <a id="tam-policy-delay"></a>固定延迟 `2` 个 epoch 生效（`MAIN_TOKEN_POLICY_UPDATE_DELAY_EPOCHS = 2`）；
  - 增发与费用结算均按“目标 epoch 的有效配置”计算。
- NodePoints 桥接与二期治理分发：
  - 预算来源：同 epoch `MainTokenEpochIssuanceRecord.node_service_reward_amount`；
  - 分配：按 `awarded_points` 比例做确定性分配，余数按排序回填；
  - 执行：扣减 `node_service_reward_pool`，增加节点主链 Token `liquid_balance` 与 `circulating_supply`；
  - 审计：写入 `main_token_node_points_bridge_records[epoch]`；
  - <a id="tam-internal-binding"></a>节点主链账户以 `WorldState.node_main_token_account_bindings: BTreeMap<String, String>` 维护；`World::bind_node_main_token_account(node_id, account_id)` 写入并由 `World::node_main_token_account(node_id)` 查询。NodePoints 桥接优先使用该绑定，而不是把 `node_id` 当作账户。
  - <a id="tam-policy-approval"></a>`UpdateMainTokenPolicy { proposal_id, next }` 除原有参数边界和两 epoch 延迟外，要求 proposal 存在且状态为 `Approved` 或 `Applied`。
  - <a id="tam-distribution"></a>`DistributeMainTokenTreasury { proposal_id, distribution_id, bucket_id, distributions }` 仅可分发 `staking_reward_pool`、`ecosystem_pool`、`security_reserve`。它扣减 treasury bucket、增加接收方 `liquid_balance` 与 `circulating_supply`（不改变 `total_supply`），并写入 `main_token_treasury_distribution_records[distribution_id]` 与 `MainTokenTreasuryDistributed` 审计事件；`distribution_id` 必须幂等。
  - <a id="tam-snapshot-defaults"></a>旧快照缺少二期字段时以 `serde(default)` 保持兼容。

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
- 二期地址绑定仅为 runtime 内部字符串映射，不是外部钱包、跨链地址、custody 或公开领取协议；二期不改变通胀公式或 split 比例。
  - 并行经济体风险：`NodePoints/PowerCredit` 与主链 Token 并行阶段仍需持续防重复激励。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| 原 AC 与 §4 专业合同 | 对应业务变更的 PR / 实际验证记录 | `test_tier_required` + `test_tier_full` | AC-1..6 与 §4 既有动作/治理/桥接合同：按下文 main-token-regression.sh required/full 入口核配置边界、创世/领取、增发/费用、批准与延迟、分发幂等、同 epoch 桥接及供应守恒；命令是验证入口，本次未执行。 | 既有专业行为及负向边界；不代签当前运行通过 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-112-001 | 历史文档迁移：逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。

专业设计追踪导航：[配置边界](mainchain-token-allocation-mechanism.design.md#tam-config-design)、[延迟](mainchain-token-allocation-mechanism.design.md#tam-delay-design)、[批准条件](mainchain-token-allocation-mechanism.design.md#tam-approval-design)、[三 bucket 分发](mainchain-token-allocation-mechanism.design.md#tam-buckets-design)、[审计幂等](mainchain-token-allocation-mechanism.design.md#tam-idempotency-design)、[内部绑定](mainchain-token-allocation-mechanism.design.md#tam-binding-design)、[快照默认](mainchain-token-allocation-mechanism.design.md#tam-defaults-design)。仅定位原文技术约束，不更新数值、状态或验收。

## 历史 strictification 迁移记录（不属于业务流程或验收）

以下保留迁移时的原流程、矩阵、结构说明和验证来源，仅供 Git history / 原 GitHub task evidence 追溯；`draft -> active -> done` 指文档迁移，不是运行状态。正文流程与 §6 现在承接本文原 AC/技术合同，未改变经济参数、权限、实现状态或重新激活前置。

- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 主链 Token 分配与发行机制（已实现） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。

原迁移验证行（仅文档结构，不代签业务验收）：
| PRD-P2P-MIG-112-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |

原迁移决定：
| DEC-PRD-P2P-MIG-112-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |
