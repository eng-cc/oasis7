# Location 电力池下线与 Agent 辐射电厂设计

- 对应需求文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.prd.md`
- 对应项目管理文档: `doc/world-simulator/kernel/location-electricity-pool-removal-and-radiation-plant.project.md`

## 1. 设计定位
定义电力资源从 `Location` 资源池迁移到 owner 侧的统一归属模型，并为 Agent 增加可持续发电的辐射采集电厂建造闭环。

## 2. 设计结构
- 资源归属层：Location 不再保存有效 `electricity` 库存，电力统一记在 Agent 或明确 owner 的库存上。
- 动作约束层：`DrawPower`、`StorePower` 直接拒绝，`BuyPower`、`SellPower` 仅允许 owner 间转移。
- 建造联动层：`BuildFactory(factory.power.radiation.mk1)` 同步注册 `PowerPlant`，继承工厂 owner 与 location。
- 发电入账层：`process_power_generation_tick` 将产电计入 owner 库存，不再写回 Location。
- 电力恢复可读性层：辐射发电、`harvest_radiation` 或等待发电作为恢复路径时，通过 `energy_recovery_preview` 解释补电收益、runway、下一步动作可负担性和防停机原因。
- 提示同步层：LLM `factory_kind` 可选集合与测试场景保持一致更新。

## 3. 关键接口 / 入口
- `Action::BuildFactory`
- `process_power_generation_tick`
- `energy_recovery_preview` / `power_survival_quote`: `recovery_action`、`power_gain_estimate`、`price_or_time_cost`、`power_state_before/after`、`survival_runway_ticks`、`next_action_affordability`、`shutdown_avoidance_reason`、`recommended_power_action`
- `crates/oasis7/src/simulator/kernel/actions.rs`
- `crates/oasis7/src/simulator/kernel/power.rs`
- `crates/oasis7/src/simulator/init.rs`
- `crates/oasis7/src/simulator/llm_agent/prompt_assembly.rs`

## 4. 约束与边界
- 初始化阶段必须清洗历史 Location 电力库存，避免旧状态混入新模型。
- 运行时任何 Location 电力入账路径都应拒绝或旁路，不能留下双轨语义。
- 辐射电厂只补齐建造与发电闭环，不在本阶段扩展完整电力模块重构。
- 电力恢复预览只解释 owner 侧补电后的玩家取舍，不恢复 Location 电力池，不重新启用 `DrawPower` / `StorePower` / `PowerStorage` 玩法，也不新增完整电力市场 UI。
- 旧回放若依赖 Location 电力池，需要通过兼容判定或明确拒绝，不静默吞错。

## 5. 设计演进计划
- 先完成 Location 电力池下线与动作拒绝。
- 再补齐 `factory.power.radiation.mk1` 的建造注册与 owner 入账。
- 最后同步 LLM 提示、场景断言与回归测试，收口文档与验收记录。
