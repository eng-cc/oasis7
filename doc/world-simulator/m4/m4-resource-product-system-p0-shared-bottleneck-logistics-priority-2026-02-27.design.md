# M4 资源与产品系统 P0：共享中间件竞争 + 运输优先级（2026-02-27）设计

- 对应需求文档: `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/m4-resource-product-system-p0-shared-bottleneck-logistics-priority-2026-02-27.project.md`

## 1. 设计定位
定义 M4 资源与产品系统 P0 设计，聚焦共享中间件竞争与运输优先级这一首批瓶颈。

## 2. 设计结构
- 共享瓶颈层：识别资源链路中的共享中间件竞争点。
- 物流优先级层：定义运输调度与抢占优先级。
- 调运影响预览层：在提交前解释到达时间、损耗、吞吐占用、优先级理由和产线阻塞变化。
- 运行反馈层：让瓶颈状态影响产线效率与调度决策。
- 验证层：通过典型物流场景验证优先级规则。

## 3. 关键接口 / 入口
- 共享资源/中间件状态
- 物流优先级规则
- `transfer_impact_preview`: `priority_class`、`why_this_priority`、`throughput_slot_impact`、`blocked_recipe_before`、`blocked_recipe_after`、`delivery_eta_ticks`、`loss_amount`、`recommended_transfer_action`
- 调度反馈入口
- 物流验证场景

## 4. 约束与边界
- 优先级规则必须可解释。
- 抢占不能破坏物料守恒。
- `transfer_impact_preview_missing` 表示提交前物流取舍不可读，不代表要新增完整寻路、拥塞网络或重平衡速度/损耗/吞吐。
- 不在本专题覆盖后续维护压力和治理联动。

## 5. 设计演进计划
- 先收敛瓶颈问题。
- 再定义物流优先级。
- 最后执行场景验证。
