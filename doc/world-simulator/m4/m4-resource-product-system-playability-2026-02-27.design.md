# M4 资源与产品系统：合理性与可玩性一体化设计（2026-02-27）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-playability-2026-02-27.project.md`

## 1. 设计定位
定义 M4 资源与产品系统的合理性与可玩性一体化设计，把工业逻辑与玩家体验同时纳入。

## 2. 设计结构
- 系统合理性层：保证资源、产品和设施关系自洽。
- 可玩性层：让反馈、目标和瓶颈对玩家有可感知节奏。
- 产品验证收益层：把 `ProductValidated` 的成功结果转成能力解锁、阶段推进和下一步用途说明。
- 平衡调节层：为成本、产出与压力提供调节旋钮。
- 验证回归层：通过场景与指标评估可玩性改动。

## 3. 关键接口 / 入口
- 资源/产品主模型
- 可玩性反馈点
- `product_validation_quote` / `validation_unlock_preview`: `product_id`、`role_tag`、`unlock_stage_before`、`unlock_stage_after`、`tradable`、`capability_unlocked`、`next_use_case`、`stage_progress_delta`、`recommended_next_action`、`validation_value_class`
- 平衡参数入口
- 体验验证场景

## 4. 约束与边界
- 可玩性增强不能破坏系统自洽。
- 平衡调整要可追踪。
- `product_validation_preview_missing` 是玩家侧产品验证收益可读性缺口，不代表要新增成就系统、科技树、产品目录或重平衡产品链。
- 校验预览只解释已有产品 profile、阶段状态和下一步用途，不改变动作主路径或 runtime ABI。
- 不在本专题替代各阶段细分专题。

## 5. 设计演进计划
- 先固定一体化目标。
- 再补反馈与平衡旋钮。
- 最后执行体验验证。
