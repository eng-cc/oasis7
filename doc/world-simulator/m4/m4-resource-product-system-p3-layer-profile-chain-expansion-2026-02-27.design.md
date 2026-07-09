# M4 资源与产品系统 P3：分层档案化与链路扩展实现（2026-02-27）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p3-layer-profile-chain-expansion-2026-02-27.project.md`

## 1. 设计定位
定义 M4 资源与产品系统 P3 设计，面向分层档案化与链路扩展，让复杂工业链路可持续演进。

## 2. 设计结构
- 分层档案层：把资源、产品与设施信息分层归档。
- 链路扩展层：支持后续新增产品链、设施链与治理链。
- 产品校验预览层：把产品 profile 的 `role_tag / tradable / unlock_stage` 转成验证后能力解锁和下一步用途说明。
- 兼容承接层：保持与 P0/P1/P2 规则的兼容演进。
- 治理回写层：沉淀扩展规则与维护入口。

## 3. 关键接口 / 入口
- 分层 profile
- 链路扩展定义
- `validation_unlock_preview`: `product_id`、`role_tag`、`unlock_stage_before`、`unlock_stage_after`、`tradable`、`capability_unlocked`、`next_use_case`、`stage_progress_delta`、`recommended_next_action`、`validation_value_class`
- 兼容承接规则
- 治理维护入口

## 4. 约束与边界
- 扩展能力不能破坏已有主链路。
- 档案层必须为后续扩展服务。
- `product_validation_preview_missing` 表示产品校验收益不可读，不代表要新增成就系统、产品目录或科技树。
- 校验预览不得重平衡产品链、配方或阶段门槛。
- 不在本专题直接铺开所有扩展内容。

## 5. 设计演进计划
- 先收敛分层档案结构。
- 再定义扩展承接规则。
- 最后回写治理入口。
