# LLM 工业采矿闭环与调试补给工具（设计文档）设计

- 对应需求文档: `doc/world-simulator/llm/llm-industrial-mining-debug-tools.prd.md`
- 对应项目管理文档: `doc/world-simulator/llm/llm-industrial-mining-debug-tools.project.md`

## 1. 设计定位
定义 LLM 工业采矿闭环与调试补给工具设计，为工业/采矿策略验证提供可观测、可注入的调试辅助。

## 2. 设计结构
- 工业采矿观测层：暴露关键资源、产线与采矿状态。
- 精炼收益预览层：解释 `refine_compound` 的 compound 投入、电力成本、硬件产出、精炼后电力和当前工业目标缺口变化。
- 调试补给层：提供受控的补给、重置或注入能力。
- 问题定位层：定位策略失败、资源卡死与链路断点。
- 回归支撑层：为工业采矿场景提供复现与验证工具。

## 3. 关键接口 / 入口
- 工业采矿调试入口
- `refine_quote` / `refine_preview`: `compound_mass_g`、`electricity_cost`、`hardware_output`、`electricity_after`、`hardware_shortfall_before/after`、`first_goal_relevance`、`recommended_refine_amount`、`refine_value_class`
- 补给/注入控制
- 状态观测面板
- 调试回归脚本

## 4. 约束与边界
- 调试能力必须受控，不能污染正式主路径。
- 补给工具需可审计、可撤销。
- 精炼收益预览不改变 `refine_compound` 动作、公式或 debug tool 暴露边界；它只补玩家/策略可读性。
- `refine_quote_missing` 不得被解释为需要新增完整加工链或 viewer 重构。
- 不在本专题扩展通用作弊工具链。

## 5. 设计演进计划
- 先补观测与问题定位。
- 再加受控补给工具。
- 最后沉淀调试回归路径。
