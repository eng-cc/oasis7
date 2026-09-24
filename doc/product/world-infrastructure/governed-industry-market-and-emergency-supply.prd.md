# 受治理工业、市场结算与危机保供（迁移 provenance）

## 文档身份

- 所属产品模块：权威世界基础设施
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`superseded`
- Owner role：`producer_system_designer`
- Last reviewed：2026-09-23
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)

## 迁移状态

本页保留工业扩展、结算资产、市场地理和危机保供语义的历史 provenance，避免迁移时丢失规则。它已不再是权威世界基础设施的 active authority、路线图或验收入口：基础设施只提供最终性、权威状态、复制、存储、恢复和确定性执行边界。旧 AC-1 至 AC-5 的产品语义已由 [`世界规则与玩法系统`](../world-rules-core-gameplay/prd.md) 下的稳定专题锚点接收。当前 `PRD-GAME-018` 已改为引用这些接收锚点，旧源路径检查未发现活跃专业 backlink；此前关于 GAME-018 仍使用旧 AC-1 至 AC-4 来源及 AC-3 产品接收待定的记录已过时。本页继续保留为 `superseded` 历史 provenance；本次状态修正不构成删除授权。专业实现、数值、runtime/P2P 合同和当前公开 claim 仍由各自专业 authority 拥有。


本文定义基础工业规则与玩家/Agent 创造的受治理能力如何共同扩展世界，市场如何在全球发现和物理结算之间保持一致，以及常态价格和紧急保供的制度边界。它不定义配方、价格、税费、版税、汇率、订单簿、物流、escrow、危机阈值或任何 runtime/UI 实现。

## 生命周期闭合

- 接收 authority：[`受治理的区域能力与扩展`](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-009)、[`工业需求目标与生产交付结算`](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#req-sc31-009)与 [`常态市场与有界紧急保供`](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-004)。
- 剩余语义：旧 AC-1 至 AC-5 的提案/模拟/试点/准入、许可/开放、OC/世界资格分离、发现/物理物流/escrow，以及常态/紧急保供语义均已有唯一 active REQ/AC 锚点；专业 PRD、runtime、P2P、testing 仍拥有各自合同和证据。本页仅保留历史 provenance。
- 当前消费者状态：[`PRD-GAME-018`](../../game/gameplay/gameplay-industrial-creation-and-cross-region-market-contract.prd.md) 已由活跃 [`GR-009`](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-009) 至 [`GR-011`](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-011)、[`SC31-009`](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#req-sc31-009) / [`AC-SC31-010`](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-010) / [`AC-SC31-011`](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-011) 与 [`ES-004`](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-004) 锚点接收对应语义；其中 [AC-GAME-018-03](../../game/gameplay/gameplay-industrial-creation-and-cross-region-market-contract.prd.md#ac-game-018-03) 映射 OC 与世界资格边界至 GR-011，没有待接收的 AC-3 产品语义。旧源路径 census 在 `doc/` 中只找到下方模块索引的历史 provenance 链接，未发现活跃专业 backlink。
- 未闭合条件：本页继续作为 `superseded` 历史 provenance 保留；该保留不表示专业引用仍未修复，也不改变其非权威状态。后续是否删除及删除时如何处理本页和模块索引中的历史 provenance，须另行审查与授权；本次状态修正不删除任何来源。
- 稳定引用：本页的历史 AC 保持可读；当前产品入口经 [`世界规则与玩法系统`](../world-rules-core-gameplay/prd.md#活跃产品专题) 下的工业、市场和保供专题接收，专业执行与验证仍由各自 authority 拥有。
- 删除条件：任何后续删除须先确认专业/历史引用和必要 inventory 记录均已妥善处理、专业实现/经济 authority 仍可达且治理/链接检查通过，并取得独立授权；满足前不得删除本页。

### 迁移映射（本页只作历史 provenance）

| 旧源条款 | 当前 active 产品接收锚点 | 接收语义与保留边界 | 当前状态 |
| --- | --- | --- | --- |
| [AC-1](#ac-1) | [REQ-WR-GR-009](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-009) / [AC-WR-GR-009](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-009) | 提案→确定性模拟→受限试点→治理准入；保留 scope、资源/机会成本、权限、失败原因、恢复/复查和不得旁路的 Non-Goals | 产品已接收；GAME-018 改指向接收 authority，本行仅保留历史映射 |
| [AC-2](#ac-2) | [REQ-WR-GR-010](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-010) / [AC-WR-GR-010](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-010) | 许可/版税/收益权的用途、区域、期限、维护、撤销/争议、开放条件；到期/开放不形成永久垄断或追溯改写 | 产品已接收；GAME-018 改指向接收 authority，本行仅保留历史映射 |
| [AC-3](#ac-3) | [REQ-WR-GR-011](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-011) / [AC-WR-GR-011](../world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-011) | OC 链上转让与世界资源/信用/合同/工业准入/区域资格双向隔离；锁定/委托/控制人只是治理输入，仍需本地贡献；不承诺 fee-free transfer 或 OC↔世界配额/兑换 | 产品已接收；GAME-018 [AC-GAME-018-03](../../game/gameplay/gameplay-industrial-creation-and-cross-region-market-contract.prd.md#ac-game-018-03) 映射至 GR-011，无 pending 产品接收项 |
| [AC-4](#ac-4) | [REQ-SC31-009](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#req-sc31-009) / [AC-SC31-010](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-010) / [AC-SC31-011](../world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-011) | 新鲜度/来源的发现、报价/合同、路线、出发/在途/到达、目的地存储、ownership/escrow 里程碑、争议与 exactly-once 恢复分层 | 产品已接收；GAME-018 改指向 SC31 接收 authority，本行仅保留历史映射 |
| [AC-5](#ac-5) | [REQ-WR-ES-001](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-001) / [AC-WR-ES-001](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#ac-wr-es-001)；[REQ-WR-ES-002](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-002) / [AC-WR-ES-002](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#ac-wr-es-002)；[REQ-WR-ES-003](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-003) / [AC-WR-ES-003](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#ac-wr-es-003)；[REQ-WR-ES-004](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-004) / [AC-WR-ES-004](../world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#ac-wr-es-004) | 正常市场与有证据、有范围/期限、补偿/公开分配、复核/申诉、可退出的系统性危机区分；退出恢复常态且不迁移待决请求 | 产品已接收；GAME-018 改指向 ES-004 接收 authority，本行仅保留历史映射 |
## 1. 产品目标

世界提供可预测的基础物理、资源、工业与结算规则；玩家和 Agent 可以在这些规则上提出新的配方、材料、设施或模块能力，但不能用未审计上传、一次投票或模型生成文本直接改写权威世界。新能力必须先证明它能在同一确定性世界中运行，再经治理获得有限、可追溯的可用范围。

市场让跨区发现、专业化和竞争成为可能，却不把货物、风险或区域差异瞬移掉。正常价格来自市场和合约；只有预定义系统性必需品危机才能触发受严格限制的保供干预。

## 2. 基础规则与受治理工业扩展

### 2.1 世界基础与创作边界

- 官方基础层拥有通用物理、资源、工业、结算、权限和治理规则；它提供开放式生产与服务的共同底座，而不是预写全部产业结果。
- 玩家或 Agent 可以提出配方、材料、设施、模块和等价工业能力，但提案不自动取得世界写入权、默认全局可用性或绕过既有资源/安全/权利边界。
- 每项提案遵循 `可审计提案 -> 确定性模拟 -> 原型或受限试点证据 -> 治理准入/范围决定`。模型建议、投票支持或技术原型任一项都不能单独替代其余环节。

### 2.2 创作收益与开放

- 获准能力可以在明确期限、区域或用途范围内给予创作者可读的许可和版税/收益权，以奖励可验证的工业创新并支持后续维护。
- 许可/收益权必须受范围、期限、审计和反垄断/开放条件约束；当预先公布的开放条件达成、许可到期或准入范围扩大时，能力按规则开放，不能被无限期私有化为永久产业封锁。
- 创作者权利不改变基础世界规则、公共安全、既有资产/身份或其他玩家的宪制保护；许可争议、拒绝、到期和开放须保留可读原因与可审计结果。

## 3. 结算资产与市场地理

### 3.1 OC 与地方世界经济

- OC 是可用于外部基础结算和治理的链上资产；游戏世界不限制其链上自由转让，也不把世界资格、地方合同或区域资源伪装成对 OC 链上所有权的额外限制。
- 世界内可以存在区域资源、服务信用、合约、地方单位或其他受规则约束的经济记录。它们的用途、资格和结算不自动等同于 OC，亦不自动获得链上转让、赎回、治理或外部价值承诺。
- 游戏内资格、竞争与高影响权利仍可依世界中的能力、参与、授权和治理条件判断；外部 OC 持有或转让本身不能绕过这些条件。

### 3.2 全球发现、物理交付与里程碑结算

- 市场可以全球发现报价、信誉、供给与需求，但发现结果受情报来源、新鲜度、不确定性和授权边界约束；陈旧或私有情报不得伪装成全局实时真相。
- 物品跨区成交后仍须通过可审计的物理路线、时间、损耗、风险和目的地存储完成交付。发现或匹配不等同于货物已经到达、所有权已经最终转移或生产阻塞已经解除。
- 资金或等价结算遵循合约的 escrow、交付/里程碑和争议处理边界；它不替代物理交付，也不得由单方宣称、离线界面或未验证 receipt 直接完成。

## 4. 价格形成与紧急保供

- 正常状态下，价格、供给关系和竞争由市场、合约、物流与公开规则形成；区域不得把日常限价、任意补贴或常态配给包装成普通治理便利。
- 仅在预先定义的系统性必需品危机中，且有可审计事实证据时，才可触发品类、受影响范围和期限均受限的采购、补偿、分配或 rationing 干预。
- 紧急保供必须公开其授权、理由、受益范围、补偿、分配结果、复核和申诉路径；不得成为永久国家控制、任意没收、常态价格操纵或绕过世界恢复/治理程序的旁路。

## 5. 范围与权威边界

产品层定义 `基础规则 -> 工业提案/模拟/试点/治理准入 -> 许可或开放 -> 发现/合约 -> 物理物流与目的地存储 -> escrow/里程碑结算 -> 正常市场或有界紧急保供` 的玩家与制度语义。

`game` 拥有工业、市场、竞争和经济平衡；`world-runtime` 拥有资源、提案、确定性模拟、许可、物流、结算、escrow、receipt 和危机执行；`p2p` 拥有 OC 链上资产、身份、治理/签名和分布式状态边界；QA 拥有具体验证。产品层不宣称新的配方、市场、外部兑换、OC 价值、支付服务或紧急机制当前已实施、可用或合规。

## 5.1 关键决策与未决迁移边界

- 关键决策：工业、市场和紧急保供的玩家语义由世界规则与玩法系统模块接收；本页只保留迁移期间的产品语义记录，基础设施产品层继续负责最终性、权威状态、确定性执行和恢复边界。专业 runtime/P2P 合同仍由对应 authority 拥有，不在本页复制。
- 未决问题：AC-1～5 的产品语义已由上列 active 条款接收，且当前 `PRD-GAME-018` 已导航至对应接收 authority；此前关于旧来源链接和“AC-3 产品接收待定”的记录已纠正。是否删除本页仍属单独的后续处置，应核对历史 provenance、模块索引和必要 inventory 记录并取得授权；产品条款接收本身不证明工业、市场、结算或危机能力当前已实现、可用或发行。
- 决策 role 与触发条件：`producer_system_designer` 负责接收与删除决定，`gameplay_designer` 负责工业/市场规则，`runtime_engineer` 与 `blockchain_ops_engineer` 负责执行、分布式状态和证据边界；只有接收文档完成回填并修复引用后才可删除本页。
- 临时排除范围：本页保持 `superseded` 和非权威状态，不新增配方、价格、支付、外部兑换、危机机制或发行 claim。

## 6. 组合验收

<a id="ac-1"></a>
- AC-1：工业能力提案样例可区分可审计提案、确定性模拟、原型/试点证据与治理准入；任一单独环节都不会直接取得权威世界能力。
<a id="ac-2"></a>
- AC-2：许可样例说明创作者收益/版税的范围和期限，并在到期或既定开放条件后按规则开放；创作收益不会形成永久垄断或覆盖世界权利。
<a id="ac-3"></a>
- AC-3：OC 的链上转让与世界内区域资源/信用/合约记录保持语义分离；外部持有不会自动取得世界内竞争、治理或高影响资格。
<a id="ac-4"></a>
- AC-4：跨区市场样例可区分情报发现、报价/合约、物理路线/到达/损耗/目的地存储，以及 escrow/里程碑结算；任一前序状态不会伪装成最终交付或结算。
<a id="ac-5"></a>
- AC-5：正常市场与系统性必需品危机样例可区分。后者只有在范围、期限、证据、补偿、公开分配、复核和申诉齐备时才可触发，且退出后不保留常态价格控制。

## 7. 验收追踪

| 产品承诺 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| AC-1 / AC-2 | gameplay_designer / wasm_platform_engineer / runtime_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 工业提案、模拟/试点、准入、许可和开放的权限/审计/回放组合证据 | test_tier_full |
| AC-3 | blockchain_ops_engineer / runtime_engineer / qa_engineer | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | OC 链上转让与世界内资格/记录分离、无旁路资格的证据 | test_tier_full |
| AC-4 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 情报新鲜度、跨区发现、物流、目的地存储、escrow/里程碑与失败恢复组合证据 | test_tier_full |
| AC-5 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 正常市场、危机触发、范围/期限、补偿/分配、退出和申诉的组合证据 | test_tier_full |

## 8. Non-Goals

- 不规定任何配方、材料、模块、生产配平、价格、税费、版税、汇率、库存、损耗、路线、订单簿、escrow 或补偿公式。
- 不实现模拟、原型、试点、治理准入、许可、市场撮合、物流、存储、结算、危机采购或 rationing。
- 不把 OC 转让自由扩写为游戏内资格、自动兑换、外部交易所、AMM、法币支付、投资收益或任何价值承诺。
- 不改变区域 charter、土地 tenure、宪制修订或玩家账户/身份的专业权威。
