# oasis7 Simulator：分块世界生成与碎片元素/化合物池（设计文档）

- 对应设计文档: `doc/world-simulator/scenario/chunked-fragment-generation.design.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history; 本文与 design 保留 chunk、资源预算与 replay 合同。

审计轮次: 5

本分册定义“按探索触发”的世界分块生成流程，并补充碎片的几何与物理量模型（体积/密度/质量）、化合物主导组成、块状分布表达。

## 1. Executive Summary
- 按探索进度进行世界生成：**未探索区块不生成**，降低初始化与存储开销。
- 固定分块尺寸为 **20km × 20km × 10km**，形成稳定可复现的空间切片。
- 明确碎片物理量：每个碎片可由多个长方体块组成，并具备**体积、密度、质量**。
- 化学组成以**化合物为主**（非单质），并可映射回元素统计口径。
- 区块生成时一次性计算资源 `total/remaining`，保证开采过程只扣减不重算。
- 保持同一 `world_seed` 下的确定性（同 chunk 坐标得到同结果）。

## 2. User Experience & Functionality

### In Scope
- 定义 chunk 坐标、边界和生命周期状态。
- 定义碎片块（长方体）几何表达与 1cm 最小单位约束。
- 定义碎片级体积/密度/质量计算规则。
- 定义化合物池与元素映射关系（统计口径）。
- 定义 chunk 级/碎片级资源总量与剩余量数据结构。
- 定义按探索触发的 chunk 生成步骤与种子策略。

### Out of Scope
- 连续体轨道动力学、碰撞、潮汐等高精度物理。
- 多线程并行生成与跨节点分布式锁。
- 完整冶炼工艺链（本阶段只定义“资源库存生成”，不定义加工玩法）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 分块常量与坐标
- `CHUNK_SIZE_X_CM = 2_000_000`（20 km）
- `CHUNK_SIZE_Y_CM = 2_000_000`（20 km）
- `CHUNK_SIZE_Z_CM = 1_000_000`（10 km）
- `ChunkCoord { x: i32, y: i32, z: i32 }`
- `ChunkBounds { min: GeoPos, max: GeoPos }`

说明：默认空间 `100km × 100km × 10km` 切分为 `5 × 5 × 1` 个 chunk。

### Chunk 生命周期
- `Unexplored`：未知区块，仅存在索引信息。
- `Generated`：已生成碎片几何、化学组成与资源量，可观测/开采。
- `Exhausted`：区块可开采资源耗尽（可选状态，用于优化查询）。

### 几何与物理量（新增）

#### 最小空间单位
- **1 cm**（`SPACE_UNIT_CM = 1`）
- 所有 block 尺寸、block 原点、chunk 内偏移均以 cm 整数表示。
- `GeoPos`、碎片中心与 chunk 边界统一以整数厘米表示，禁止生成或持久化 sub-cm 坐标。

#### 单块结构（长方体）
- `FragmentBlock`
  - `origin_cm: GridPosCm`（chunk 局部坐标）
  - `size_cm: CuboidSizeCm`（`x_cm/y_cm/z_cm`）
  - `density_kg_per_m3: i64`
  - `compounds: CompoundComposition`

约束：
- `size_cm.x_cm >= 1`
- `size_cm.y_cm >= 1`
- `size_cm.z_cm >= 1`
- block 之间不重叠（同一碎片内）

#### 体积/密度/质量公式
- `volume_cm3 = x_cm * y_cm * z_cm`
- `volume_m3 = volume_cm3 / 1_000_000`
- `mass_kg = density_kg_per_m3 * volume_m3`
- 工程落地建议存储整数克：
  - `mass_g = density_kg_per_m3 * volume_cm3 / 1000`

#### 碎片聚合物理量
- `fragment_volume_cm3 = Σ(block.volume_cm3)`
- `fragment_mass_g = Σ(block.mass_g)`
- `fragment_bulk_density_kg_per_m3 = fragment_mass_kg / fragment_volume_m3`

### 化学组成：化合物主导（新增）

#### 化合物池（示例）
- `FragmentCompoundKind`
  - `SilicateMatrix`（硅酸盐基质）
  - `IronNickelAlloy`（铁镍合金）
  - `WaterIce`（水冰）
  - `HydratedMineral`（含水矿物）
  - `CarbonaceousOrganic`（碳质有机复合物）
  - `SulfideOre`（硫化物矿）
  - `RareEarthOxide`（稀土氧化物）
  - `UraniumBearingOre`（铀矿相）
  - `ThoriumBearingOre`（钍矿相）

#### 组成表达
- `CompoundComposition`: `BTreeMap<FragmentCompoundKind, u32>`（ppm）
- `ElementComposition`: `BTreeMap<FragmentElementKind, u32>`（ppm，统计口径）

说明：
- 生成阶段优先采样**化合物组成**；
- 元素组成由化合物签名映射得到（用于资源统计、查询、开采规则）；
- 单质可作为极低概率特例，不作为默认主组成。

### 资源库存模型（生成即定量）
- `FragmentResourceBudget`
  - `total_by_element: BTreeMap<FragmentElementKind, i64>`
  - `remaining_by_element: BTreeMap<FragmentElementKind, i64>`
- `ChunkResourceBudget`
  - `total_by_element: BTreeMap<FragmentElementKind, i64>`
  - `remaining_by_element: BTreeMap<FragmentElementKind, i64>`

计算原则：
1. 先生成碎片及其 block 列表（长方体）；
2. 按 block 采样化合物组成（ppm）与密度；
3. 计算每个 block 的体积/质量；
4. 将化合物组成映射为元素组成；
5. 计算 `element_total = mass * composition * recoverability`；
6. 生成时写入 `total` 与 `remaining = total`，后续仅做扣减。

实现口径（CG4）：
- 账本单位统一为克（g）。
- 默认恢复率 `DEFAULT_ELEMENT_RECOVERABILITY_PPM = 850_000`（85%）。
- `element_total_g = fragment_mass_g * element_ppm / 1_000_000 * recoverability_ppm / 1_000_000`。
- `ChunkResourceBudget` 等于该 chunk 所有碎片预算逐元素累加。
- 开采扣减时必须同时更新 `fragment_budget.remaining` 与 `chunk_budget.remaining`。
- 守恒约束：任一元素始终满足 `0 <= remaining <= total`。

### 关键接口（建议）
- `fn chunk_coord_of(pos: GeoPos) -> ChunkCoord`
- `fn ensure_chunk_generated(coord: ChunkCoord, kernel: &mut WorldKernel)`
- `fn generate_chunk(seed: u64, coord: ChunkCoord, config: &WorldConfig) -> ChunkSnapshot`
- `fn chunk_seed(world_seed: u64, coord: ChunkCoord) -> u64`
- `fn infer_element_ppm(compounds: &CompoundComposition) -> ElementComposition`
- `fn synthesize_fragment_budget(profile: &FragmentPhysicalProfile) -> FragmentResourceBudget`
- `fn consume_fragment_resource(location_id: &str, element: FragmentElementKind, amount_g: i64)`

### 场景接入配置（CG5）
- `asteroid_fragment.bootstrap_chunks: Vec<ChunkCoord>`：场景可显式声明启动即生成的 chunk 列表。
- 初始化顺序：`seed_positions -> bootstrap_chunks -> agent_spawn_positions`。
- 分块尺寸固定：`20km × 20km × 10km`（常量 `CHUNK_SIZE_X/Y/Z_CM`），场景不可覆盖。
- 当前场景与生成器只接受规范名 `asteroid_fragment`；已完成的旧命名迁移不保留字段兼容别名。
- `min_fragment_spacing_cm` 的生成器默认值为 `50_000` cm（500m）；有效值 `<= 0` 表示关闭额外 spacing 约束。场景未显式覆盖时沿用 `WorldConfig.asteroid_fragment`，覆盖优先级与初始化边界由 `scenario-files.prd.md` 定义。

### Frag 密度、材质分布与保底配置

`AsteroidFragmentConfig` 的稳定资源生成合同包括：

- `min_fragments_per_chunk = 6`，sanitize 到 `0..=max_fragments_per_chunk`；正常采样不足时执行确定性、有界保底，约束不可满足时允许低于目标但必须终止。
- `starter_core_radius_ratio = 0.35`，sanitize 到 `0.0..=1.0`；归一化平面半径不超过该值的候选进入 starter core。
- `starter_core_density_multiplier = 1.6`，sanitize 到至少 `1.0`；只提高 starter core 的采样密度，不保证固定资源数量或玩家收益。
- `material_distribution_strategy = uniform`；`uniform` 沿用统一材质权重，`core_metal_rim_volatile` 按 core / middle / rim 三段离散径向区间使用固定权重：中心提高 metal/composite，中段采用折中权重，外缘提高 ice/carbon。
- 同一 `world_seed + config + chunk_coord` 产生相同候选、保底与材质选择。保底对每个缺口最多尝试 24 轮、每个候选最多尝试 8 次，继续遵守间距与 chunk 数量上限。

这些参数属于生成器权威，不进入产品承诺；starter core 与最低数量只降低极端冷启动概率，不消除世界差异，也不保证第一工业目标必然完成。

### 运行期 Frag 补种与落账

- `replenish_interval_ticks = 100`；负值 sanitize 为 `0`，`0` 表示关闭。仅 asteroid-fragment runtime 已启用、当前 tick 命中非零周期、且 generated/exhausted chunk 低于 `max_fragments_per_chunk` 时补种。
- `replenish_percent_ppm = 10_000`（1%），sanitize 到 `0..=1_000_000`。每次目标量为 `min(missing, max(1, ceil(max_fragments_per_chunk * ppm / 1_000_000)))`。
- 随机源由 asteroid-fragment seed、当前 tick、固定 salt 与 `chunk_seed` 派生；候选必须通过本 chunk 与相邻 26 个 chunk 的 spacing 校验，因此同一权威输入可复现。
- 成功候选先写入 locations，并把每个 fragment budget 聚合到 chunk budget，再原子记录 `FragmentsReplenished { entries }`。
- replay 只应用事件中已提交的 replenishment entries，不重新运行补种算法；资源 `total/remaining` 守恒和 committed-delta 恢复边界保持不变。

聚焦回归覆盖配置 sanitize、每 chunk 保底、starter core 偏置、材质分区、100 tick / 1% 补种与 replay 应用等价性。实现或默认值变化时必须同步这些合同和对应测试，不能只更新玩家预览。

### 原生分辨率声明（2026-05-07 对齐）
- `chunk-grid`
  - native resolution: 固定 `20km × 20km × 10km`
  - cm mapping: `GeoPos` 以 cm 真值进入 chunk 除法
  - rounding: floor 到 chunk index；当位置恰好落在世界上边界时 clamp 到最后一个合法 chunk
- `asteroid-fragment-voxel`
  - native resolution: `AsteroidFragmentConfig.voxel_size_km`，默认 `10km`，sanitize 后最小 `1km`
  - cm mapping: voxel bounds 先转为 chunk 局部 cm 空间，再随机生成碎片中心
  - rounding: 候选中心坐标 round 到最近整数 cm 后再写入 `Location.pos`
- `fragment-block-geometry`
  - native resolution: block 仍使用 cm 几何，但不是“任意小数厘米”
  - cm mapping: `origin_cm/size_cm` 都直接落在 cm 真值
  - rounding: 任意 `<1cm` 边长一律 clamp 到 `1cm`

### 运行时触发契约（与 observe/act 集成）
- `observe` 触发：当 Agent 进行观测时，先对“自身所在坐标 chunk”执行 `ensure_chunk_generated`，再构建 observation。
- `move` 触发：校验移动动作前，必须保证 `from_chunk` 与 `to_chunk` 已生成。
- `harvest/transfer/query` 触发：动作依赖 location 资源时，必须先确保该 location 所在 chunk 已生成。
- 统一顺序：`ensure_chunk_generated -> action validation -> action apply -> event append`。
- 一致性要求：同一 tick 内多个 Agent 命中同一未生成 chunk 时，只允许一次成功生成，其余请求复用结果。

### 持久化与回放契约（M2 对齐，CG6 落地）

#### 快照字段（落地）
- `WorldSnapshot.chunk_generation_schema_version: u32`
  - 标记 chunk 生成/校验契约版本（与 snapshot/journal 主版本独立）。
- `WorldSnapshot.chunk_runtime`
  - 持久化 `world_seed / asteroid_fragment_enabled / seed_offset / spacing`，保证回放时可重建同一生成上下文。
- `WorldModel.chunks + WorldModel.chunk_resource_budgets`
  - 快照内保留完整 chunk 状态与资源账本（`total/remaining`）。

#### 事件新增类型（落地）
- `ChunkGenerated {`
  - `coord: ChunkCoord`
  - `seed: u64`
  - `fragment_count: u32`
  - `block_count: u32`
  - `chunk_budget: ChunkResourceBudget`
  - `cause: ChunkGenerationCause`（`init` / `observe` / `action`）
- `}`

#### 回放规则（落地）
- 回放遇到 `ChunkGenerated` 时，使用事件中的 `coord + seed` 与快照中的 `chunk_runtime` 重放该 chunk 生成。
- 回放后必须校验 `fragment_count / block_count / chunk_budget` 与事件载荷一致；不一致即 `ReplayConflict`。
- `snapshot.version=2` / `journal.version=2` 允许迁移到当前版本 `v3`（CG6），并补齐 `chunk_generation_schema_version` 默认值。
- 其他未知版本保持拒绝加载，避免无声破坏账本一致性。

#### 资源恢复预览约束（与 FRB4 对齐）
- `FragmentsReplenished` 作为运行期恢复事件时，不应只作为 replay / 后台账本事件存在；当玩家面临 chunk/frag 资源不足、目标 frag 采空或第一工业目标缺料时，玩家侧应能读取 `resource_replenishment_quote` / `fragment_refill_preview`。
- 预览必须把 `remaining_by_element` / `remaining_by_compound` 的粗粒度状态、下一次补种 tick、预计补种数量、等待成本和当前工业目标关联转换为玩家选择：等待当前 chunk、移动到其他 frag/chunk，或切换替代材料路线。
- 若动作或策略推荐等待补种，但缺少 `next_replenish_tick`、`estimated_replenished_frag_count`、`next_industrial_goal_relevance` 或 `recommended_resource_action`，标记为 `resource_replenishment_quote_missing`；该缺口不改变生成即定量、采集扣减、回放校验或资源守恒规则。

### 经济资源映射契约（与 M4 对齐，CG8 落地）
为接入现有 `electricity/hardware/data` 三类核心资源，先落地“化合物质量 -> hardware”的最小闭环：

- `RefineCompound`（动作）：输入 `owner + compound_mass_g`。
- `CompoundRefined`（事件）：输出 `electricity_cost + hardware_output`。
- `electricity` 作为加工消耗项；`hardware` 作为加工产物。
- `data` 与高级工艺链（`ManufactureHardware/SynthesizeData`）留到后续迭代。
- 玩家侧提交前应能读取 `refine_quote` / `refine_preview`，解释本次精炼消耗多少电力、产出多少硬件、是否足以推进首个工厂/制成品目标；不得只在执行后事件里暴露扣电与产出。

参数与公式：
- `WorldConfig.economy.refine_electricity_cost_per_kg`（默认 `2`）
- `WorldConfig.economy.refine_hardware_yield_ppm`（默认 `1000`）
- `electricity_cost = ceil(compound_mass_g / 1000) * refine_electricity_cost_per_kg`
- `hardware_output = compound_mass_g * refine_hardware_yield_ppm / 1_000_000`
- 预览字段：
  - `compound_mass_g`: 本次计划投入的 compound 质量。
  - `electricity_cost`: 本次预计消耗的电力。
  - `hardware_output`: 本次预计产出的 hardware。
  - `electricity_after`: 成功精炼后的预计剩余电力。
  - `hardware_shortfall_before`: 精炼前距离当前第一工业目标所需 hardware 的缺口。
  - `hardware_shortfall_after`: 精炼后距离当前第一工业目标所需 hardware 的缺口。
  - `first_goal_relevance`: 本次产出将如何推进首个工厂、首个制成品或首条稳定产线。
  - `recommended_refine_amount`: 推荐精炼质量；应避免默认把所有 compound 一次性转成 hardware。
  - `refine_value_class`: `enough_for_next_step / partial_progress / poor_power_tradeoff`。

最小守恒与约束：
- 输入约束：`compound_mass_g > 0`。
- 电力约束：`owner.electricity >= electricity_cost`，否则 `ActionRejected(InsufficientResource)`。
- 产出约束：`hardware_output > 0`，否则 `ActionRejected(InvalidAmount)`。
- 落账约束：成功动作必须追加 `CompoundRefined`，并在回放中重放扣电与加硬件。
- Edge case: 若 `RefineCompound` 可提交但玩家看不到 `electricity_cost`、`hardware_output` 或 `hardware_shortfall_before/after`，标记为 `refine_quote_missing`；该缺口属于精炼净收益/电力机会成本可读性问题，不改变公式、产率或拒绝条件。
- Acceptance: 精炼前玩家至少能判断“花这些电，换来的 hardware 是否足够推进当前第一工业目标”；若不足，应给出推荐精炼量或提示先采矿/先补电。

### 跨 chunk 边界一致性规则（CG7 落地）
- 归属规则：碎片按中心点归属 chunk；候选碎片生成时先做“本 chunk 内 spacing”，再做“邻接 chunk（26 邻域）校验”。
- 邻块校验：当前 chunk 生成时，读取已生成邻块碎片并执行 `radius_a + radius_b + spacing` 距离约束。
- 边界保留：新增 `WorldModel.chunk_boundary_reservations: BTreeMap<ChunkCoord, Vec<BoundaryReservation>>`。
- 保留写入：若邻块处于 `Unexplored` 且候选碎片到邻块包围盒距离 `<= radius + spacing`，写入 `BoundaryReservation`。
- 保留消费：邻块未来生成时先消费（remove）该 chunk 的 reservations，再据此过滤候选碎片。
- 确定性策略：冲突默认“已存在碎片优先（先生成者优先）”，保证同 seed + 同触发序列结果可复现。
- 回归边界：相同 `world_seed + effective asteroid_fragment config + chunk trigger order` 必须得到相同布局；验证既覆盖已放置碎片，也覆盖跨 chunk 邻居和 reservation。replay 只应用已提交的生成/补种 delta，不用当前默认值重新生成历史布局。

### 性能预算与降级策略（CG9 落地）
新增三档硬预算（`AsteroidFragmentConfig`）：
- `max_fragments_per_chunk`：单 chunk 最多接纳碎片数（默认 `4_000`）。
- `max_blocks_per_fragment`：单碎片最多保留 block 数（默认 `64`）。
- `max_blocks_per_chunk`：单 chunk 最多保留 block 总量（默认 `120_000`）。

确定性降级顺序（同 seed 同触发序列可复现）：
1. 先应用 `max_fragments_per_chunk`（超过上限后停止接纳新碎片）。
2. 再应用 `max_blocks_per_fragment`（超限时按生成顺序截断 block，并重算体积/质量/成分）。
3. 最后应用 `max_blocks_per_chunk`（根据 chunk 剩余 block 预算截断当前碎片，预算耗尽后停止接纳后续碎片）。

说明：`ChunkGenerationSkipped(reason=budget_exceeded)` 事件暂不在 CG9 引入，后续在更高阶性能治理阶段落地。

### 验收标准（DoD）
- 同一 `world_seed + chunk_coord` 在不同运行中生成结果一致（碎片数量/资源账本一致）。
- 未访问 chunk 不进入 `Generated` 状态。
- 回放后 chunk 状态与资源账本与原运行一致。
- RefineCompound 链路满足电力约束与回放一致性。
- RefineCompound 预览满足 `refine_quote` 可读性：电力成本、硬件产出、精炼后电力、目标缺口前后变化和第一工业目标关联均可追溯。
- FragmentsReplenished 预览满足 `resource_replenishment_quote` 可读性：下一次补种 tick、预计补种量、当前剩余量摘要、等待成本、第一工业目标关联和推荐资源行动均可追溯。
- 在默认预算下，批量生成不出现 OOM 或极端耗时。

### 回放一致性与性能回归测试（CG10 落地）
- 回放一致性：对携带预算约束的 `ChunkGenerated` 事件执行重放，校验 `fragment_count/block_count/chunk_budget` 与原运行一致。
- 预算回归：多 chunk 批量初始化下逐块验证 `max_fragments_per_chunk / max_blocks_per_fragment / max_blocks_per_chunk` 约束。
- 防回归策略：覆盖 `init` 触发与 `action` 触发两条路径，避免后续改动绕过预算裁剪。

## 世界生成步骤
1. **初始化索引阶段**：创建 chunk 网格索引，状态置 `Unexplored`。
2. **引导区块阶段**：预生成 origin/初始基地/Agent 出生点所在 chunk，并支持场景 `asteroid_fragment.bootstrap_chunks` 显式指定起始 chunk。
3. **探索触发阶段**：观测/移动/任务访问坐标时调用 `ensure_chunk_generated`。
4. **区块种子阶段**：`chunk_seed = hash(world_seed, chunk_coord)` 派生随机源。
5. **碎片外形阶段**：在 chunk 内生成碎片骨架（位置 + 大小范围 + 最小间距）。
6. **块状离散阶段**：将碎片离散为若干长方体 block（最小单位 1cm）。
7. **化合物赋值阶段**：为每个 block 采样化合物组成（ppm）。
8. **物理量计算阶段**：计算 block 与碎片级体积/密度/质量。
9. **元素映射阶段**：由化合物组成推导元素统计分布。
10. **资源定量阶段**：写入碎片与 chunk 的 `total/remaining` 资源账本。
11. **提交与可见阶段**：写入 `WorldModel` 与 chunk 索引，状态切到 `Generated`。
12. **事件落账阶段**：追加 `ChunkGenerated`（含 seed 与校验摘要）。
13. **开采扣减阶段**：开采只减少 `remaining`，不重算 `total`。

## 5. Risks & Roadmap
- **CG1**：完成分块生成与元素/化合物池设计文档。
- **CG2**：实现 chunk 索引与按探索触发生成（最小可用闭环）。
- **CG3**：实现碎片块状物理模型（体积/密度/质量）与化合物组成。
- **CG4**：实现资源预算一次性写入与开采扣减守恒。
- **CG5**：场景接入起始 chunk 预生成 + 固定 20km×20km×10km 分块配置。
- **CG6**：实现持久化与回放契约（ChunkGenerated 事件/快照字段/版本迁移）。
- **CG7**：实现跨 chunk 边界一致性（邻块校验 + BoundaryReservation 保留/消费）。
- **CG8**：实现经济资源映射最小闭环（`RefineCompound -> electricity/hardware`）。
- **CG8.1**：冻结 `refine_quote` / `refine_preview` 玩家侧合同；本补充只约束精炼提交前可读性，不重平衡精炼公式。
- **CG9**：实现分块生成性能预算与确定性降级（fragments/blocks 三档上限）。
- **CG10**：补充回放一致性与性能回归测试（预算约束场景）。

### Technical Risks
- chunk 边界附近的最小间距约束需要考虑相邻 chunk，避免穿边重叠。
- block 粒度提升后，生成与序列化成本上升，需控制每碎片 block 数量上限。
- 化合物到元素映射若调整，会影响旧存档资源账本一致性。
- 质量公式采用整数近似时可能产生累计误差，需要统一舍入策略。
- ChunkGenerated 事件体积随 chunk 密度上升而增大，需结合快照频率控制日志膨胀。

## 6. Validation & Decision Record
- 追溯: GitHub task issue evidence 与 Git history 保留原实施过程；本文与 design 保持现行约束语义。
