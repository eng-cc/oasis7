# M4 资源与产品系统 P2：阶段引导与市场-治理联动观测（2026-02-27）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p2-stage-guidance-market-governance-linkage-2026-02-27.project.md`

## 1. 设计定位
定义 M4 资源与产品系统 P2 设计，把阶段引导与市场-治理联动纳入工业经济反馈。

## 2. 设计结构
- 阶段引导层：根据发展阶段调整目标、提示与约束。
- 市场联动层：把供需、价格与阶段目标联动。
- 市场决策预览层：把本地库存、world/import 来源、治理税和运输损耗转成玩家可读的采购/调运/延后/治理取舍。
- 治理联动层：让治理动作影响资源与产品系统规则。
- 反馈评估层：评估联动是否改善产业链体验。

## 3. 关键接口 / 入口
- 阶段状态
- 市场联动入口
- `market_quote_decision_preview`: `material_kind`、`requested_amount`、`recommended_source`、`local_vs_world_delta_ppm`、`tax_cost_ppm`、`transit_loss_cost_ppm`、`shortfall_after_recommended_source`、`cost_pressure_class`、`why_this_source`、`next_cost_reduction_action`
- 治理联动接口
- 反馈评估指标

## 4. 约束与边界
- 联动规则需避免过度复杂化。
- 阶段引导必须服务可玩性。
- `market_quote_decision_missing` 表示玩家侧市场信号可读性不足，不代表要新增订单簿、撮合交易动作或完整市场 UI。
- 市场决策预览只解释已有成本指数、税费、运输损耗和本地缺口，不调整价格公式、税率或材料分布。
- 不在本专题扩展档案化链路。

## 5. 设计演进计划
- 先补阶段引导。
- 再接市场和治理联动。
- 最后评估反馈效果。
