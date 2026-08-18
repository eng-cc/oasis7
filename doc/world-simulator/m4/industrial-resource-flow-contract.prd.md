# M4 工业资源流转合同

- 对应设计文档: `doc/world-simulator/m4/industrial-resource-flow-contract.design.md`
- 专业 PRD-ID: `PRD-WORLD_SIMULATOR-047`
- 上层产品映射: [`SC-21 多阶段工业流水线`](../../product/world-rules-core-gameplay/prd.md#5-done成功标准与验收)
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

### 多阶段批次、预留与背压合同（PRD-WORLD_SIMULATOR-047）

- 每批中间品必须保留可回溯的来源阶段/边、父级 receipt、材料与规格/品质适用证据、数量及当前到达账本。下游只有在批次已经结算并到达、且按当前规则明确判为适用时才能消费；`待验证/证据不足` 与 `不适用` 都必须在下游 sink 或进度前 fail closed。材料名称、Agent 建议、Viewer 缓存或模块元数据不能替代适用性证据，也不能把未知批次推断、降级或混合成适用批次。
- 中间品预留是权威、数量有界且绑定批次、阶段/边、消费主体与生命周期的 hold；报价、计划、上游已接受或加工中状态不构成预留。同一数量不得同时承诺给多个下游。预留过期、撤销或前置失效时，只能按适用合同释放、保持待决或原子拒绝，不能重复扣料、生成第二份库存或静默转成欠费/补贴。
- 每条中间品边必须声明有限 buffer 与确定性的背压结果。下游不可用或 buffer 满时，只能保持尚未消费的上游投入、把已结算产出放入仍有容量的 buffer，或原子拒绝新的上游承诺；不得静默丢弃、瞬移、无限堆积、自动改道或伪造下游完成。overflow、返工、报废或 salvage 只有在对应专业合同存在时才能产生一次可审计结果，否则必须拒绝。
- split、混批与 merge 必须逐输入批次重新校验适用性，并使用稳定、可复现的分配与齐套规则；专业合同必须明确采用“只消费适用数量并保留其余批次”还是“整笔原子拒绝”，不得依赖客户端/Agent 提交顺序、隐藏混合或表现层缓存。阶段、配方、设施或边变化后，未结算承诺按当前规则重新裁决，不能继承旧候选资格或稳定进度。
- 跨阶段 lineage 必须在持久化、恢复与 replay 后仍能从子阶段承诺、边和中间品状态回溯到父级 receipt、预留、适用结论与实际损耗。重复提交、Agent 重试、重连或 replay 对每个阶段至多产生一次 sink 与产出，不能跳过未满足的前置，也不能把未知/不适用批次重试成适用。`PRD-WORLD_RUNTIME-001/019` 拥有执行顺序、事件/receipt、持久化、幂等与 replay/recovery；M4/domain 拥有批次、边、适用性和预留语义；game 拥有节奏、数值与机会成本。
- 两个或以上有效的 accepted intents 共同消耗同一 stage execution slot、edge throughput 或 destination buffer 时，必须形成一个权威 allocation domain，并在同一 allocation snapshot 上评估完整 contender 集、可用容量与既有 holds。`accepted` 不代表已经获得容量；每个 contender 只能取得 full、专业 profile 声明支持的 partial、deferred 或 denied 结果之一，且总 hold 与消费不得超过快照容量。裁决使用稳定冲突键、当前专业策略及稳定 intent identity 解 tie，不能由网络/调用到达顺序或逐请求局部写入决定。
- partial 结果必须只消费已分配数量并保留可追溯的未满足部分；deferred/denied 不得生成隐性 hold、sink 或进度。未消费 hold 在取消、过期、消费者失效或前置变化时至多原子释放一次；容量释放或上游到达后，仍有效 contender 只能从新的 canonical snapshot 重评，不能回滚已消费量、复制 hold/产出或静默提升旧意图资格。
- 重评必须保留原 intent identity、lineage、既有 hold、评估依据与结果；retry、reconnect 与 replay 不得刷新优先级、重置等待或产生第二次效果。专业 profile 必须声明可观察的重评条件，以及延期意图的过期、终止或重新规划边界；在容量反复可用且意图仍有效时，不得无解释地无限跳过。专业 surface 只能披露当前主体有权读取的 shared resource、held/unmet quantity、full/partial/deferred/denied、顺序理由、下一复查点与恢复动作，不得伪造精确 queue position/ETA 或泄露其他主体私密事实。
- 改变阶段、配方、设施能力/版本、边目的地或其他产出因果/消费资格的配置变更必须创建新候选身份，并以一个权威 cutover snapshot 线性化边界；只改变展示或非因果元数据的变更不得误切候选。同 tick 的变更与生产/运输结果沿用 runtime 的稳定冲突顺序。边界前产生的 hold、WIP、transit、buffer batch 与 receipt 保持旧身份，边界后新意图只属于新身份，不能按材料同名、目的相同或调用顺序静默重绑。
- Cutover 必须为每项旧状态产生一次确定处置：未消费 hold 只能在旧身份下保持、释放、延期或拒绝；WIP 只有在旧前置仍有效且专业 profile 明确允许时才能 drain/finish，否则暂停、终止或进入受支持的 rework/salvage；transit 保留原 receipt、edge、目的账本、规格和已发生损耗，失效时只能经显式 transfer/return/hold/reject 处理；buffer batch 保留旧 ledger 与 lineage，新候选仅能通过专业合同支持的 handoff/conversion 并重新校验适用性与容量后消费。没有对应处置能力时必须 fail closed，禁止隐式混批、重贴标签、瞬移、自动改道、同时退款与完成或半旧半新的状态。
- 新候选不得继承旧候选的稳定窗口、里程碑、排队资格、reservation 或 receipt。受支持的 handoff/conversion 必须只产生一次 receipt，并关联父 receipt、旧/新候选身份、数量、实际损耗与变更原因；旧 receipt 不能直接复用为新阶段 sink、进度或奖励。`PRD-WORLD_RUNTIME-001/019` 拥有 cutover 的原子顺序、journal/event/receipt、持久化、幂等与 replay/recovery；M4/domain 拥有导致失效的条件、各状态允许的处置、适用性和迁移语义；game 拥有换线节奏、选择与机会成本。
- Cutover boundary 与逐项 disposition 必须从同一有序权威输入确定并可由 journal 恢复；重复变更、retry、reconnect 或 replay 不得重复释放 hold、完成/复制 WIP、交付 transit、迁移 buffer、退款或重新开启旧候选。任何部分失败只能保持可追溯的旧状态或显式拒绝。专业 surface 至少表达旧/新身份、受影响状态、处置、保留/损失/占用、稳定窗口影响和下一步，不得把旧进度显示为新候选进度或伪造 ETA/queue position。

本合同只冻结多阶段资源流的确定性边界，不声明当前 runtime、Agent、Viewer 或 pure API 已实现该能力，也不冻结品质数值/公式、buffer 数字、runtime schema/枚举、图/队列算法或 UI 布局。

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
- DRF-004 / `PRD-WORLD_SIMULATOR-047`: `test_tier_required` 至少覆盖两阶段适用成功、未知/不适用拒绝、同一 allocation snapshot 上多 contender 的容量不足争用、full/partial/deferred/denied 守恒、释放后单次重评、buffer 满背压及根因/派生 blocker、稳定 tie/order witness、retry/reconnect/replay 不插队且单次效果；还须分别在旧 hold、WIP、transit 与 buffer batch 存在时执行因果 cutover，证明稳定边界、旧/新身份分离、唯一 disposition、新窗口归零、未知/不兼容迁移 fail closed，且请求顺序置换或 replay 不改变结果。`test_tier_full` 覆盖不同规格批次的三阶段 split/merge 与共享 edge/buffer、跨账本运输损耗、支持的 rework/salvage/return/handoff/conversion、重复换线、持久化/恢复/replay、长期争用无静默 starvation，以及 Viewer 与 pure API 的 lineage、分配/换线结果、守恒、适用结论和单次效果一致性。
