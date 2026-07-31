# M4 工业资源流转合同

- 对应设计文档: `doc/world-simulator/m4/industrial-resource-flow-contract.design.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history；本文与 design 保留 M4 资源流转和确定性报价合同。

## 1. Executive Summary

本专题是 M4 工厂、材料物流与电力资源流转的现行专业合同。它收敛四组已完成历史三件套：工业 WASM、M4 电力系统、材料多账本/在途物流，以及 Location 电力池下线与辐射电厂建造；历史实现细节可从 Git history 和 GitHub task issue evidence 追溯。

## 2. User Experience & Functionality

### In Scope

- 工厂按配方消耗输入、产生产物，并在确定性、可审计的世界状态中推进。
- 材料可在 world、agent、site 与 factory 账本间转移；跨账本调运遵守现有距离、损耗、延迟与吞吐限制，并保留快照/回放兼容边界。
- 电力归明确 owner 持有；Location 不持有有效 electricity 库存。`factory.power.radiation.mk1` 可把建造与 `PowerPlant` 注册接到 owner 入账路径。
- 当前不提供 `PowerStorage`、`DrawPower` 或 `StorePower` 路径；不得将历史储能或 Location 电力池模型重新表述为现行能力。
- 工业模块、builtin descriptor/catalog、artifact hash 与 identity manifest 继续由相应 ABI/runtime canonical contract 拥有；本专题不替代其 release policy。

### Unresolved player-readable debts

GitHub #2166 中的 `schedule_quote`、`logistics_transfer_quote` / `transfer_impact_preview`、`product_validation_quote` / `validation_unlock_preview`、`power_survival_quote` / `energy_recovery_preview`、`power_sale_quote` / `energy_liquidity_preview`、`market_quote_decision_preview`、`refine_quote` / `refine_preview` 与 data-access recovery 仍是未收口的可读性债务。它们只要求未来实现解释既有规则与后果，不代表本次文档迁移已经实现 quote、Viewer surface、ABI 变更、数值重平衡或发布 readiness。

工业恢复的玩家语义由 gameplay agency/top-level 合同承接：资源不足、采空、不可达或精炼短缺不能把 Agent 的自动裁剪、改道或拒绝伪装成原意图已经完成；正式 surface 将原 accepted intent、约束原因、保留进度和 wait / repair / reroute / reprioritize 的下一决策连在同一因果链。`refine_value_class` 的唯一现行玩家分类是 `enough_to_advance / partial_progress / poor_power_tradeoff`；历史 `enough_for_next_step` 只作该首项的迁移别名，不另行冻结公式或行为。

历史 LLM debug 资源注入保持 `LLMSKIP`/debug-probe 边界：默认不暴露给正式玩家，非 debug 模式必须拒绝，且不得作为工业 progression、经济结果、active-LLM formal lane 或 playability/release 证据。该边界不新增调试工具、玩家资源入口或运行时实现承诺。

### Out of Scope

- 完整物流寻路/拥塞网络、金融市场、科技树或职业系统。
- 电价、电耗、产率、阈值、损耗、速度、吞吐或维护公式重平衡。
- LLM/provider 策略、Viewer 图谱/缩放；simulator `PowerStorage` hard-removal 的当前边界已在本文收敛，历史实施仅从 Git history / GitHub task evidence 追溯。

## 3. AI System Requirements (If Applicable)

N/A: 本专题不新增 Agent 推理、provider observation 或公共 action schema。任何未来把工业恢复诊断写入 Agent observation 的变更，仍须通过对应 LLM/provider contract 与 schema version。

## 4. Technical Specifications

### 工业与材料

- Recipe、Product 与 Factory 模块定义工业规则；运行时负责授权校验、状态落地、事件、回放与拒绝语义。
- `MaterialLedgerId` 可表示 world、agent、site 与 factory 账本。旧快照的材料兼容与 WASM 请求兼容必须由 runtime 处理，不能由产品或 Viewer 层伪造。
- `MaterialStack.kind` 是模块/domain 拥有的字符串材料标签，可以表达 `compound`、`hardware` 等工业材料；它不能扩展、伪装或自动映射为 simulator 内建 `ResourceKind`。内建资源仅有 `Electricity | Data`，模块资产的 manifest、hash、interface、capability、limits、owner 与安装/调用拒绝继续服从 WASM ABI/runtime authority。
- `TransferMaterial` 的即时或在途结果必须保持守恒、确定性和可审计；其具体字段、事件顺序和 ABI 以 runtime/ABI 真值为准。

### 电力与设施

- Owner-held electricity 是唯一当前口径：初始化清洗 Location electricity，运行时拒绝 Location 入账与已下线充放电路径。
- 这里的 `PowerStorage` 仅指已移除的 simulator facility/action/event 语义；不等同于仍由 runtime 专业域管理的 `m1_power_storage` builtin，也不得据此删除 legacy scenario runner 对 `require_power_storages` 的明确拒绝。
- `BuildFactory(factory.power.radiation.mk1)` 的设施注册、owner/location 继承和发电入账由 runtime 实现拥有。
- 历史包含 storage 或 Location pool 的快照/事件必须由专业实现明确兼容或拒绝，不能静默吞错。

### 动态电力报价与订单合同

- 动态报价配置保留 `dynamic_price_enabled`、base/min/max price、scarcity adjustment 与 allowed price band；`market_distance_price_per_km_bps` 只作为 deprecated snapshot/config 兼容字段读取，当前报价不使用距离调整。具体数值由当前配置和 runtime 测试拥有，不在本 PRD 冻结。
- 启用动态报价且提交价为零或未显式指定时，交易使用权威 current quote；显式价格超出 current quote 允许带宽时拒绝，并返回可解释的 quote/band 原因。
- current quote 由 base price 加 scarcity 调整后钳制在 min/max 范围；相同权威状态必须产生相同报价，event/receipt 记录实际 quoted price 与 settlement amount，replay 不重新报价。
- limit order 只有在权威 current quote 同时落入买卖双方 limits 时成交；买单按最高限价优先、卖单按最低限价优先，同价按更早 order identity。未匹配订单保持 open 且可取消，不代表 escrow 或保证成交。
- 本合同保留价格约束与确定性规则，但不声称 `power_sale_quote` / `energy_liquidity_preview` 已实现；玩家可读机会成本仍是对应 GitHub task evidence 记录的未收口债务。

## 5. Risks & Roadmap

- 状态、回放、WASM 请求和 builtin identity 的兼容风险必须由 runtime 回归覆盖。
- quote 债务若只在执行后事件中展示后果，仍会削弱工业经营选择的可读性；该风险保持 GitHub #2166 路由，直到对应专业实现与验证完成。
- Viewer industry graph 继续可消费工业语义，但不因本专题而证明 Viewer 当前交互或视觉验证通过。

## 6. Validation & Decision Record

- DRF-001: 当前工业资源流转不得恢复 `PowerStorage`、`DrawPower`、`StorePower` 或 Location electricity pool。
- DRF-002: 四组历史三件套删除前，已将其现行语义、完成态 provenance、未收口 #2166 债务与相邻专业 authority 收敛到本三件套及引用入口。
- DRF-003: 产品承诺与当前体验 verdict 不在本文新增；产品层和 QA 仍须以各自权威与 fresh evidence 为准。
