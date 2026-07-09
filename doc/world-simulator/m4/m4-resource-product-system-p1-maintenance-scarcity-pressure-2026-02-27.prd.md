# M4 资源与产品系统 P1：维护压力与本地稀缺供给延迟（2026-02-27）

- 对应设计文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 在 P0 基础上强化“维护成本压力”，让高负载产线更快折旧，形成持续维护决策。
- 接入“本地稀缺供给延迟”语义：当站点库存不足被迫回退到 world 账本时，配方完工时间增加。
- 高负载维护压力必须给出玩家可读的 runway / 停机临界点，让玩家能判断继续排产、先维护或降载之间的取舍。
- 保持兼容：不改动作 ABI，不新增必填字段，不破坏旧快照/旧事件回放。

## 2. User Experience & Functionality

### In Scope
- 工厂折旧接入负载系数（按 `active_jobs / recipe_slots` 放大衰减）。
- 工厂存在非零折旧或非零维护消耗时，玩家侧应能读取 `factory_maintenance_status`，包含剩余维护 runway、停机阈值、压力档位和建议动作。
- 配方排产在“本地库存不足且存在关键中间件消耗”时增加供给延迟 tick。
- 当排产会触发本地稀缺供给延迟时，玩家在确认前必须看到 `schedule_quote`，而不是只在执行后从事件里发现额外等待。
- 补齐 `test_tier_required`：
  - 负载下折旧快于空载。
  - world fallback 触发供给延迟且按期完成。

### Out of Scope
- 不改 `oasis7_wasm_abi::RecipeExecutionPlan` 结构。
- 不引入市场撮合或新治理税种。
- 不改 viewer 结构，仅通过现有事件行为可观测。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

### 1) 负载折旧
- 位置：`runtime/world/economy.rs::process_factory_depreciation`。
- 新逻辑：
  - 基础衰减：`maintenance_per_tick * FACTORY_DEPRECIATION_PPM_PER_MAINTENANCE_UNIT`。
  - 负载放大：`load_factor_bps = 10000 + floor(active_jobs * 10000 / recipe_slots)`，上限 20000。
  - 最终衰减：`base_decay * load_factor_bps / 10000`。
- 玩家侧维护状态合同 `factory_maintenance_status`：
  - `factory_id`: 当前工厂或产线单元。
  - `durability_ppm`: 当前耐久。
  - `current_decay_per_tick`: 当前负载下预计每 tick 折旧。
  - `maintenance_runway_ticks`: 以当前折旧速度到达停机/critical 阈值前的预计 tick 数。
  - `downtime_threshold_ppm`: 低于该阈值会进入停机、critical 或需要强维护处理。
  - `pressure_class`: `safe / watch / high / critical`。
  - `recommended_maintenance_action`: 继续排产、先维护、降载或补充维护材料等最小建议。
  - `continue_production_risk`: 若继续按当前负载生产，玩家会承担的停机/维护窗口风险。
- Edge case: 若工厂存在非零折旧或本次排程存在非零 `maintenance_sink`，但玩家看不到 `maintenance_runway_ticks` 或 `downtime_threshold_ppm`，标记为 `maintenance_runway_missing`；该缺口属于维护压力可读性问题，不改变折旧公式或阈值。

### 2) 本地稀缺供给延迟
- 位置：`runtime/world/event_processing/action_to_event_economy.rs`。
- 触发条件：
  - `preferred_consume_ledger` 不是 `world`。
  - `consume_ledger` 回退为 `world`。
  - 本次配方命中 `bottleneck_tags`（P0 已接线）。
- 延迟规则：
  - 计算本地缺口占比 `deficit / requested`。
  - 缺口 > 0 且 < 70%：`+1 tick`。
  - 缺口 >= 70%：`+2 ticks`。

### 3) 排程报价可读性
- 当 `ScheduleRecipe` 预计触发 world fallback + bottleneck 延迟时，确认排产前应生成玩家可读的 `schedule_quote`：
  - `base_duration_ticks`: 原始配方时长。
  - `local_shortage_delay_ticks`: 本地缺口导致的额外 tick。
  - `shortage_reason`: 哪类关键中间件/本地库存不足触发延迟。
  - `recommended_pre_step`: 可避免或降低延迟的预备动作，例如先补本地库存、转移材料或降低负载。
- 当 `ScheduleRecipe` 会叠加非零维护消耗或提高折旧压力时，`schedule_quote` 还应展示维护 runway 变化：
  - `runway_before_ticks`: 提交前预计维护 runway。
  - `runway_after_ticks`: 本次排程后预计维护 runway。
  - `downtime_threshold_ppm`: 当前停机/critical 阈值。
  - `maintenance_pressure_delta`: 本次排程导致的压力档位变化，若无变化则显式为 `none`。
  - `recommended_maintenance_action`: 继续排产、先维护、降载或补维护材料。
- 若实际执行会增加供给延迟但 quote 缺失，记为 `schedule_quote_missing`；该缺口属于玩家侧后果可读性问题，不改变本专题的延迟阈值。
- 若实际执行会消耗维护 runway 但 quote 缺少前后 runway 或停机阈值，记为 `maintenance_runway_missing`；执行后日志不能替代排程前维护取舍。

## 5. Risks & Roadmap
- P1-T0：设计文档与项目文档建档。
- P1-T1：代码接线（负载折旧 + 供给延迟）。
- P1-T2：补齐 required 单测并回归。
- P1-T3：回写项目状态与 devlog。

### Technical Risks
- 行为漂移：配方完工时序变化可能影响既有事件顺序断言。
- 参数风险：延迟阈值过高可能导致产线停滞体感。
- 叠加风险：负载折旧与维护成本共同作用可能提高新手失败率。
- 可读性风险：若 `+1/+2 ticks` 只在事后事件中出现，玩家会把稀缺延迟误读为随机等待或系统惩罚。
- 维护可读性风险：若高负载折旧只显示压力档位，玩家会知道“更危险”但不知道还能撑多久，容易把停机理解成事后惩罚。

缓解：
- 阈值采用保守值（1~2 tick），先做可观测，再调参。
- 仅在 world fallback + bottleneck 同时满足时触发延迟。
- quote 先解释本次基础时长、额外延迟和可选预备动作，再允许玩家确认排产。
- 维护 runway 先冻结为玩家侧 quote / status 合同，不在本专题重平衡折旧、维护 sink 或停机阈值。
- 先补 required 测试再跑回归。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
- DEC-M4-P1-001: 本地稀缺供给延迟应先作为排程前 quote 呈现，再作为执行后事件落账；玩家侧可读性优先于调高/调低延迟参数。
- DEC-M4-P1-002: 高负载维护压力必须先给出 `maintenance_runway_ticks` 与 `downtime_threshold_ppm`，让玩家在继续排产、先维护和降载之间做提交前判断；本决策不改变折旧公式、维护 sink 参数或 runtime ABI。
