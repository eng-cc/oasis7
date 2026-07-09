# M4 电力子系统设计文档

- 对应设计文档: `doc/world-simulator/m4/m4-power-system.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-power-system.project.md`

审计轮次: 5

> 注：本分册为历史设计记录。自 2026-03-06 起 `PowerStorage`/`power_storages` 已在 simulator + viewer + scripts 全链路下线，现行口径以 `doc/world-simulator/kernel/power-storage-complete-removal-2026-03-06.prd.md` 为准。

## 1. Executive Summary

为硅基文明构建一个真实、可演化的电力系统：
- 电力是 Agent 生存的核心资源（无电力 = 停机/休眠）
- 电力供给有限且需要投资/维护
- 电力消耗与行动直接挂钩
- 电力市场可在文明发展中演化出来，形成价格信号，驱动协作与竞争

## 背景设定与开放性约束

- 初始状态：硅基个体未工业化，每个 Agent 自带基础发电与储能零件，可维持短期自给自足
- 初始世界不预置大型工业电力设施；外部发电/储能设施需要由 Agent 后续建造与扩展
- 初始阶段不存在“市场”概念；市场机制可能在文明发展过程中自然出现
- 框架保持开放：设施可被创建/升级/拆解/转移，并允许未来引入新的设施类型与制造规则
- **自由沙盒**：Agent 可自行设计新规则/新设施逻辑，编译为 WASM 并动态调用，这是整个系统的基础能力
- 技术上参考 **AgentOS**：WASM 模块 + Effect/Receipt + Capability/Policy 的受控扩展方式

## 2. User Experience & Functionality

### In Scope
- 电力生产（发电设施）
- 电力存储（储能设施/电池）
- 电力消耗（移动、计算、维护、空闲）
- 电力传输（位置间传输有损耗）
- 电力交易（Agent 间、Agent 与设施间）
- 停电处理（电力不足时的降级/休眠）
- 设施注册与所有权管理（作为后续建造系统的接入点）

### Out of Scope（本阶段）
- 复杂电网拓扑（输配电网络）
- 可再生能源波动（日照/风力变化）
- 储能化学特性（充放电效率曲线）
- 电力期货与复杂金融工具
- 工业化建造/制造流程（设施生产、产线、资源链）
- 市场机制的实现与演化驱动（仅预留扩展点）

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

- 核心配置：`PowerConfig`（空闲消耗、阈值、传输损耗、最大传输距离）。
- 关键实体：
  - `PowerPlant`
  - `PowerStorage`
  - `AgentPowerState`
- 关键动作：
  - `BuyPower` / `SellPower`
  - `DrawPower` / `StorePower`
- 关键事件：
  - `PowerGenerated`
  - `PowerConsumed`
  - `PowerTransferred`
  - `PowerStateChanged`
- 运行时入口：`WorldKernel::process_power_tick` 与相关电力状态查询接口。

## 核心概念

### 电力单位
- **PowerUnit (PU)**：电力的基本单位，1 PU = 1 单位电力
- Agent 的电力以 `i64` 存储，当前实现采用饱和扣减，不会出现负值；`level <= 0` 时进入 `Shutdown`

### 电力来源

发电与储能既可以是 Agent 内置部件（初始阶段），也可以是后续建造的外部设施。本阶段使用统一的设施模型表达，内置部件默认绑定 owner 与所在位置，不参与转移。

#### 1. 发电设施 (PowerPlant)
每个 Location 可以有一个或多个发电设施：
```rust
struct PowerPlant {
    id: FacilityId,
    location_id: LocationId,
    owner: ResourceOwner,
    // 发电能力
    capacity_per_tick: i64,    // 每 tick 最大发电量
    current_output: i64,        // 当前发电量
    // 运营成本
    fuel_cost_per_pu: i64,      // 每 PU 燃料成本（Data 资源）
    maintenance_cost: i64,      // 每 tick 维护成本
    // 状态
    status: PlantStatus,        // Running / Offline / Maintenance
    efficiency: f64,            // 效率 0.0-1.0
    degradation: f64,           // 老化程度 0.0-1.0
}
```
- 初始阶段：可将 Agent 的内建发电部件表示为绑定该 Agent 的 PowerPlant（容量较小、不可转移）
- 工业化后：可建造独立设施，允许转移/共享

#### 2. 储能设施 (PowerStorage)
```rust
struct PowerStorage {
    id: FacilityId,
    location_id: LocationId,
    owner: ResourceOwner,
    capacity: i64,              // 最大存储量
    current_level: i64,         // 当前存储量
    charge_efficiency: f64,     // 充电效率 (0.8-0.95)
    discharge_efficiency: f64,  // 放电效率 (0.9-0.98)
    max_charge_rate: i64,       // 每 tick 最大充电速率
    max_discharge_rate: i64,    // 每 tick 最大放电速率
}
```
- 初始阶段：可将 Agent 的内建储能部件表示为绑定该 Agent 的 PowerStorage（容量较小、不可转移）
- 工业化后：可建造独立设施，允许转移/共享

### 电力消耗

#### 消耗类型
1. **空闲消耗 (Idle)**：Agent 存活的基础消耗
   - 默认：1 PU/tick
2. **移动消耗 (Move)**：已实现，按距离计费
   - 参考口径（`time_step_s=10`、`power_unit_j=1000`、`move_cost_per_km_electricity=1`）下为 `ceil(distance_km) PU`
   - 实际运行时通过 `WorldConfig::movement_cost(distance_cm)` 按 `time_step_s` 与 `power_unit_j` 自动缩放
3. **计算消耗 (Compute)**：执行决策/推理的消耗
   - 默认：每次决策 1 PU
4. **维护消耗 (Maintenance)**：硬件老化的持续消耗
   - 基于硬件健康度，健康度越低消耗越高

#### 配置参数
```rust
struct PowerConfig {
    idle_cost_per_tick: i64,        // 空闲消耗，默认 1
    decision_cost: i64,             // 决策消耗，默认 1
    default_power_capacity: i64,    // Agent 默认容量，默认 100
    default_power_level: i64,       // Agent 初始电量，默认 100
    low_power_threshold_pct: i64,   // 低电阈值百分比，默认 20
    critical_threshold_pct: i64,    // 临界电阈值百分比，默认 5
    transfer_loss_per_km_bps: i64,  // 传输损耗（每公里，bps，默认 10=0.1%）
    transfer_max_distance_km: i64,  // 跨 Location 传输最大距离，默认 10_000
}
```

#### 与物理配置的联动（已实现）

- 移动电耗最终由 `WorldConfig::movement_cost` 计算，除 `PowerConfig` 外还受 `PhysicsConfig.time_step_s` 与 `PhysicsConfig.power_unit_j` 影响
- `process_power_tick` 在处理空闲耗电的同时执行热量散逸（与 `PhysicsConfig.thermal_capacity / thermal_dissipation / thermal_dissipation_gradient_bps` 联动）

### 电力传输

#### 传输规则
- 同 Location 内传输：无损耗
- 跨 Location 传输：仅 Location ↔ Location 允许，Agent 仍需共址交易
  - 传输损耗 = 距离(km) × 损耗系数（默认 0.1% per km，按 bps 计算）
  - 长距离传输可能需要中继站

### 电力不足处理

#### 电力状态
```rust
enum AgentPowerState {
    Normal,           // 电力充足
    LowPower,         // 电力不足（< 20%），触发节能模式
    Critical,         // 电力临界（< 5%），只能执行关键操作
    Shutdown,         // 停机/休眠，需外部充电才能恢复
}
```

#### 降级策略
1. **LowPower**：当前主要作为状态信号，供策略层（规则/调度）降级决策
2. **Critical**：当前主要作为状态信号，默认动作层尚未单独强制限制
3. **Shutdown**：已在动作层强制拒绝关键动作（如 `MoveAgent`），并需外部充电恢复

### 电力交易

#### 交易动作
```rust
enum PowerAction {
    // 购买电力（从设施或其他 Agent）
    BuyPower {
        buyer: ResourceOwner,
        seller: ResourceOwner,
        amount: i64,
        price_per_pu: i64,
    },
    // 出售电力
    SellPower {
        seller: ResourceOwner,
        buyer: ResourceOwner,
        amount: i64,
        price_per_pu: i64,
    },
    // 从储能设施放电到所在 Location
    DrawPower {
        storage_id: FacilityId,
        amount: i64,
    },
    // 从所在 Location 向储能设施充电
    StorePower {
        storage_id: FacilityId,
        amount: i64,
    },
}
```

#### 玩家侧电力恢复预览

当 Agent 处于 `LowPower`、`Critical`、`Shutdown` 邻近状态，或策略层建议通过 `BuyPower`、`harvest_radiation`、等待发电恢复电力时，玩家侧必须能读取 `power_survival_quote` / `energy_recovery_preview`：

- `agent_id`
- `current_power_level`
- `power_state_before`
- `recovery_action`: `buy_power` / `harvest_radiation` / `wait_for_generation`
- `power_gain_estimate`
- `price_or_time_cost`
- `power_state_after`
- `survival_runway_ticks`
- `next_action_affordability`
- `shutdown_avoidance_reason`
- `recommended_power_action`

Edge case: 若系统推荐补电、采集辐射或等待发电，但玩家看不到恢复后的 runway、状态变化、下一步动作可负担性或防停机原因，标记为 `power_survival_quote_missing`。该缺口属于电力恢复可读性问题，不改变电力消耗、发电、价格、阈值、runtime ABI 或 `BuyPower` / `SellPower` / `harvest_radiation` 动作语义。

验收口径：玩家在执行买电、采集辐射或等待发电前，应能看懂“补多少电、花什么成本、能撑多久、是否足够完成当前下一步、是否能避免或解除停机”。

#### 玩家侧售电机会成本预览

当玩家准备通过 `SellPower` 将电力变现时，玩家侧必须能读取 `power_sale_quote` / `energy_liquidity_preview`，用于判断短期现金流是否会牺牲当前产线、移动、采矿、排程或维护窗口的稳定性：

- `agent_id`
- `current_power_level`
- `power_state_before`
- `sale_amount`
- `price_per_pu`
- `expected_revenue`
- `power_state_after_sale`
- `remaining_runway_ticks`
- `next_action_affordability_after_sale`
- `production_interrupt_risk`
- `recommended_sale_action`: `sell_full` / `sell_partial` / `defer_sale` / `buy_or_wait_first`
- `why_sale_is_safe_or_risky`

Edge case: 若玩家准备 `SellPower`，但看不到售电后剩余电力、可行动 runway、下一步动作是否仍可负担或产线停机风险，标记为 `power_sale_quote_missing`。该缺口属于售电机会成本可读性问题，不改变电价、电力消耗、发电效率、电力阈值、runtime ABI、Location 电力池或 `PowerStorage` 语义。

验收口径：玩家在执行售电前，应能看懂“卖多少电、赚多少、卖完还能撑多久、是否会影响当前产线/下一动作、系统建议全卖/少卖/暂缓的理由”。

## 接口设计

### WorldKernel 扩展
```rust
impl WorldKernel {
    // 电力系统 tick 处理
    fn process_power_tick(&mut self) -> Vec<WorldEvent>;
    
    // 查询 Agent 电力状态
    fn agent_power_state(&self, agent_id: &AgentId) -> Option<AgentPowerState>;

    // 判断 Agent 是否停机
    fn is_agent_shutdown(&self, agent_id: &AgentId) -> bool;

    // 查询所有停机 Agent
    fn shutdown_agents(&self) -> Vec<AgentId>;
    
    // 设施查询目前通过 WorldModel.power_plants / power_storages 访问
}
```

### 新增事件类型
```rust
enum PowerEvent {
    PowerPlantRegistered { plant: PowerPlant },
    PowerStorageRegistered { storage: PowerStorage },
    PowerGenerated { plant_id, location_id, amount },
    PowerStored { storage_id, location_id, input, stored },
    PowerDischarged { storage_id, location_id, output, drawn },
    PowerConsumed { agent_id, amount, reason: ConsumeReason, remaining },
    PowerStateChanged { agent_id, from: AgentPowerState, to: AgentPowerState, trigger_level },
    PowerTransferred { from, to, amount, loss, price_per_pu },
    PowerCharged { agent_id, amount, new_level },
}

enum ConsumeReason {
    Idle,
    Move { distance_cm: i64 },
    Decision,
    Maintenance,
    Custom { name: String },
}
```

### 扩展点（预留）

- 目标：支持文明演化出来的新机制（例如市场规则、设施行为、定价逻辑）
- 形式：Agent 自行设计的模块可被编译为 WASM 并动态调用，以扩展电力系统逻辑
- 约束：主系统负责**沙箱隔离、能力/政策约束、收据审计**与版本兼容性校验
- 状态：本阶段仅声明扩展点，不落地具体实现

## 实现计划

### Phase 1：基础电力消耗
1. 扩展 Agent 结构，添加 `power` 字段（`AgentPowerStatus`）
2. 实现空闲消耗：每 tick 扣除电力
3. 实现电力不足检测与状态切换
4. 实现 Shutdown 状态的调度器处理

### Phase 2：发电与储能
1. 实现 PowerPlant 结构与基础发电逻辑
2. 实现 PowerStorage 结构与充放电逻辑
3. 将设施绑定到 Location

### Phase 3：电力传输与交易
1. 实现电力传输动作与损耗计算
2. 实现电力交易动作
3. 添加相应的事件类型

### Phase 4：高级功能
1. 设施老化与维护
2. 电价波动（供需平衡，纯供需模型）
3. 停电恢复流程

## M4.4 供需电价模型（2026-02-26 冻结口径）

本轮冻结为“纯供需定价”，不引入峰谷时段调节项。

- `base_price = clamp(market_base_price_per_pu, market_price_min_per_pu, market_price_max_per_pu)`
- `demand_pressure_bps = clamp((requested * 10000 / max(seller_available_before, 1)) - 10000, 0, market_supply_demand_price_max_bps)`
- `quote_price = clamp(base_price + base_price * demand_pressure_bps / 10000, market_price_min_per_pu, market_price_max_per_pu)`

说明：

- 当 `requested <= seller_available_before`，压力项接近 `0`，价格接近 `base_price`。
- 当请求量显著高于可供给量时，价格按压力项上浮，但受 `market_supply_demand_price_max_bps` 与全局 `min/max` 双重约束。
- 显式报价偏离仍由 `market_price_band_bps` 控制（用于防止离谱报价），但不参与时段调价。

## 5. Risks & Roadmap

- M4.1：基础电力消耗与状态管理（空闲消耗、低电量休眠）
- M4.2：发电与储能设施
- M4.3：电力传输与交易
- M4.4：电价与市场机制

### Technical Risks

1. **复杂度膨胀**：电力系统容易变得过于复杂，需要阶段性控制范围
2. **平衡性**：电力消耗与产出的平衡需要调优，避免资源锁死
3. **性能**：大量 Agent 的电力计算可能成为瓶颈，需要批量处理

## 测试策略

1. 单元测试：每个组件独立测试
2. 集成测试：电力系统与 WorldKernel 的交互
3. 场景测试：
   - 单 Agent 电力耗尽与恢复
   - 多 Agent 竞争有限电力
   - 发电设施故障与恢复

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
