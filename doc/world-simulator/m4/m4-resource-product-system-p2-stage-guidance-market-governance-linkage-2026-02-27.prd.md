# M4 资源与产品系统 P2：阶段引导与市场-治理联动观测（2026-02-27）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 在不改动作 ABI 的前提下，为工业链补齐“可解释的市场观测”和“阶段化进度”能力。
- 让配方排产在事件层可直接观察到材料侧成本信号，并显式体现治理税率的影响。
- 将材料行情从成本指数补成玩家可读的来源取舍：本地采购、外部调运、延后、治理调整或拆分来源。
- 提供轻量阶段引导状态（`bootstrap/scale_out/governance`），用于后续 UI 与策略模块消费。

## 2. User Experience & Functionality

### In Scope
- 扩展 `RecipeStarted` 事件：追加可选 `market_quotes`，用于输出材料侧行情快照。
- 当 `market_quotes` 存在本地缺口、运输损耗或治理税时，玩家侧应能读取 `market_quote_decision_preview`，解释成本主因、推荐来源和下一步降本动作。
- 在 `WorldState` 中新增工业阶段进度状态和观测缓存，并通过产业事件更新。
- 新增 `test_tier_required` 测试：
  - 治理税率变化影响 `market_quotes` 的有效成本指标。
  - 随着产业事件推进，阶段状态按预期从 `bootstrap` 进入 `scale_out/governance`。

### Out of Scope
- 不新增撮合交易动作，不引入复杂订单簿。
- 不改 viewer 交互（仅输出 runtime 可观测状态）。
- 不新增强制阶段闸门（仅观测与提示，不阻断动作执行）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 事件扩展
- 位置：`runtime/events.rs::DomainEvent::RecipeStarted`
- 新增字段：
  - `market_quotes: Vec<MaterialMarketQuote>`（`serde(default)`）

`MaterialMarketQuote` 建议字段：
- `kind: String`
- `requested_amount: i64`
- `local_available_amount: i64`
- `world_available_amount: i64`
- `local_deficit_amount: i64`
- `transit_loss_bps: i64`
- `governance_tax_bps: u16`
- `effective_cost_index_ppm: i64`

玩家侧 `market_quote_decision_preview` 合同：
- `material_kind`: 对应材料种类。
- `requested_amount`: 本次排产或查询需要的材料量。
- `recommended_source`: `local_stock / import_world / delay / governance_adjustment / split_source`。
- `local_vs_world_delta_ppm`: 本地采购与 world/import 来源的有效成本差。
- `tax_cost_ppm`: 治理税对有效成本的贡献。
- `transit_loss_cost_ppm`: 调运损耗对有效成本的贡献。
- `shortfall_after_recommended_source`: 按推荐来源处理后的剩余缺口。
- `cost_pressure_class`: `normal / elevated / expensive / blocked`。
- `why_this_source`: 为什么推荐该来源或等待/治理动作。
- `next_cost_reduction_action`: 降低下一次成本的最小动作，例如补本地库存、延后排产、调低治理税、拆分来源或先建本地供给。
- Edge case: 若 `market_quotes` 存在非零本地缺口、运输损耗或治理税，但玩家看不到推荐来源、成本主因或下一步降本动作，标记为 `market_quote_decision_missing`。
- Acceptance: 排产或查看行情时，玩家至少能看懂“为什么这次材料成本高、推荐从哪里补、税费/运输分别贡献多少、继续排产还是先调整供给/治理更合理”。

### 2) 阶段状态
- 位置：`runtime/state.rs::WorldState`
- 新增状态：`industry_progress: IndustryProgressState`（`serde(default)`）

`IndustryProgressState` 建议字段：
- `stage: IndustryStage`
- `stage_updated_at: WorldTime`
- `completed_recipe_jobs: u64`
- `completed_material_transits: u64`
- `latest_market_quotes: BTreeMap<String, MaterialMarketQuote>`

`IndustryStage`：
- `bootstrap`
- `scale_out`
- `governance`

### 3) 阶段推进规则（首版）
- `bootstrap -> scale_out`：
  - `completed_recipe_jobs >= 3` 且 `factories.len() >= 1`
- `scale_out -> governance`：
  - 已满足 `scale_out`，且 `(electricity_tax_bps > 0 || data_tax_bps > 0)`，且
  - `completed_recipe_jobs >= 6` 或 `completed_material_transits >= 3`
- 推进单向，不自动回退。

## 5. Risks & Roadmap
- P2-T0：设计文档 + 项目文档建档。
- P2-T1：事件与状态接线（market quote + industry progress）。
- P2-T2：补齐 required 单测并回归。
- P2-T3：文档与 devlog 收口。

### Technical Risks
- 事件负载增长：`RecipeStarted` 载荷变大，可能影响日志体积。
- 参数偏差：成本指数与阈值设定不当会导致阶段推进过快或过慢。
- 可读性风险：`effective_cost_index_ppm` 若缺少来源推荐和成本拆解，会成为专家指标，玩家仍无法判断下一步降本动作。
- 回归风险：新增可选字段可能触发旧测试的事件断言差异。

缓解：
- 首版保持字段精简且 `serde(default)` 兼容。
- 阈值使用保守值，先验证“有信号”再精调。
- `market_quote_decision_preview` 只解释现有行情、税费和运输损耗，不新增订单簿、撮合交易动作、市场 UI 或治理系统。
- 对依赖时序的测试采用“推进至稳定态”断言，避免脆弱断言。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-M4-P2-001: `market_quotes` 不能只暴露成本指数；当存在本地缺口、运输损耗或治理税时，必须给出玩家侧推荐来源、成本主因和下一步降本动作。本决策不重平衡价格、税率、运输损耗或材料分布。
