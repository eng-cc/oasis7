# M4 资源与产品系统：合理性与可玩性一体化设计（2026-02-27）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.project.md`

审计轮次: 5


## 1. Executive Summary
- 设计一套既符合当前架构约束、又能持续产生策略决策的资源与产品系统。
- 在保持“最小内建资源 + 模块扩展资源”原则下，提升产业链深度、物流博弈和中后期可玩性。
- 为后续实现提供可直接接入 runtime/simulator 的数据契约与测试口径。

## 现状基线（与当前实现对齐）
- 内建资源已收敛为 `Electricity`、`Data`，产业语义通过材料字符串与模块定义承载。
- M4 已具备基础闭环：工厂建造、配方排产、产物校验、账本入账。
- 产物校验不能只作为 `ProductValidated` 成功日志；玩家需要知道该产品验证后解锁了什么能力、推进哪个阶段、下一步最值得如何使用。
- 多账本与物流约束已存在：`world/agent/site/factory` 账本、在途延迟、运损、并发上限。
- 治理与经济基础已存在：供需电价、税费、电费、合约与声誉反刷。

当前主要问题：
- 工业链层级偏浅，关键中间件竞争不足。
- 物流策略维度偏少，缺少时效优先级和运力分层。
- 资源消耗口偏弱，中后期易出现库存堆积。
- 新手到规模化阶段缺少清晰目标梯度。

## 2. User Experience & Functionality

### In Scope
- 资源层级与产品分级重构（不破坏现有 API 主路径）。
- “合理性”规则：来源、加工、运输、维护、回收、衰减闭环。
- “可玩性”规则：瓶颈竞争、阶段目标、风险收益、策略分工。
- 数据契约建议：`Material/Product/Recipe` 扩展档案结构。

### Out of Scope
- 跨服结算与复杂金融衍生品。
- 一次性重写 viewer 交互框架。
- 改动共识主流程或破坏现有事件回放兼容。

## 设计原则
- 最小内核稳定：内核继续只保障账本、不变量、审计与回放。
- 产业语义外置：资源与产品规则优先由模块与配置档案表达。
- 决策密度优先：每个阶段都必须有“取舍”而非单一路径最优。
- 可测优先：每条新增链路必须能进入 `test_tier_required` 或 `test_tier_full`。

## 资源与产品分层

### R0 内建基础资源（内核强校验）
- `Electricity`：生产与物流活动的通用驱动成本。
- `Data`：策略、交易与治理活动的通用结算/门槛资源。

### R1 原料层（模块定义材料）
- 示例：`iron_ore`、`copper_ore`、`carbon_fuel`、`silicate_ore`、`rare_earth_raw`。
- 主要特征：低加工价值、高运输体积，受区域分布与运损影响显著。

### R2 中间材料层
- 示例：`iron_ingot`、`copper_wire`、`alloy_plate`、`polymer_resin`、`circuit_substrate`。
- 主要特征：跨链复用，构成瓶颈争夺核心。

### R3 功能部件层
- 示例：`gear`、`control_chip`、`motor_mk1`、`sensor_pack`、`power_core`。
- 主要特征：吞吐受工厂槽位、功耗、耐久共同制约。

### R4 终端产品层
- 示例：`logistics_drone`、`field_repair_kit`、`survey_probe`、`module_rack`。
- 主要特征：直接服务扩产、维护、探索、治理收益。

### R5 基础设施件层
- 示例：`factory_core`、`relay_tower_kit`、`grid_buffer_pack`。
- 主要特征：用于建设更高层工厂与跨区物流节点，形成长期投资回报。

## 可玩性闭环设计

### 闭环 A：生存-启动（前期）
- 目标：保证小规模稳定产出，避免早期断供。
- 关键取舍：先补能源/维护，还是先做扩产部件。
- 成功指标：前 N tick 内关键链路拒绝率降低且无长时间停摆。

### 闭环 B：扩产-竞争（中期）
- 目标：围绕共享中间件形成排产竞争与物流调度。
- 关键取舍：本地低效加工 vs 跨区高损耗运输。
- 成功指标：跨账本转运显著增加，且队列优先级有策略分化。

### 闭环 C：治理-博弈（后期）
- 目标：把产业链与税费/禁区/合约信誉联动。
- 关键取舍：高税高安全区稳定经营 vs 低税高风险区高收益。
- 成功指标：治理参数变化可显著影响价格、运输与产线选择。

## 合理性机制（防通胀与长期稳定）
- 耐久折旧：产线负载越高，工厂耐久消耗越快。
- 维护消耗：高阶产品维持运行需持续消耗 R2/R3 材料。
- 回收损耗：回收不是无损返还，保留衰减比例。
- 库存衰减（可选）：对特定易损材料施加低幅衰减，抑制囤积。
- 物流损耗分层：高体积低价值材料跨区运输成本显著更高。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 新增标准档案（建议）

`MaterialProfileV1`（可挂 runtime 配置或模块清单）
- `kind: String`
- `tier: u8`（R1~R5）
- `category: String`（ore/intermediate/component/product/infrastructure）
- `stack_limit: i64`
- `transport_loss_class: String`（low/medium/high）
- `decay_bps_per_tick: i64`
- `default_priority: String`

`ProductProfileV1`
- `product_id: String`
- `role_tag: String`（survival/scale/explore/governance）
- `maintenance_sink: Vec<MaterialStack>`
- `tradable: bool`
- `unlock_stage: String`

`RecipeProfileV1`
- `recipe_id: String`
- `bottleneck_tags: Vec<String>`
- `stage_gate: String`
- `preferred_factory_tags: Vec<String>`

### 2) 与现有动作事件的接线策略
- 保持现有动作主路径：`BuildFactoryWithModule`、`ScheduleRecipeWithModule`、`ValidateProductWithModule`、`TransferMaterial`。
- 保持现有事件主路径：`Factory*`、`Recipe*`、`ProductValidated`、`MaterialTransit*`。
- 增量原则：优先新增可选字段与旁路观测，避免破坏旧模块。
- 玩家侧 `product_validation_quote` / `validation_unlock_preview` 合同：
  - `product_id`
  - `role_tag`: `survival / scale / explore / governance`
  - `unlock_stage_before`
  - `unlock_stage_after`
  - `tradable`
  - `capability_unlocked`
  - `next_use_case`
  - `stage_progress_delta`
  - `recommended_next_action`
  - `validation_value_class`: `capability_unlock / trade_value / stage_progress / low_immediate_value`
- Edge case: 若 `ValidateProductWithModule` / `ProductValidated` 能确认产物有效，但玩家看不到能力解锁、阶段推进或下一步用途，标记为 `product_validation_preview_missing`。
- Acceptance: 玩家校验产品前后，至少能看懂“这个产品验证成功后解锁什么能力、推进哪个阶段、下一步最值得怎么用”。

### 3) 运输优先级（建议最小改动）
- `TransferMaterial` 增加可选 `priority`：`standard` / `urgent`。
- 在途队列按 `priority -> ready_at -> job_id` 排序。
- 观测补充：`MaterialTransitStarted/Completed` 记录 SLA 标签。

### 4) 阶段进度状态（建议）
- 新增轻量 `IndustryStageProgress`：`bootstrap`、`scale_out`、`governance`。
- 只做提示与观测，不作为硬阻断规则。

## 平衡参数建议（首版）
- R1 跨区运损权重 > R2 > R3/R4（鼓励就地初加工）。
- 折旧系数与配方复杂度正相关（高价值产物承担更高维护压力）。
- 共享中间件至少覆盖 2 条终端链（保证竞争而非孤链最优）。
- 早期关键配方保持低门槛，后期高阶产品通过维护成本形成持续资源回流。

## 5. Risks & Roadmap
- M0（本轮）：设计与任务拆解完成。
- M1（P0）：共享中间件竞争 + 运输优先级 + 观测指标。
- M2（P1）：耐久/维护/回收强化 + 区域稀缺分布接线。
- M3（P2）：阶段目标引导 + 市场/治理联动验证。

## 测试口径

### test_tier_required
- 共享中间件导致的排产冲突与拒绝路径。
- 运输优先级排序和 SLA 基础正确性。
- 维护/回收守恒与负库存保护。

### test_tier_full
- 多工厂、多账本、跨区运输、治理税费联合回归。
- 长跑场景下库存、耐久、事件回放一致性。
- LLM 闭环场景下阶段达成率与拒绝率趋势验证。

### Technical Risks
- 复杂度上升风险：层级增加后调参空间显著扩大。
- 行为漂移风险：旧策略在新瓶颈下可能表现退化。
- 兼容风险：历史模块未提供新增档案字段。
- 可观测性风险：若指标定义不统一，难以评估改动收益。
- 可读性风险：产物校验若只记录成功，会把首个制成品降级成日志事件，而不是能力解锁。

缓解策略：
- 采用“先观测后强约束”的渐进落地。
- 新字段尽量可选并提供默认语义。
- `validation_unlock_preview` 只解释已有产物档案、阶段状态与下一步用途，不新增完整成就系统、科技树或产品目录重做。
- 不重平衡产品链、配方、阶段门槛，不改变 runtime ABI 或 viewer 实现。
- 每个里程碑绑定 required/full 测试与回放验证。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
