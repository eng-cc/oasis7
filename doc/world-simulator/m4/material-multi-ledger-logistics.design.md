# M4 材料多账本与物流约束（设计文档）设计

- 对应需求文档: `doc/world-simulator/m4/material-multi-ledger-logistics.prd.md`
- 对应项目管理文档: `doc/world-simulator/m4/material-multi-ledger-logistics.project.md`

## 1. 设计定位
定义 M4 材料多账本与物流约束设计，统一材料在多账本、多运输链路中的流转语义。

## 2. 设计结构
- 多账本层：为不同材料状态建立独立但可对账的账本。
- 物流约束层：定义运输容量、路径和优先级限制。
- 物流收益预览层：把调运预计到达量、损耗、ETA、吞吐占用和产线阻塞变化转成玩家提交前 quote。
- 对账同步层：保持账本变化与物流状态一致。
- 审计验证层：支持材料流转回溯和异常定位。

## 3. 关键接口 / 入口
- 材料多账本模型
- 物流约束规则
- `logistics_transfer_quote` / `transfer_impact_preview`: `from_ledger`、`to_ledger`、`material_kind`、`requested_amount`、`estimated_received_amount`、`loss_amount`、`ready_at_tick`、`delivery_eta_ticks`、`priority_class`、`throughput_slot_impact`、`blocked_recipe_before`、`blocked_recipe_after`、`why_this_priority`、`recommended_transfer_action`
- 对账同步入口
- 材料流转审计

## 4. 约束与边界
- 多账本不能产生重复计算或黑洞。
- 物流限制需与工业主路径一致。
- `transfer_impact_preview_missing` 是玩家侧物流可读性缺口，不代表要新增完整寻路/拥塞网络或调整物流数值。
- 调运 quote 只解释现有距离、损耗、延迟、吞吐和优先级规则，不改变 runtime ABI 或 viewer 实现。
- 不在本专题扩展完整金融清结算。

## 5. 设计演进计划
- 先固定多账本语义。
- 再接物流约束。
- 最后固化对账与审计。
