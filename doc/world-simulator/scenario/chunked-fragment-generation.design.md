# oasis7 Simulator：分块世界生成与碎片元素/化合物池（设计文档）设计

- 对应需求文档: `doc/world-simulator/scenario/chunked-fragment-generation.prd.md`
- 对应项目管理文档: `doc/world-simulator/scenario/chunked-fragment-generation.project.md`

## 1. 设计定位
定义分块世界生成与碎片元素/化合物池设计，让碎片世界生成具备分块组织、资源池配置与可复现行为。

## 2. 设计结构
- 分块生成层：按 chunk 组织碎片世界的生成范围与密度。
- 资源池层：为元素/化合物配置碎片生成池与分布规则。
- 精炼预览层：把 compound -> hardware 的电力成本、产出和当前第一工业目标缺口转成玩家提交前可读 quote。
- 资源恢复预览层：把 `FragmentsReplenished` 的补种时间、预计补种量和当前目标关联转成玩家等待/转移/替代路线判断。
- 初始化接线层：把分块生成结果写入场景和世界初始化路径。
- 验证回归层：校验 chunk 布局、资源分布与生成稳定性。

## 3. 关键接口 / 入口
- chunk 生成配置
- 碎片资源池定义
- `refine_quote` / `refine_preview`: `compound_mass_g`、`electricity_cost`、`hardware_output`、`electricity_after`、`hardware_shortfall_before`、`hardware_shortfall_after`、`first_goal_relevance`、`recommended_refine_amount`、`refine_value_class`
- `resource_replenishment_quote` / `fragment_refill_preview`: `chunk_coord`、`target_frag_id`、`current_frag_remaining_summary`、`chunk_remaining_summary`、`next_replenish_tick`、`ticks_until_replenish`、`estimated_replenished_frag_count`、`estimated_replenished_resource_hint`、`next_industrial_goal_relevance`、`wait_cost_summary`、`recommended_resource_action`
- 世界初始化生成入口
- 生成回归用例

## 4. 约束与边界
- 同一 seed 的生成结果必须稳定。
- 资源池变更不得破坏既有场景可加载性。
- 精炼预览只解释现有 `RefineCompound` 公式和当前目标缺口，不改变产率、电力成本、动作 ABI 或回放语义。
- `refine_quote_missing` 是玩家侧机会成本可读性缺口，不代表要新增完整加工链或市场系统。
- 补种预览只解释现有 `FragmentsReplenished` 节奏和当前目标关联，不改变补种周期、补种比例、分布权重、资源守恒或 replay 语义。
- `resource_replenishment_quote_missing` 是玩家侧等待/转移/替代路线可读性缺口，不代表要新增动态经济调参或完整资源市场。
- 不在本专题扩展复杂程序化地形系统。

## 5. 设计演进计划
- 先固化 chunk 生成结构。
- 再补资源池与初始化接线。
- 最后执行稳定性回归。
