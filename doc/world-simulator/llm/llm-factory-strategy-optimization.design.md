# LLM 工厂闭环策略稳定性优化（llm_bootstrap）设计

- 对应需求文档: `doc/world-simulator/llm/llm-factory-strategy-optimization.prd.md`
- 专题入口与权威边界: `doc/world-simulator/llm/README.md`

## 1. 设计定位
定义 llm_bootstrap 工厂闭环策略稳定性优化设计，让工厂策略生成、评估与纠偏形成稳定闭环。

## 2. 设计结构
- 策略生成层：基于工厂状态生成可执行策略建议。
- 精炼恢复解释层：当硬件不足恢复为 `refine_compound` 时，解释电力成本、硬件产出和首个工业目标缺口变化。
- 电力恢复解释层：当电力不足恢复为 `harvest_radiation`、`BuyPower` 或等待发电时，解释恢复后 runway、状态变化、下一步动作可负担性和防停机收益。
- 闭环评估层：比较策略执行结果与目标偏差。
- 纠偏优化层：对低效策略进行自动或半自动调整。
- 稳定性回归层：围绕关键工业场景验证策略波动与收敛。

## 3. 关键接口 / 入口
- 工厂状态输入
- 策略建议输出
- `refine_quote` 摘要输出：`electricity_cost`、`hardware_output`、`electricity_after`、`hardware_shortfall_before/after`、`first_goal_relevance`、`recommended_refine_amount`、`refine_value_class`
- `power_survival_quote` / `energy_recovery_preview` 摘要输出：`current_power_level`、`power_state_before/after`、`recovery_action`、`power_gain_estimate`、`price_or_time_cost`、`survival_runway_ticks`、`next_action_affordability`、`shutdown_avoidance_reason`、`recommended_power_action`
- 纠偏与评估接口
- 工业闭环回归

## 4. 约束与边界
- 优化不能破坏基础工业主路径。
- 策略调整需要可解释、可追踪。
- `refine_compound` 恢复建议不得只展示动作改写结果；缺少净收益/机会成本时标记 `refine_quote_missing`。
- 精炼预览不改变恢复规则强度、精炼公式、runtime ABI 或 viewer 实现。
- 电力恢复建议不得只展示“去补电/采集辐射”的动作改写结果；缺少恢复后 runway、下一步可执行性或防停机收益时标记 `power_survival_quote_missing`。
- 电力恢复预览不改变电力消耗、发电、价格、阈值、runtime ABI、viewer 实现或 `harvest_radiation` / `BuyPower` 动作语义。
- `power_survival_quote_missing` 是 future routed diagnostic；若未来进入 agent observation 或公共 `last_action`，必须同步升级 dual-mode provider contract / schema version，当前设计只约束 strategy hint 或玩家可读预览的可读性。
- 不在本专题引入黑盒自动调参系统。

## 5. 设计演进计划
- 先建立策略输入输出。
- 再接闭环评估与纠偏。
- 最后执行稳定性回归。
