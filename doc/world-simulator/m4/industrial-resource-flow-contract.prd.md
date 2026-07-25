# M4 工业资源流转合同

- 对应设计文档: `doc/world-simulator/m4/industrial-resource-flow-contract.design.md`
- 对应项目管理文档: `doc/world-simulator/m4/industrial-resource-flow-contract.project.md`

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

### Out of Scope

- 完整物流寻路/拥塞网络、金融市场、科技树或职业系统。
- 电价、电耗、产率、阈值、损耗、速度、吞吐或维护公式重平衡。
- LLM/provider 策略、Viewer 图谱/缩放、PowerStorage hard-removal 历史专题及其各自的专业合同。

## 3. AI System Requirements (If Applicable)

N/A: 本专题不新增 Agent 推理、provider observation 或公共 action schema。任何未来把工业恢复诊断写入 Agent observation 的变更，仍须通过对应 LLM/provider contract 与 schema version。

## 4. Technical Specifications

### 工业与材料

- Recipe、Product 与 Factory 模块定义工业规则；运行时负责授权校验、状态落地、事件、回放与拒绝语义。
- `MaterialLedgerId` 可表示 world、agent、site 与 factory 账本。旧快照的材料兼容与 WASM 请求兼容必须由 runtime 处理，不能由产品或 Viewer 层伪造。
- `TransferMaterial` 的即时或在途结果必须保持守恒、确定性和可审计；其具体字段、事件顺序和 ABI 以 runtime/ABI 真值为准。

### 电力与设施

- Owner-held electricity 是唯一当前口径：初始化清洗 Location electricity，运行时拒绝 Location 入账与已下线充放电路径。
- `BuildFactory(factory.power.radiation.mk1)` 的设施注册、owner/location 继承和发电入账由 runtime 实现拥有。
- 历史包含 storage 或 Location pool 的快照/事件必须由专业实现明确兼容或拒绝，不能静默吞错。

## 5. Risks & Roadmap

- 状态、回放、WASM 请求和 builtin identity 的兼容风险必须由 runtime 回归覆盖。
- quote 债务若只在执行后事件中展示后果，仍会削弱工业经营选择的可读性；该风险保持 GitHub #2166 路由，直到对应专业实现与验证完成。
- Viewer industry graph 继续可消费工业语义，但不因本专题而证明 Viewer 当前交互或视觉验证通过。

## 6. Validation & Decision Record

- DRF-001: 当前工业资源流转不得恢复 `PowerStorage`、`DrawPower`、`StorePower` 或 Location electricity pool。
- DRF-002: 四组历史三件套删除前，已将其现行语义、完成态 provenance、未收口 #2166 债务与相邻专业 authority 收敛到本三件套及引用入口。
- DRF-003: 产品承诺与当前体验 verdict 不在本文新增；产品层和 QA 仍须以各自权威与 fresh evidence 为准。
