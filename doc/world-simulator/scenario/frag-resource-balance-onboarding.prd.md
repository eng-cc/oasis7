# oasis7 Simulator：Frag 资源平衡与新手友好生成（设计文档）

- 对应设计文档: `doc/world-simulator/scenario/frag-resource-balance-onboarding.design.md`
- 对应项目管理文档: `doc/world-simulator/scenario/frag-resource-balance-onboarding.project.md`

审计轮次: 5

本分册定义一版可落地的 frag 资源生成改造，目标是同时满足两点：
1) 资源产出不出现明显“前期断供/后期通胀”；
2) 新手开局在默认场景下更稳定地获得可采集 frag。

## 1. Executive Summary
- 降低 chunk 级生成方差：避免默认参数下出现“部分 chunk 几乎无 frag”的冷启动断供。
- 增强新手可达性：让 chunk 中心区域更容易出现 frag，减少开局盲目搜索成本。
- 补齐运行期恢复节奏：当 chunk frag 数量低于上限时，按固定周期小步补种，避免一次性暴涨。
- 补齐资源类型空间分布：支持按区域偏置材质分布，形成可解释的“中心/边缘”资源差异。
- 当该分布用于首局推荐 frag 时，玩家侧应看到粗粒度材质预期，而不是只在生成策略中隐藏该差异。
- 保持确定性：同 `seed + config + chunk` 生成结果一致，便于回放与调试。
- 保持现有预算体系兼容：不改动 `total/remaining` 账本守恒规则。

## 2. User Experience & Functionality

### In Scope
- 扩展 `AsteroidFragmentConfig`：
  - `min_fragments_per_chunk`：每 chunk 最低 frag 数量保底。
  - `starter_core_radius_ratio`：中心引导区半径占比（0~1）。
  - `starter_core_density_multiplier`：中心区密度倍率（>=1）。
  - `replenish_interval_ticks`：运行期补种周期（tick）。
  - `replenish_percent_ppm`：每周期补种比例（ppm，默认 1%）。
  - `material_distribution_strategy`：资源类型分布策略（均匀/分区偏置）。
- 改造 `generate_fragments`：
  - 在体素泊松采样基础上，叠加中心区密度倍率；
  - 采样结束后执行确定性保底补种，尽量达到 `min_fragments_per_chunk`；
  - 材质选择支持按空间分区策略调整权重。
- 玩家侧 starter frag hint：
  - 推荐首个采集 frag 时，允许从 `material_distribution_strategy` 派生粗粒度 `expected_material_hint`。
  - hint 应说明该 frag 更可能支持的第一工业目标，例如金属/复合材料倾向可支撑早期工业加工，挥发物倾向可支撑补给或后续路线。
  - hint 不得承诺精确产出、稀有掉落或固定收益。
- 改造运行期补种：
  - `WorldKernel::step` 每经过 `replenish_interval_ticks` 检查可生成 chunk；
  - 对低于 `max_fragments_per_chunk` 的 chunk，按 `replenish_percent_ppm` 补入新 frag；
  - 补种结果写入独立事件，保障 replay 一致性。
- 测试补充：覆盖“保底数量生效”“中心分布偏置生效”“配置 sanitize 边界”。

### Out of Scope
- 按玩家行为实时动态调参（在线经济反馈回路）。
- 新资源品类、加工链条或 UI 引导流程重构。
- 跨 chunk 的新保底策略（当前仅保证单 chunk 生成器语义）。
- 在线经济反馈闭环（玩家行为驱动的实时调参）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 配置扩展
在 `AsteroidFragmentConfig` 新增字段：
- `min_fragments_per_chunk: i64`
  - 默认值：`6`
  - 约束：`0 <= min_fragments_per_chunk <= max_fragments_per_chunk`
- `starter_core_radius_ratio: f64`
  - 默认值：`0.35`
  - 约束：`0.0..=1.0`
- `starter_core_density_multiplier: f64`
  - 默认值：`1.6`
  - 约束：`>= 1.0`
- `replenish_interval_ticks: i64`
  - 默认值：`100`
  - 约束：`>= 0`（`0` 表示关闭运行期补种）
- `replenish_percent_ppm: i64`
  - 默认值：`10_000`（1%）
  - 约束：`0..=1_000_000`
- `material_distribution_strategy: MaterialDistributionStrategy`
  - 默认值：`uniform`
  - 可选：
    - `uniform`：沿用单一 `material_weights`
    - `core_metal_rim_volatile`：中心提升 metal/composite，边缘提升 ice/carbon

### 生成公式（增量）
- 体素中心平面距离归一化：
  - `r = planar_distance_to_chunk_center / max_planar_distance`
- 中心区判定：
  - `r <= starter_core_radius_ratio` 视为中心区。
- 密度倍率：
  - 中心区 `density *= starter_core_density_multiplier`
  - 非中心区 `density` 不变。

### 保底补种策略
- 正常体素采样完成后，若 `current_count < min_fragments_per_chunk`，触发补种。
- 补种在 chunk 空间内进行随机候选投放，沿用已有半径/材质/间距判定。
- 补种为有界循环（固定最大尝试次数），不可达时允许低于保底值，但必须可终止。

### 运行期补种策略（FRB4）
- 触发条件：`time % replenish_interval_ticks == 0` 且 `current_fragments < max_fragments_per_chunk`。
- 目标补种量：
  - `target = ceil(max_fragments_per_chunk * replenish_percent_ppm / 1_000_000)`；
  - 每次至少 `1`（仅在存在缺口时）。
- 结果落账：
  - 新 frag 写入 `WorldModel.locations`；
  - 新 frag 的 `fragment_budget` 计入对应 `chunk_resource_budgets`；
  - 追加 `FragmentsReplenished` 事件（包含新增 frag 明细），用于 replay。

### 运行期补种玩家预览
- 当玩家所在 chunk / 当前目标 frag 已低资源、采空或下一工业目标受材料不足阻塞时，玩家侧应能读取 `resource_replenishment_quote` / `fragment_refill_preview`，用于判断等待补种、转移到其他 frag/chunk，还是改走替代材料路线。
- 最小字段：
  - `chunk_coord`
  - `target_frag_id`
  - `current_frag_remaining_summary`
  - `chunk_remaining_summary`
  - `next_replenish_tick`
  - `ticks_until_replenish`
  - `estimated_replenished_frag_count`
  - `estimated_replenished_resource_hint`
  - `next_industrial_goal_relevance`
  - `wait_cost_summary`
  - `recommended_resource_action`: `wait_for_replenishment` / `move_to_other_frag` / `move_to_other_chunk` / `switch_material_route`
- Edge case: 若资源不足或采空场景推荐等待、转移或替代路线，但玩家看不到下一次补种 tick、预计补种量、等待成本或对下一工业目标的帮助，标记为 `resource_replenishment_quote_missing`。
- Acceptance: 玩家在决定等待当前 chunk 恢复、转移采集目标或切换材料路线前，至少能看懂“何时补、补多少、是否足够推进当前目标、等待会损失什么、系统推荐哪条路”。
- Non-goal: 本预览不改变 `replenish_interval_ticks`、`replenish_percent_ppm`、`FragmentsReplenished` replay 语义、资源分布权重、保底补种算法或经济平衡参数。

### 资源类型分布策略（FRB4）
- `uniform`：所有区域使用同一 `material_weights`。
- `core_metal_rim_volatile`：
  - 中心区（`r <= starter_core_radius_ratio`）：提高 `metal/composite` 权重；
  - 外缘区：提高 `ice/carbon` 权重；
  - 中间区：权重平滑回落，避免阶跃突变。

### 首局 frag 材质预期提示
- 最小字段：
  - `target_frag_id`
  - `expected_material_hint`
  - `starter_value_reason`
  - `distance_or_accessibility_reason`
  - `first_recipe_relevance`
- Acceptance: 当系统推荐首局采集某个 frag 时，行动前必须说明“为什么是这个 frag”和“它可能支持哪个第一工业目标”。
- Edge case: 若推荐采集目标但缺少材质预期/工业关联说明，标记为 `starter_frag_hint_missing`。

### 兼容性
- 与 `max_fragments_per_chunk` 三档预算兼容：
  - 生成器侧保底不应突破上层 chunk 预算裁剪语义。
- 与回放兼容：
  - 运行期补种通过事件重放，不依赖“隐式重算”。
- 与场景覆盖兼容：
  - 场景未配置新字段时使用默认值；
  - 已有 `min_fragment_spacing_cm` 覆盖机制保持不变。

## 5. Risks & Roadmap
- **FRB1**：输出本设计文档与项目管理文档。
- **FRB2**：完成配置与生成器实现，补齐 `test_tier_required` 覆盖。
- **FRB3**：回写总项目文档与当日日志，完成回归与收口。
- **FRB4**：完成运行期补种与资源类型分布策略（含 replay 与测试）。

### Technical Risks
- 中心区倍率过高可能形成“中心资源过热”，抬高后期通胀风险。
- 保底补种若尝试上限过高，可能放大极端参数下的生成耗时。
- 生成数量提高会间接增加预算体量，需要持续观察采集与加工节奏指标。
- 运行期补种若参数过大，可能导致中后期资源膨胀与事件日志增长过快。
- 材质提示若写得过准，可能被玩家误读为固定掉落承诺；必须保持 coarse hint。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-FRB-001: `material_distribution_strategy` 的玩家侧用途是解释“为什么先采这个 frag”，不是暴露完整资源表或重平衡资源生成。
