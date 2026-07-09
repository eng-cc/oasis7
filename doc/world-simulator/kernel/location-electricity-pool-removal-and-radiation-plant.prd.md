# oasis7 Simulator：Location 电力资源池下线与 Agent 辐射电厂建造

- 对应设计文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.design.md`
- 对应项目管理文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.project.md`

审计轮次: 5

## 1. Executive Summary

- 删除 Location 侧 `electricity` 资源池语义，统一电力资源归属到 Agent（或明确 owner 的可持有实体）侧。
- 下线依赖 Location 电力池的供电动作链路，避免“地点当电池”与 Agent 供能模型并存。
- 在 Agent 可建造物中新增“辐射采集发电电厂”，使 Agent 可通过建造获得持续发电能力。

## 2. User Experience & Functionality

### In Scope
- simulator 初始化阶段清洗 Location 资源中的 `electricity`，不再保留 Location 电力库存。
- simulator 动作层下线/拒绝依赖 Location 电力池的供电路径（`DrawPower` / `StorePower` / Location 参与的 `BuyPower`/`SellPower`）。
- 新增 Agent 可建造电厂类型：`factory.power.radiation.mk1`。
- `BuildFactory` 在建造 `factory.power.radiation.mk1` 时同步注册一个辐射发电设施（`PowerPlant`）。
- 发电入账改为 owner 侧（优先 Agent owner），不再写入 Location 电力库存。
- 更新 LLM 提示中 `factory_kind` 支持集合。
- 补齐 simulator 相关单元测试与场景断言。

### Out of Scope
- runtime 侧 WASM M1 电力模块语义重构。
- viewer 的 UI 文案大改（仅做必要兼容）。
- 完整移除 `PowerStorage` 数据结构（本次仅下线依赖 Location 电力池的充放电动作链路）。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications

- 新增可建造类型：
  - `factory.power.radiation.mk1`
- 行为约束：
  - Location 不再持有有效 `ResourceKind::Electricity` 库存（初始化时清洗；运行时不接受 Location 电力入账）。
  - `DrawPower` / `StorePower` 返回拒绝（供电路径下线）。
  - `BuyPower` / `SellPower` 仅允许 Agent owner 间电力转移；若包含 Location owner 则拒绝。
- 建造行为：
  - `Action::BuildFactory` 当 `factory_kind == "factory.power.radiation.mk1"` 时：
    - 常规工厂建造事件照常产生；
    - 同步注册同 ID 的 `PowerPlant`（owner 与 location 继承工厂）。
- 发电行为：
  - `process_power_generation_tick` 的发电入账从“Location 电力池”调整为“power plant owner 资源库存”。
- 玩家侧恢复预览：
  - `factory.power.radiation.mk1`、等待发电或 `harvest_radiation` 被推荐为电力恢复路径时，必须能派生 `energy_recovery_preview` / `power_survival_quote`。
  - 最小字段：`agent_id`、`current_power_level`、`power_state_before`、`recovery_action`、`power_gain_estimate`、`price_or_time_cost`、`power_state_after`、`survival_runway_ticks`、`next_action_affordability`、`shutdown_avoidance_reason`、`recommended_power_action`。
  - Edge case: 若 owner 侧已能入账发电或采集电力，但玩家看不到恢复后的 runway、状态变化和下一步可执行性，标记为 `power_survival_quote_missing`；该缺口不恢复 Location 电力池，不重新引入 `PowerStorage` 路径，也不改变 `factory.power.radiation.mk1` 的建造/发电语义。

## 5. Risks & Roadmap

- **R1 文档与任务拆解**：输出设计文档与项目管理文档。
- **R2 Location 电力池下线**：完成初始化清洗与供电动作链路下线。
- **R3 Agent 辐射电厂建造**：新增 `factory.power.radiation.mk1` 及建造联动注册。
- **R4 回归与文档回写**：测试通过、项目文档状态更新、devlog 记录与提交。

### Technical Risks

- 历史测试和场景若依赖 Location 初始电力库存会出现断言回归，需要同步修订。
- `PowerStorage` 路径下线后，`power_bootstrap` 等设施专项场景需调整验收口径。
- 回放（replay）中旧事件序列若包含 Location 电力入账，可能需要兼容处理或明确不兼容边界。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
