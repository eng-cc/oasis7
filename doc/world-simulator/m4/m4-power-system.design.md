# M4 电力子系统设计文档

- 对应需求文档: `doc/world-simulator/m4/m4-power-system.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-power-system.project.md`

## 1. 设计定位
定义 M4 电力子系统设计，统一发电、储能、输配与工业耗能语义。

## 2. 设计结构
- 供给层：定义发电来源、容量与时序。
- 储配层：收敛储能、输电与能量分配规则。
- 消费层：让工厂、设施与运输对接统一耗能语义。
- 恢复可读性层：当 `BuyPower`、`harvest_radiation` 或等待发电用于脱离低电/停机风险时，展示恢复后可行动 runway、下一步动作可负担性和防停机收益。
- 售电可读性层：当 `SellPower` 用于短期变现时，展示售电后剩余 runway、下一步动作可负担性和产线停机风险。
- 治理观测层：暴露电力状态、瓶颈与调优入口。

## 3. 关键接口 / 入口
- 电力供给/储配模型
- 设施耗能接口
- 电力状态观测
- `power_survival_quote` / `energy_recovery_preview`: `agent_id`、`current_power_level`、`power_state_before/after`、`recovery_action`、`power_gain_estimate`、`price_or_time_cost`、`survival_runway_ticks`、`next_action_affordability`、`shutdown_avoidance_reason`、`recommended_power_action`
- `power_sale_quote` / `energy_liquidity_preview`: `agent_id`、`current_power_level`、`power_state_before`、`sale_amount`、`price_per_pu`、`expected_revenue`、`power_state_after_sale`、`remaining_runway_ticks`、`next_action_affordability_after_sale`、`production_interrupt_risk`、`recommended_sale_action`、`why_sale_is_safe_or_risky`
- 电力调优入口

## 4. 约束与边界
- 电力语义需与工业主路径一致。
- 能量守恒与瓶颈表达要清晰。
- `power_survival_quote_missing` 只表示玩家无法判断补电后能撑多久、能否执行下一步或避免停机；不得被解释为需要重平衡电力消耗、发电、价格、阈值或动作 ABI。
- `power_sale_quote_missing` 只表示玩家无法判断售电后的剩余 runway、下一步可执行性或产线停机风险；不得被解释为需要重平衡电价、电力消耗、发电效率、阈值、交易 ABI 或新增电力金融市场。
- 不在本专题扩展电力市场金融玩法。

## 5. 设计演进计划
- 先固定电力主模型。
- 再接工业耗能。
- 最后补观测与调优。
