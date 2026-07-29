# oasis7 Runtime：节点贡献积分激励

- 对应设计文档: `doc/p2p/node/node-contribution-points.design.md`
- 对应项目管理文档: `doc/p2p/node/node-contribution-points.project.md`

审计轮次: 5
## 专业权威口径
- 本文件是节点贡献积分产品与专业规则的当前主入口。
- 既有 runtime closure、multi-node closure test、存储系统奖励池与基础在线时长奖励增量文档的有效语义已合并到本三件套；源文件删除后，历史过程由 Git history 与对应 GitHub task evidence 追溯。
- 本文件定义专业积分与结算合同，不承诺玩家可用性、当前数值平衡、公开网络经济安全或 release readiness。

## 1. Executive Summary
- Problem Statement: 在 oasis7 的区块链 + P2P FS 闭环内，引入可审计的节点积分激励（Node Points）。
- Proposed Solution: 明确“基础义务”和“额外贡献”的边界：
- Success Criteria:
  - SC-1: 为自身 Agent 提供模拟计算属于基础义务，不直接奖励；
  - SC-2: 为离线节点代跑模拟、执行世界维护任务属于额外计算，应获得奖励。
  - SC-3: 为长期在线且提供更多有效存储的节点提供额外收益。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：节点贡献积分激励 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: 新增节点积分结算引擎（epoch 级）。
  - AC-2: 贡献维度：
  - AC-3: `delegated_sim_compute_units`（代跑离线节点）；
  - AC-4: `world_maintenance_compute_units`（世界维护任务）；
  - AC-5: `effective_storage_bytes`（有效存储）；
  - AC-6: `uptime_seconds`（在线时长）；
  - AC-7: runtime 必须把贡献快照确定性地结算到对应 epoch，重复执行或恢复重放不得重复入账，普通快照更新不得静默改变奖励台账。
  - AC-8: 多节点闭环至少覆盖 3 个节点、连续 2 个 epoch，并同时验证计算、存储、在线、可靠性与惩罚输入。
  - AC-9: 固定 epoch 积分池不得超发；贡献排序、惩罚效果与累计积分单调性必须有可重复断言。
- Non-Goals:
  - 链上可交易代币、真实经济清算。
  - 完整质押/罚没资产系统（仅保留积分惩罚入口）。
  - 复杂证明协议（PoRep/PoSt/ZK）的真实网络接线。
  - 用本地多节点夹具冒充真实公网证明、生产 readiness 或经济安全结论。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/node/node-contribution-points.prd.md`
  - `doc/p2p/node/node-contribution-points.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 核心配置（草案）
```rust
NodePointsConfig {
  epoch_duration_seconds: u64,
  epoch_pool_points: u64,
  min_self_sim_compute_units: u64,
  storage_pool_points: u64,
  min_uptime_challenge_pass_ratio: f64,
  min_storage_challenge_pass_ratio: f64,
  min_storage_challenge_checks: u64,
  max_rewardable_storage_to_staked_ratio: f64,
  delegated_compute_multiplier: f64,
  maintenance_compute_multiplier: f64,
  weight_compute: f64,
  weight_storage: f64,
  weight_uptime: f64,
  weight_reliability: f64,
  obligation_penalty_points: f64,
}
```

### 节点贡献输入（草案）
```rust
NodeContributionSample {
  node_id: String,
  self_sim_compute_units: u64,
  delegated_sim_compute_units: u64,
  world_maintenance_compute_units: u64,
  effective_storage_bytes: u64,
  uptime_seconds: u64,
  uptime_valid_checks: u64,
  uptime_total_checks: u64,
  storage_valid_checks: u64,
  storage_total_checks: u64,
  staked_storage_bytes: u64,
  verify_pass_ratio: f64,
  availability_ratio: f64,
  explicit_penalty_points: f64,
}
```

### 结算输出（草案）
```rust
NodeSettlement {
  node_id: String,
  obligation_met: bool,
  compute_score: f64,
  storage_score: f64,
  uptime_score: f64,
  reliability_score: f64,
  storage_reward_score: f64,
  rewardable_storage_bytes: u64,
  penalty_score: f64,
  total_score: f64,
  main_awarded_points: u64,
  storage_awarded_points: u64,
  awarded_points: u64,
  cumulative_points: u64,
}

EpochSettlementReport {
  epoch_index: u64,
  pool_points: u64,
  storage_pool_points: u64,
  distributed_points: u64,
  storage_distributed_points: u64,
  total_distributed_points: u64,
  settlements: Vec<NodeSettlement>,
}
```

### 计分公式（MVP）
- 额外计算分：
  - `compute_units = delegated * delegated_multiplier + maintenance * maintenance_multiplier`
  - `compute_score = compute_units * verify_pass_ratio`
- 存储分：
  - `storage_gib = effective_storage_bytes / 1024^3`
  - `storage_score = sqrt(storage_gib) * availability_ratio`
- 独立存储奖励池：
  - `epoch_pool_points` 与 `storage_pool_points` 是两个独立的固定 epoch 积分预算；各自只向正的合格得分分配，允许出现未分配余额，不承诺自动结转。
  - 当 `storage_pool_points > 0` 时，主池归一化权重会把 `weight_storage` 置零，避免同一存储贡献同时从主池和存储池重复获奖。
  - 只有 `storage_total_checks >= min_storage_challenge_checks` 且通过率严格高于 `min_storage_challenge_pass_ratio` 时，存储奖励得分才为正；挑战质量按阈值以上区间归一化。
  - `max_rewardable_storage_to_staked_ratio > 0` 时，可奖励存储不超过 `staked_storage_bytes * ratio`；启用封顶但质押为零时可奖励量为零。零值或非有限 ratio 表示不启用该封顶。
  - `storage_reward_score = sqrt(rewardable_storage_gib) * normalized_challenge_quality * availability_ratio`。
- 在线分：
  - epoch 内存在挑战记录时，以 `uptime_valid_checks / uptime_total_checks` 为原始在线率；没有挑战记录时才回退到 `uptime_seconds / epoch_duration_seconds`。
  - 原始在线率在 `min_uptime_challenge_pass_ratio` 及以下得分为零，超过阈值后在剩余区间线性归一化到 `[0, 1]`。
- 可靠性分：
  - `reliability_score = (verify_pass_ratio + availability_ratio) / 2`
- 总分：
  - `total = w_c*compute + w_s*storage + w_u*uptime + w_r*reliability - penalty`
  - `total < 0` 则按 `0` 处理。
- 基础义务惩罚：
  - 当 `self_sim_compute_units < min_self_sim_compute_units` 时，额外加罚 `obligation_penalty_points`。
- 结算合并：
  - `awarded_points = main_awarded_points + storage_awarded_points`；累计积分只在 epoch 结算中更新，采样或快照写入不得直接增发。

### 积分与资产结算边界
- NodePoints 可进入既有 PowerCredit 路径，但仍须通过身份、签名、预算、reserve、nonce 与 replay gate；积分不是自动流动价值，也不证明 custody、市场或兑付 readiness。
- 主链 Token 的 NodePoints bridge 是另一条独立路径：它从同 epoch 的 `node_service_reward` 发行预算按 `awarded_points` 确定性分配，不是存储池或主池本身，也不是积分自动兑换。
- 同时启用多条激励路径存在重复奖励风险。调整池权重、阈值、兑换比例或发行分桶必须另行经过经济治理和 runtime 复核。

### Runtime 与多节点闭环约束
- runtime 以 epoch 边界消费节点贡献快照并产出 `EpochSettlementReport`；同一 epoch 的 settlement 必须具备幂等键或等价去重语义。
- 恢复、重放或重复 tick 只能复现同一结算结果，不得增加 `awarded_points` 或 `cumulative_points`。
- 快照采集与奖励结算分层：更新 compute/storage/uptime/reliability 样本本身不得提前写入积分台账。
- `NodePointsRuntimeCollectorSnapshot` 是采样器实现状态，不是结算授权：它持久化 ledger、heuristics、epoch 起点、每节点 cursor 与当前 epoch accumulator。`oasis7_chain_runtime` 在采样前恢复它，以保持 epoch 幂等；重复 tick、重启或 replay 不得因恢复而新增 `awarded_points` 或 `cumulative_points`。
- collector 状态由 `reward-runtime-state.json` 原子写入。文件不可读时 runtime 会显式发出 warning/metric 并以新 collector 启动；这不是资产对账、已结算状态恢复或生产 custody 成功的证明。
- 多节点验证最小拓扑为 3 个具有可区分贡献画像的节点，跨越至少 2 个 epoch；验证固定池守恒、贡献更高者排序不反转、显式或义务惩罚会降低得分，以及每个节点累计积分不回退。
- 该夹具证明确定性业务闭环，不证明采样真实性、复杂证明协议、真实传输或生产网络 readiness。

## 5. Risks & Roadmap
- Phased Rollout:
  - NCP-1：设计文档 + 项目管理文档。
  - NCP-2：节点积分引擎核心实现（计算/存储/在线/惩罚 + 台账）。
  - NCP-3：测试与导出接线（test_tier_required 口径）。
  - NCP-4：文档状态回写与 devlog 收口。
- Technical Risks:
  - 参数不当可能导致单一资源（大存储或大算力）垄断积分，需要通过 `sqrt(storage)` 与权重平衡缓解。
  - 若没有真实证明接线，`verify_pass_ratio/availability_ratio` 的真实性依赖上层采样器，后续需替换为链路证明数据。
  - 积分池固定时，低活跃 epoch 可能出现“有效贡献过少”，需在后续迭代加入最小活跃阈值与回收池机制。
  - collector 状态文件损坏会丢失未结算采样上下文；它必须以可观测降级处理，不能被表述为已结算资产的安全恢复。
  - uptime/storage challenge 与 collector 字段只是启发式采样输入，不是 PoRep/PoSt、VRF、多观察点证明、抵押罚没、公开网络真实性或生产经济安全证明。
  - 挑战频率过低会放大在线率或存储资格波动；存储封顶、挑战阈值和多激励路径配置不当会压制真实贡献或造成重复激励。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-091-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-091-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
