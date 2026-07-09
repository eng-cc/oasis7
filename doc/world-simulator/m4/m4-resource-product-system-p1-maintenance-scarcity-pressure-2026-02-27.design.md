# M4 资源与产品系统 P1：维护压力与本地稀缺供给延迟（2026-02-27）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p1-maintenance-scarcity-pressure-2026-02-27.project.md`

## 1. 设计定位
定义 M4 资源与产品系统 P1 设计，聚焦维护压力与本地稀缺供给延迟对工业链路的影响。

## 2. 设计结构
- 维护压力层：引入设施维护、损耗与停机风险。
- 维护 runway 层：把当前耐久、每 tick 折旧、剩余 runway、停机阈值和推荐维护动作转成玩家提交前可读信息。
- 稀缺供给层：表达本地资源不足与补给延迟。
- 排程报价层：在确认排产前解释基础时长、本地稀缺额外 tick 和可避免动作。
- 压力传播层：让稀缺与维护影响产线、物流和策略。
- 观测验证层：输出压力指标并做场景验证。

## 3. 关键接口 / 入口
- 维护状态与损耗: `factory_id`、`durability_ppm`、`current_decay_per_tick`
- `factory_maintenance_status`: `maintenance_runway_ticks`、`downtime_threshold_ppm`、`pressure_class`、`recommended_maintenance_action`、`continue_production_risk`
- 本地供给延迟
- `schedule_quote`: `base_duration_ticks`、`local_shortage_delay_ticks`、`shortage_reason`、`recommended_pre_step`、`runway_before_ticks`、`runway_after_ticks`
- 压力传播信号
- 验证场景

## 4. 约束与边界
- 压力机制不能变成纯随机惩罚。
- 稀缺反馈需对玩家可感知。
- 若本地稀缺会增加排产 tick，确认前必须说明原因和避免方式；事后事件不能替代排程前 quote。
- 若高负载会消耗维护 runway，确认前必须说明还能撑多久、何时进入停机/critical，以及推荐先维护还是继续排产。
- `maintenance_runway_missing` 只描述玩家侧可读性缺口，不触发折旧公式、维护 sink 或停机阈值重平衡。
- 不在本专题扩展阶段引导与治理联动。

## 5. 设计演进计划
- 先补维护与稀缺状态。
- 再让压力传播到产线。
- 最后固化观测与验证。
