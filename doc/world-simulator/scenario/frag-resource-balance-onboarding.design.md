# oasis7 Simulator：Frag 资源平衡与新手友好生成（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/frag-resource-balance-onboarding.prd.md`
- 对应项目管理文档: `doc/world-simulator/scenario/frag-resource-balance-onboarding.project.md`

## 1. 设计定位
定义 Frag 资源平衡与新手友好生成设计，使碎片场景在早期体验上兼顾资源平衡与引导性。

## 2. 设计结构
- 资源基线层：定义碎片初始资源、稀缺度与分布。
- 新手引导层：确保开局关键资源与行动路径可达。
- 材质预期层：把中心/边缘材质偏置转成首局推荐 frag 的粗粒度玩家提示。
- 补种预览层：把运行期 `FragmentsReplenished` 节奏转成等待、转移或替代材料路线的玩家可读取舍。
- 平衡调节层：对过度稀缺或过度富集场景进行修正。
- 验证回归层：通过 onboarding 场景验证首轮体验稳定。

## 3. 关键接口 / 入口
- 碎片资源基线配置
- 新手友好生成规则
- starter frag material hint: 材质预期、第一工业目标关联、距离/可达性理由
- `resource_replenishment_quote` / `fragment_refill_preview`: `chunk_coord`、`target_frag_id`、`current_frag_remaining_summary`、`chunk_remaining_summary`、`next_replenish_tick`、`ticks_until_replenish`、`estimated_replenished_frag_count`、`estimated_replenished_resource_hint`、`next_industrial_goal_relevance`、`wait_cost_summary`、`recommended_resource_action`
- 平衡调节参数
- onboarding 回归场景

## 4. 约束与边界
- 开局资源应支持基本体验闭环。
- 平衡优化不能消除场景差异性。
- 玩家提示只能表达 coarse material expectation，不得把分布权重暴露成确定收益或完整矿物百科。
- `resource_replenishment_quote_missing` 只表示玩家无法判断等补种、换 frag/chunk 或切换材料路线的取舍；不得被解释为要重平衡补种周期、补种比例、分布权重或资源经济。
- 不在本专题扩展完整经济系统重构。

## 5. 设计演进计划
- 先固化资源基线。
- 再补新手友好约束。
- 最后执行可玩性回归。
