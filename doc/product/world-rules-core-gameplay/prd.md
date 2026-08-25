# 世界规则与核心玩法 PRD

## 文档身份

- 产品模块：世界规则与核心玩法
- 产品模块 slug：`world-rules-core-gameplay`
- 产品层唯一 PRD：`doc/product/world-rules-core-gameplay/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-001`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-08-01`
- 后继文档：`无`
- 下层专业域：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)

本文只定义玩家目标、间接能动性、核心循环、成长与资源压力的产品承诺。玩法规则、数值平衡、专题 PRD-ID 与测试证据由 `game` 专业域拥有。

### 活跃产品专题

- [`首局与持续游玩`](first-session-and-continuation.prd.md)：首局微循环、后引导承接、首次持续能力、中循环展开与失败恢复。
- [`间接控制下的玩家能动性与续接`](indirect-control-agency-and-continuation.prd.md)：玩家通过 Agent 推动世界时的意图可读、因果可解释、干预重排、记忆纠正与回流续接。
- [`Agent 所有权与持续经营`](agent-ownership-and-stewardship.prd.md)：玩家以明确承诺取得、维持或结束自己的 Agent 控制权，并读懂成本、风险和恢复选择。
- [`成熟世界成长与区域参与`](mature-world-progression.prd.md)：首次持续能力之后的独立成长、区域专业化、有限影响与 anti-grind / 恢复边界。
- [`受治理的区域能力与扩展`](governed-regional-capabilities-and-extensions.prd.md)：区域设施从条件报价到有限服务、维护/耗尽/退役的生命周期，以及创作者扩展从治理提案到权威生效、审计与恢复的产品边界。
- [`区域冲突、软赛季与可恢复损失`](chartered-conflict-soft-seasons-and-recovery.prd.md)：宣战、有限参战范围、实体战利品/占领、可恢复重建和不重置世界的软赛季边界。
- [`沟通、合同、声誉与 R&D 连续性`](communication-contracts-reputation-and-rd-continuity.prd.md)：人类沟通与 Agent 合同的边界、持续服务争端、情境声誉及研究归因/份额的长期产品语义。
- [`组织连续性、解散与长期不活跃保护`](organization-continuity-dissolution-and-dormancy-protection.prd.md)：组织 charter 的个人保护底线、可审计解散顺序，以及长期不活跃时的保护、恢复主张与有限处置边界。
- [`常态市场与有界紧急保供`](market-normal-state-and-emergency-supply.prd.md)：常态价格形成、系统性必需品危机的最小授权包、受限干预和可审计退出边界。
- [`普通共同决策与宪制边界`](governed-common-decisions-and-constitutional-boundaries.prd.md)：普通政策、公共财库与 charter 运行事项的有限白名单，以及玩家保护、安全权威、宪制轨道和紧急防绕过边界。
- [`Frontier 扩展与世界信息边界`](frontier-expansion-and-world-information-boundaries.prd.md)：相邻探索、物流与 charter 共同形成的扩展边界，非主权 pioneer priority，以及公共事实、探索数据与实时运营信息的分层可见性。
- 战争/治理的玩家结果、可读预览与反支配边界由本 PRD 承诺；成本、收益、冷却和评分的专业数值权威见 [`gameplay-war-politics-mvp-baseline.design.md`](../../game/gameplay/gameplay-war-politics-mvp-baseline.design.md)。战争不表示已成为当前首局主线。
- [`可玩性证据与承诺边界`](playability-evidence-and-claim-boundaries.prd.md)：玩家杠杆、继续游玩价值与分层证据如何共同约束当前产品结论。

## 1. 产品承诺

玩家通过可读、有代价、有反馈的行动持续影响同一个持久世界，并在权威规则内与其他玩家、Agent 和区域系统产生可审计的涌现结果。产品不设置全体玩家共享的胜利或通关终局；玩家持续完成有边界、可归因的阶段成果，并在同一世界中形成新的能力、区域价值或下一阶段方向。

长期推荐围绕三条相连但不强制线性的抱负轴展开：建立并守住可恢复的能力、用该能力服务区域需求、获得有限且可审计的区域影响。组织、协议或治理等文明尺度项目只作为玩家自愿进入的共同扩展，不能取代独立成长或成为唯一有效路线。

### 分层可进入性与持续世界节奏

产品面向愿意逐步承担系统深度的玩家，而不是只面向能够从首局同时掌握工业、组织、外交和治理全貌的玩家。正式体验先把当前世界状态收成一个可理解的目标、主要阻塞和下一步；玩家可以在准备好时自主展开区域专业化、市场/物流、组织、外交和治理。渐进披露只能重排与解释复杂性，不能隐藏会改变当前损失、锁定、权利、风险或恢复路径的真实取舍，也不能把深层系统削成没有后果的模板或纯自动化。

持续世界的常态节奏是混合的：日常短命令与结果复盘足以维持普通目标，已授权且有边界的 Agent 工作包可在玩家离开时推进；深度建设、外交、区域项目和其他自愿共同扩展可以支持较长会话。高风险竞争或其他需要回应的事项必须给出有界、可预期的窗口与授权/恢复路径，不得把持续在线、在线时长或无止境值守变成取得基本成长、独立恢复或资格的前提。

## 2. 范围与玩家边界

覆盖首局目标、micro-loop、后引导承接、间接控制、资源压力与长期参与。玩家可以观察、决策、行动并处理反馈；不能越过资源、时间、权限、治理或反滥用边界直接改写世界。

### 物理尺度、间接控制与未来候选

玩家影响的是一个有物理尺度且持续存在的世界，但当前默认体验是通过目标、Agent、地点、设施、配方和治理等间接动作推进，而不是第一人称逐块编辑。表现层可以为可读性抽象或夸张，但不得把它呈现为世界物理真值。

当玩家提出当前未开放的过细动作时，产品体验必须给出可执行的 canonical 替代动作；没有安全替代时，必须说明边界和下一次可决策点，而不是伪造动作或只留下无解释的失败。具身或 block-editing 仅是未来候选：只有在强化本模块的间接控制主路线、具备对应专业域合同与验证，并经显式跨域决策后才可进入原型。

canonical 替代不是对原请求的静默自动执行。存在一个或多个安全候选时，每个已发布候选至少说明目标与作用范围、主要成本、主要风险、预计后果、是否仍可撤回，以及为何推荐；这些信息是可比较的预览，不产生资源扣减、权限继承、排队、锁定或世界效果。玩家可以比较、确认、放弃或改道；当替代将引入原请求未包含的新资源承诺、新权限、控制权变化或不可逆后果时，必须由玩家或具备对应范围的有效 Agent 授权明确确认，并在提交时重新校验，不能仅凭原请求、Agent 计划、客户端缓存或推荐排序推定同意。

替代动作只能继承原请求中仍有效且覆盖该替代目标、作用范围与后果的授权；超出部分必须取得新的明确授权。若候选已过期、授权或世界前置发生变化、玩家拒绝，或不存在安全候选，原请求与候选均保持无世界效果，并返回变化原因和下一次可决策点。并发确认、重连或重试至多产生一次 receipt 支持的结果；原请求、所选替代与最终结果之间保留可审计关联，但该关联不是重复执行或扩大权限的凭据。

本产品层只定义上述玩家承诺和端到端边界；玩法动作粒度由 [`doc/game/prd.md`](../../game/prd.md) 与其核心玩法骨架拥有，物理/执行真值由 `world-runtime`，表现真值由 `world-simulator` 的对应专业域文档拥有。

世界只有一条持续、权威的时间线和一套可测量的物理真值。厘米级距离、顺序、成本与持久化结果可以由工业、物流、治理等粗粒度子系统消费，也可以由 Viewer 做可读性抽象，但任何映射都必须确定、可追溯，不能因子系统分辨率或视觉夸张而改写权威结果。

间接控制不等于旁观：对当前受支持的玩家意图，系统必须呈现意图是否被接受、Agent 如何解释并执行、主要世界后果，以及玩家可用的打断、重排、纠正、fallback 或恢复动作；不能以 Agent 自主性为由隐藏因果或让玩家失去下一次决策权。

### Data 所有权与授权边界

Data 是有归属、有获取成本且受授权边界约束的世界资源。未经授权的使用必须原子失败，不产生未授权收益；产品体验需要说明成本、归属、用途、授权状态和可恢复的授权或替代路径，且可读性层或 Agent 自动化不能静默绕过权限。

Data 授权必须贯穿请求的完整生命周期，而不能只在预览或提交入口做一次布尔判断。预览只说明候选 Data 的 owner、recipient / 使用主体、purpose、scope 与当时的授权有效性，不产生访问、转移、消费或可复用权利；已接受但尚未结算的请求仍不得让 recipient 获得 Data 或由其产生收益。提交到结算之间若授权过期、被撤销、作用域变化或已无法证明当前有效，未结算请求必须原子拒绝或进入可读待决，不产生 Data sink、访问收益或隐性义务，并给出重新授权、缩小用途、改用其他合法来源或放弃的适用路径。

只有 receipt 支持的已结算结果才代表一次、且仅一次的授权使用或转移；receipt 必须能追溯 owner、recipient / 使用主体、purpose、scope、授权依据与实际结果，但产品层不冻结这些信息的字段结构。重试、重连、重复提交或历史 receipt 重放不得复制该结果，也不得让已经过期或撤销的授权复活。结算后的合法使用及其 provenance 不因后续撤销而被追溯抹除；纠错、删除、退款、争议和其他副作用仍由对应专业合同裁决。本模块只定义这些玩家承诺与跨阶段不变量，具体许可状态机、时钟、结算规则、幂等实现和副作用矩阵由专业域拥有。

<a id="resource-model-and-cross-module-provenance"></a>
### 资源模型与跨模块 provenance 边界

本表定义玩家世界规则中的来源、可用 sink 类别与不可跨越的语义边界；它不定义数值、汇率、runtime 字段或实现状态。每项的余额、资格与 receipt 必须由相应专业域证明，不能由产品文字产生可用性或公开 claim。分布式基础设施只保证已提交状态的最终性、复制和恢复，不拥有资源玩法语义。

| 资源 / 记录 | 允许的来源与 sink 类别 | 不可转移 / 转换边界 | 反补贴边界 | 专业 owner / 合同 |
| --- | --- | --- | --- | --- |
| `Electricity` / `Data` | 通用资源可按权威规则用于已授权的世界操作、工业/服务消耗与经批准的设施 commission、service 或 upkeep sink。 | 不因材料、产品、设施记录或任何 starter 支持而自动转换、铸造、转移所有权或扩大可用范围。 | 不构成持续赠与；设施不得把它们重写为无成本或无限供给。 | [`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) |
| restricted slot-1 claim/upkeep support | 仅可在符合资格时支持首个非零 Agent claim 及其声明的 upkeep 承诺。 | 不可变为自由余额、玩家间转移、liquid starter OC、设施库存或材料。 | 不是免费认领，也不提供持续 claim/upkeep 或设施补贴。 | [`Agent ownership and stewardship`](agent-ownership-and-stewardship.prd.md)、[`agent claim economy contract`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md) |
| liquid starter OC | 仅在 Agent 已存在后承担首次对话解锁这一受限用途。 | 不支付或延长 claim/upkeep，不能转为 restricted support、设施库存、材料或通用资源。 | 不形成持续对话、认领或设施补贴。 | [`Agent ownership and stewardship`](agent-ownership-and-stewardship.prd.md)、[`agent claim economy contract`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md) |
| facility / material inventory and records | 仅在授权设施/材料生命周期内记录 commission、服务、维护、回收和可审计 receipt；允许的 sink 由对应专题声明。 | 不是通用资源类型，不能自动转换为 `Electricity`、`Data`、claim support 或 liquid starter OC，也不自动获得转移/结算权。 | 设施库存/记录不能成为持续或无成本设施补给；补充、重置或新设施来源必须由专业合同另行授权。 | [`micro_depot contract`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md) |

来源、sink 或转换发生冲突时，以表中专业 owner 合同与新鲜证据为准，并由根 `README.md` 保持公开 claim envelope。

### 跨资源承诺、失效与重复提交边界

所有会影响资源、资格、设施库存或持续义务的玩家路径，都必须让玩家区分**预览**、**已接受的承诺**与 **receipt 支持的已结算结果**。该区分适用于直接操作、Agent 代办、恢复重试和重连后的继续；它不定义专业域的字段、锁定实现、时钟、价格或状态机。

| 产品阶段 | 世界允许发生什么 | 玩家能据此认为什么 | 明确不能被当作什么 | 失效、重试与审计边界 |
| --- | --- | --- | --- | --- |
| 预览 / 报价 / 推荐 | 读取当前权威事实并说明目标、预计来源/成本、主要 blocker、适用范围和下一步；不得单独产生扣减、预留、资格、排队顺位或持续义务。 | 当前可以比较或放弃候选路径；若继续，仍须提交权威动作。 | 已接受、已扣减、已锁定价格/资格，或未来必然成功。 | 输入、权限、世界状态、报价有效条件或信息新鲜度变化后，旧预览不能静默复用；必须重新评估或明确拒绝，并保留替代/恢复路径。 |
| 已接受的承诺 | 只有通过当前权限、资源、治理与反滥用校验的动作，才能按专业合同创建有限的 hold、排队或持续义务；结果必须说明作用对象、范围、授权来源、是否已有不可逆结果及下一次可决策点。 | 动作已被权威系统接收，以及哪些后果仍待完成或可被拒绝/解除。 | 已结算世界结果、自动续期、对无关资源/资格的授权，或对后续同类动作的无限保证。 | 过期、撤销、前置条件失效、重复提交或无法证明当前有效性时，不得生成新的 sink 或第二次效果；未结算部分按专业合同释放、拒绝或保持待决，不能静默改成欠费、补贴或其他资源类别。 |
| receipt 支持的已结算结果 | 权威世界结果、允许的 source/sink 变化及其责任/恢复边界已经生效，并可由对应专业域 evidence 追溯。 | 该范围内的资源/资格/义务后果已经成立，可据 receipt 进入下一步或适用的救济。 | 对历史 receipt 的重放以取得额外资源、再次结算、复活已到期资格，或抹除既有责任/历史。 | 纠错、退款、reclaim、争议或持续义务的违约只按各自专业合同处理，并保留资金来源、已发生后果与责任 provenance；产品层不得把它们承诺为统一自动退款或成功。 |

因此，玩家可以在承诺前比较和撤回候选，在已接受后读懂未决范围与下一步，并在结算后凭 receipt 继续、申诉或恢复；不能把建议、界面缓存、Agent 的计划、重连或历史 receipt 当成免费资源、既得资格或第二次执行授权。Agent 自动化与任何表现层同样受此边界约束，不能用“已计划”或“请求已送达”伪装为世界已经结算。

#### 条件报价与提交再校验合同

报价是读取时刻的条件性判断，不是对资源、设施容量、资格、价格、排队顺位或世界结果的预留。报价至少绑定其目标、输入、作用域与当时用于判断的权威条件；产品层不冻结版本字段、过期时钟或锁定实现，但不得把缺少这些实现细节误写成“报价仍然有效”。`electricity_after` 只说明该次报价中的可用资源扣减后余额；它不等同于 Agent 的电力运行 runway，也不能单独承诺生产会持续、不会进入临界/停机状态。两者都影响当前决定时，必须分别说明适用的余额、runway/风险和推荐的恢复或暂缓路径。

- **提交时重新裁决：** 提交必须针对当时的权限、资源、设施/地点、治理与反滥用条件重新校验，而不是消费旧报价。若任一条件已变化，系统只能以当前规则接受并产生一次可追溯 receipt，或原子拒绝且不扣减任何资源/不新建义务；不得静默沿用旧成本、旧风险、旧资格或把不足额变成隐藏欠费。拒绝或重新报价必须说明改变了哪类条件，并给出重新评估、等待、修复、改道或放弃当前候选中的适用下一步。
- **竞争、重连与 Agent 边界：** 两个玩家/Agent 对同一可争用事实取得的报价可以并存，但先被接受的动作改变条件后，后续提交没有优先权。重连、刷新或重复请求报价只会得到新的条件判断，不构成提交幂等键、自动续期或离线执行授权；Agent 取得报价后仍须拥有独立、有效且范围受限的动作授权才能提交。已经接受的动作的幂等、撤销和未决处理由其专业合同与 receipt 负责，不能由报价本身补足。

以下边界必须可用同一条代表性工业/服务路径验收：报价后插入一个已接受动作，使资源余额、设施可用性、权限或电力状态中的至少一项改变；旧报价的后续提交要么在当前条件下只结算一次并提供 receipt，要么原子拒绝且保留原有余额/义务。玩家能够区分“资源余额仍可支付”与“运行 runway/停机风险”，并能读到变化原因与下一步；把报价缓存直接当作预留、把失败转为欠费，或在重试时产生第二次 sink，均为失败。

#### BuildFactory 建设电力报价边界（冻结玩家承诺，不冻结实现公式）

`BuildFactory` 必须遵循条件报价合同。提交前，玩家或具备范围授权的 Agent 能比较 `build_now`、`prepare_inputs`、`move_to_site`、`restore_power_first` 与 `defer`；只展示专业合同当前真实支持且前置可证明的候选，每个可选项都说明目标/范围、追加成本与仍占用、预计结果/复查时延、失败或损失风险、可撤回性及推荐理由。报价至少说明 owner、site/location、factory kind/id、owner/location/chunk/existing-or-pending ID/recipe-fit 前置、当次 candidate/config/world revision，以及 owner-held electricity obligation 和每项 construction input obligation 的 kind、quantity、适用 ledger、before/after；若 surface 提供 aggregate，仍必须能展开到每项输入，不能隐藏 multi-input shortfall、重复收费或漏收。报价还必须说明建设/激活边界、首个工业目标关联、主要 blocker、下一次复查点和推荐动作，并提供可将本次条件判断与后续 submit/receipt 关联的稳定 quote identity 或等价 immutable context digest；产品层不冻结字段名或编码。报价只读，不产生 sink、hold、设施、排队顺位、激活或未来电力承诺；`electricity_after` 只表示按报价预计扣除后的余额，不表示 Agent runway、设施维护 runway 或不停机保证。

建设电力义务必须由适用专业 profile 明示为 `start-only sink`、`held-through-completion/arrival`、`boundary-revalidated conditional` 或 `non-reserved` 之一；未纳入 reservation chain 的部分必须标为 power-best-effort。建设成本、后续 `base_power_draw`/maintenance、配方电力与 battery runway 是不同信号，不能互相冒充。若 factory kind 会注册 PowerPlant，专业 profile 必须唯一选择建设完成或激活中的一个权威 output boundary；发电能力只能在该结果后生效，不能由报价或待建设状态提前入账。

在实现冻结前，成本/资源映射、power mode、建设/激活边界及权威 revision 必须回写到 [`world-runtime` industrial execution status and authority matrix](../../world-runtime/prd.md#industrial-execution-status-and-authority-matrix)：`world-runtime` 拥有 event/schema 与当前实现状态，M4 拥有 construction domain/profile，game/product 拥有玩家承诺，由 TPM 合流为单一 task truth；本条不选择其中任一实现，也不建立联合第二权威。该决定未记录前，任何 surface 不得把某一实现公式、字段或跨模块“已对齐”作为当前产品保证；无法证明的事实显示 `unknown/degraded` 并说明 authority 尚未冻结，不能伪造为 0、可支付或已预留。

提交必须绑定报价所示 candidate/config/world revision，并从新鲜权威状态重新校验 owner、site/location/chunk、kind/id、资源余额与授权。报价后竞争性状态发生变化时，只能按当前条件至多结算一次并产生 receipt，或原子拒绝且不产生任何新 sink、设施或义务；不得沿用旧成本、隐藏欠费、自动 top-up、overdraft、隐式 hold，或把失败重试当成新授权。重复提交、重连、Agent retry、snapshot restore 与 replay 只能重读同一结果，不能复制建设扣款、`FactoryBuilt`、激活、发电或奖励。

`test_tier_required` 至少覆盖：报价无世界效果且同一快照确定；正常建设、缺电、任一 construction input shortfall、owner/location/chunk/kind/existing-or-pending ID blocker 的正负样例；报价后资源/权限/地点漂移的 current-state revalidation 或 atomic reject；`start-only` 在接受后余额漂移时不改写旧 commitment，`held-through` 的 hold/consume/release 各至多一次，`boundary-revalidated` 在声明的 construction/activation boundary 重验，`non-reserved` 明示 best-effort，reservation mode 的竞争与 unknown 不 over-hold、不插队也不伪造 0；construction sink 与 recipe/runway/maintenance 的分离；PowerPlant 在 profile 声明的 output boundary 前不产出；Viewer 与 pure API 的逐项成本、mode、blocker、下一步、quote correlation 和 receipt 一致。本文不冻结绝对数值、runtime schema、队列、UI 布局或当前实现完成声明。

#### 在途工业任务的承诺、中断与结算

工业排程被接受后，不能继续沿用“报价可随时放弃”的语义，也不能把目标换向、停机或重连自动解释为取消。代表性配方任务必须让玩家在提交前读懂：接受时会消耗或占用的投入与能源、预期产出及副产物去向、形成结果所需的时间或阶段、是否存在取消/中止窗口，以及适用规则会保留、退回或损失什么。报价仍然只读；提交时只能原子拒绝且无新 sink，或创建一次可追溯的已接受工业承诺。

已接受任务必须保持可区分的 `已接受/待开始 -> 已开始 -> 进行中 -> 已完成 / 被阻塞 / 已中止` 产品结果；`被阻塞` 是等待恢复、继续或按专业合同转为终止的可决策状态，不是隐含完成。若专业合同允许开始前释放、撤回或过期，也必须将其表达为未开始的终止结果，不能伪装成已中止的在制任务。这里的分类不冻结 runtime 枚举或 UI 字段，但任何专业实现都不能把“请求送达”“目标已换向”“设施暂不可用”或“客户端已断开”伪装成开始、完成或中止。目标换向、Agent 打断、维护/权限/设施状态变化、重连、重复提交与事件重放均不得静默迁移旧任务、再次扣料、复制产出/副产物，或同时生成退款与完成结果。

取消或中止只在专业合同声明存在该能力时可用，并必须遵循当次任务适用的确定性处置规则：尚未开始的 hold 可以按规则释放；已经产生 sink、在制进度或世界效果的任务不得承诺全额自动退款。任何 salvage、部分返还、在制品保留、继续执行或安全停机都必须范围有界、只结算一次，并保留原投入、已发生结果和处置原因的 provenance；没有已实现且有证据的中止能力时，玩家只能看到真实可用的等待、修复、改道或重新规划路径，不能展示虚构的“取消”。

工业任务进入 `被阻塞` 后，必须收成一次可比较的恢复/退出决策，而不是只展示状态标签。玩家至少要看到：最早可归因的根因所在阶段/边、仍占用或已消费的投入与容量、当前保留/损失的在制价值、稳定窗口是暂停/清零/不适用、下一次权威复查边界，以及专业合同当前真实支持的 `继续/等待`、`修复`、`改道`、`降载或拆分`、`重新规划`、`中止/放弃` 路径。每个展示候选都必须逐项说明作用目标/范围、追加成本与仍占用、预计结果/复查时延、失败/损失风险、可撤回性及推荐理由；缺少任一字段的候选不得展示为可选路径。不存在安全路径时必须明确停止并给出下一次可决策点；不得用后台重试、自动改道、自动退款或“已恢复”标签代替玩家确认。恢复若仍属于同一因果计划，只能以同一 root 的 continuation 继续；若改变配方、设施、边或终端等产出因果，必须创建链接旧任务的新 revision/candidate，并从 `W=0` 重新计数。重复确认、重连、Agent 重试与 replay 只能返回同一阻塞/恢复处置，不得重复释放、扣料、产出、退款或里程碑。

中断后的玩家反馈至少回答：原任务及当前阶段、已经投入与仍被占用的价值、已形成或仍待形成的产出/副产物、保留与损失、对稳定产线候选进度的影响，以及当前真实可用的继续、等待、修复、改道、中止或重新规划动作。`game` 拥有这些选择的玩法取舍与平衡，`world-runtime` 拥有事件、状态、资源守恒、去重与 replay，Agent 拥有意图打断/换向的可解释 handoff，Viewer 与 pure API 拥有同一权威结果的表达，QA 拥有组合证据；产品层不规定取消 action、退款公式、队列算法、状态 schema 或 UI 布局。

#### 从采集/补充到首个可消费账本输入的 handoff

首个可消费账本输入必须沿一条可追溯的来源链形成：`extraction/replenishment → refinement → source-to-ledger handoff → first canonical consumable ledger input`。这里的“形成”不是把采集结果、精炼结果或一条推荐直接当作下游库存；只有来源/精炼效果已按专业合同结算、handoff 按适用的账本与物流规则完成、实际数量和损耗可对账，并取得当前阶段的适用性结论后，批次才可作为下游 canonical input。产品层只冻结这条玩家可理解的因果边界；这些权威边界由 [`M4 industrial resource flow contract`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md)、`world-runtime` 与相关 Recipe/Product/Factory 专业合同联合界定：M4 负责材料账本/类型标签、handoff 与领域适用性语义，world-runtime 负责 receipt/event/schema、转移/结算顺序与当前实现状态，Recipe/Product/Factory 合同负责各自 profile 的输入/输出适用性及能力声明；本节不新增字段或实现完成声明。

玩家可以在提交前比较当前专业合同真实支持的采集/补充、减量、换来源/地点、等待补充、先恢复电力、精炼至目标或最小合法量、调运/等待容量和延期路径，并读到来源、预计实际产出/损耗、成本、目标账本/用途、第一配方关联、主要 blocker 与下一次复查点；确认后仍须按当前权威状态重新校验。玩家不能把 preview、推荐、已接受、精炼中、来源账本中的库存或在途承诺当作已到达、适用或已消耗的 canonical input，不能直接写入/改写目标账本、绕过 owner/权限/规格/容量、按材料名称猜测适用性，或以客户端/Agent 重试复制采集、精炼、转移、到达和首个 input sink。没有专业合同支持的自动补充、免费转换、静默降级、隐式改道或全额退款不得作为可选动作。

玩家与 Agent 共享的只读结果面必须把以下语义状态分开表达（这些是产品语义，不冻结 runtime 枚举或 UI）：

| 读面状态 | 允许玩家据此理解 | 仍然不能据此理解 |
| --- | --- | --- |
| `preview` | 当前来源、精炼与 handoff 候选、预计数量/损耗、目的账本、成本、风险和复查点可比较；无世界效果 | 已扣减、已预留、已到达、已适用或必然成功 |
| `source-settled` | 采集/补充或精炼的 source effect 已由一次 receipt 结算；这是源效果结算，不是 M4 的目的账本/终端 `settled` 生命周期结算，实际产物与 provenance 仍可追溯并待交接 | 下游账本已有可消费库存、规格已适用或已产生下游进度 |
| `in-transit` | 已接受一条明确的 source→edge→destination handoff，来源 effect 至多一次，预计到达量/损耗/等待和占用可读 | 目的账本已 credit、下游已解锁或交付已完成 |
| `arrived` | 实际到达数量已进入声明的目的账本/有界 buffer，保留 parent receipt、批次与损耗 lineage | 到达自动等于适用、可独占消费、生产完成或终端交付 |
| `applicable` | 当前 root/阶段/配方/规格/owner/账本条件下，matching parent receipt 与 arrival 允许该批次成为首个 canonical consumable ledger input | 可以跨 root/阶段复用、绕过 join/容量，或提前产生第二次 sink/奖励 |
| `blocked` | 最早可归因 blocker（来源、精炼、电力、权限、路线、容量、到达、规格/证据或 owner）及真实支持的等待、修复、补充、重报价、改道、隔离、返工、返还、补偿或放弃路径可读 | 已完成、已适用、已退款，或可由后台重试自动解除 |

失败恢复必须保持同一因果链：报价/预览过期或提交前置漂移时原子拒绝并保留原状态；采集/补充或精炼失败时不扣未结算资源，玩家可减量、换来源、等待补充、补电或重新报价；handoff 因权限、路线、容量或目的账本失效时，可在专业合同支持下等待容量、保留/隔离、显式 return、重报价、改道或补偿，否则保持有界 `blocked`；到达后规格/证据不足只能重验证、等待或拒绝，不能以同名材料、旧 receipt 或自动降级解锁下游。已经发生的 sink、运输损耗、在制或到达效果不得被“恢复”抹除；每个释放、返还、返工、salvage、conversion 或 compensating receipt 都必须一次且有 parent provenance，重复提交、重连、乱序、snapshot restore 与 replay 只重读同一 disposition。

该 handoff 的守恒与反滥用边界是：来源数量、精炼转化、运输损耗、目的账本 credit、适用性 decision 与首个 input sink 分层对账；一条 source→edge→destination operation 至多产生一个有效 source effect 与一个目的结算，未知/不适用/过期/错误 root 的 decision 在首个 sink 前 fail closed；同一数量不能同时属于多个目的账本或 join；未支持的 split/merge/mix 不得把残余变成免费库存；任何新用途、奖励或稳定进度只能由相应 production/delivery/settlement receipt 触发。玩家和 Agent 不能通过反复 preview、拆小批、改变到达顺序、重复 arrival 或跨入口重试刷取材料、解锁、优先级、稳定窗口或奖励。

这条产品边界的组合影响必须保持明确：**world-first** 要求账本输入反映真实来源、精炼、运输与容量约束；**emergence-first** 允许玩家/Agent 在这些约束内自选来源、时机、路线和恢复，而不是领取预写的免费转换；**persistent** 要求批次、receipt、lineage 与 blocked/待决状态跨重连、恢复和 replay 延续；**auditable** 要求每次实际数量、损耗、适用结论、owner/账本、原因与处置可追溯；**extensible** 允许未来新增 extraction、replenishment、refinement、ledger 或 handoff profile 复用同一边界而不改变既有历史。玩家承诺只到首个 canonical consumable ledger input；更细的配方、数值平衡、字段、队列、物流/账本算法、WASM/runtime 实现和 Viewer 布局仍由专业 authority 决定。

本 handoff 的 Non-Goals 是：不规定采集、补种、精炼或运输的产率/价格/损耗公式，不新增或重命名 runtime/ABI/ledger 字段，不决定队列、公平性、UI 布局或 Agent 自动化策略，不把产品读面当作当前实现完成证明，也不替代 `game` 的玩法平衡、M4 的材料/账本合同或 `world-runtime` 的事件、状态、持久化与 replay 权威。

#### 原材料批次时效、质量漂移与仓储保管

材料到达目的账本不等于其质量永远有效。每个材料/产品 profile 必须声明自身属于 `stable` 或 `time/environment-sensitive`：`stable` 批次在等待、运输和有界 buffer 保管期间不因时间自动失效，但仍受新的权威规格、owner、账本或污染/损坏事实约束；`time/environment-sensitive` 批次则必须声明适用的有效性边界、保管/运输条件与重新验证点。材料批次、profile、质量/规格适用性与 custody 语义的专业 authority 是 [`M4 industrial resource flow contract`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md)；`world-runtime` 继续拥有事件、schema、权威状态、时间与当前实现状态，产品层只冻结玩家承诺，不成为第二套 schema authority。未声明或 legacy profile 在完成显式 admission/backfill 前一律视为 `unknown`，并在首个 input sink、WIP、稳定进度或奖励前 fail closed；不能由本段文字、材料名称、客户端缓存或 Agent 推断为 `stable`、可用或已完成 backfill。该分类只定义产品结果语义，不规定所有材料都要衰减。

材料在 source/refinement settlement 时必须保留可追溯的质量 provenance：批次身份、材料 profile、实际数量、规格/品质证据、owner/账本、结算与当前 custody 的权威锚点，以及与 parent receipt 的 lineage。运输、转入/转出 buffer、保管条件改变或重新验证都不能就地改写 parent receipt；只能在声明的 custody segment 上产生链接的状态/decision 或处置 receipt，并保留已发生的数量、损耗和责任。仓储容量、占用和保管义务属于真实的 batch/custody 状态，不能因 surface 显示、重连或“等待”而免费释放或隐藏。

`time/environment-sensitive` 批次只在 profile 声明的边界重新验证，例如 handoff 接受、transit launch/arrival、buffer admission、recipe input join/start 或某个权威 world-time checkpoint；不得由 Viewer 刷新、客户端计时或每次重复预览自行制造验证。每个边界都必须以当前权威时间、路线/保管条件、批次 provenance、目的账本与下游规格重新判断。边界证据缺失、条件漂移或结果无法证明时，批次为 `unknown`/`blocked`，在首个 input sink、WIP、稳定进度、需求减少或奖励前 fail closed；不能把未知写成零损耗、稳定、可适用或自动延期。

质量是叠加在既有 handoff lifecycle 上的正交 facet，不替换 `preview`、`source-settled`、`in-transit`、`arrived`、`applicable` 与 `blocked` 的批次流转身份：`arrived` 只证明 custody/到达，不证明质量可用；`applicable` 必须同时满足到达、当前规格/用途条件与质量 facet 为 usable。这里的 `usable` 不是新的 enum 或持久化 lifecycle state，而是由 canonical M4/Recipe/Product target profile 针对当前用途派生的 predicate；它仅在当前 profile 证明质量有效，或显式允许 `degraded` 用于当前用途时成立。`unknown`、`quarantine`、`expired` 在消费侧一律映射为 `blocked`；`degraded` 只有目标 recipe/阶段 profile 明示允许该质量用途时才可映射为 `applicable`，否则仍为 `blocked`。质量重验保留旧 decision 的 immutable 结果，并只通过 linked revalidation decision 改变后续消费判断，不能回写旧 handoff receipt 或把质量结论提升为新的到达结算。

玩家与 Agent 的读面必须区分：`blocked`（必要有效性或 custody 证据尚未成立，保留当前占用与复查点）、`degraded`（profile 证明质量下降但存在明确的受限用途或 at-risk 继续规则）、`expired`（有效边界已过，不取得新的消费资格）和 `quarantine`（因污染、条件违约或待复验而隔离，不能被下游消费）。`degraded` 只有在目标 recipe/阶段 profile 明确允许时才可进入新的候选；`expired` 或 `quarantine` 不得靠同名材料、降级标签、混批或换账本恢复。读面至少说明批次/阶段、质量原因、仍占用或已损失的价值、下一次权威复查点，以及当前专业合同真实支持的恢复动作；这些是产品语义，不冻结 runtime enum 或 UI 布局。

恢复与处置必须显式且有界：专业合同支持时，玩家可以比较 `revalidate`、`rework`、`salvage`、`return`、`discard`、继续隔离或重新取得合法来源；每个候选说明追加成本、保管/容量占用、预计质量/数量结果、交付或配方影响、失败损失、可撤回性和推荐理由。`revalidate` 只能从 fresh authoritative state 产生链接旧 decision 的新 decision；`rework`、`salvage` 或转换必须创建新的 batch/lineage，并披露实际投入、损耗与新用途；`return`、`discard` 和补偿只能按专业合同各结算一次。没有对应能力时保持真实 `blocked`/`quarantine` 或拒绝，不能自动退款、免费补发、静默修复、无限延期或把隔离物料伪装成下游库存。

每个 `batch + custody segment + revalidation boundary` 的质量判断和处置至多生效一次，并绑定 parent receipt、当前 root/阶段/配方用途及权威时间/状态锚点。边界前后发生漂移时，旧 decision 保持 immutable，由 profile 创建 linked revalidation/处置；重复 submit、arrival、拆小批、重连、Agent retry、乱序、snapshot restore 与 replay 只能重读相同结果，不能刷新有效期、复制 sink、重复释放保管容量、把同一数量同时分给多个 join，或以混批掩盖失效批次。拆分、汇合和跨账本转移必须分别复验每个 parent batch；任何由 rework/salvage/return 形成的新用途都必须由新的可追溯 receipt 触发。

该边界保持 **world-first**：配方输入反映真实时间、保管条件、规格与损耗，而不是稳定库存假设；保持 **emergence-first**：玩家在保管、调运、复验、返工和改换来源之间做有代价的选择；保持 **persistent**：批次年龄/条件结论、隔离、处置和 lineage 跨重连、恢复与 replay 延续；保持 **auditable**：质量证据、custody、处置、数量/损耗和 parent receipt 可追溯；保持 **extensible**：未来可增加稳定、时效、环境或污染 profile，而不改写既有 settlement 与历史结果。`game` 拥有 profile 的节奏、机会成本和玩法取舍，M4/Recipe/Product 拥有材料质量与适用性语义，`world-runtime` 拥有权威时钟、状态、事件、持久化和 replay，QA 拥有组合验证；产品层不替代这些专业 authority。

本节的 Non-Goals 是：不规定衰减/污染/温度/湿度/保质期的绝对数值、公式或仿真模型；不默认所有材料可腐败或必须保温；不新增或重命名 runtime/ABI/ledger 字段，不决定运输寻路、队列、公平性、价格、产率、损耗、返工收益、配给或 UI 布局；不把 `degraded`、`expired`、`quarantine` 写成当前 runtime 已实现或公开发布能力，也不承诺自动补货、退款、销毁或补偿。

`test_tier_required` 至少覆盖：`stable` 批次在等待/运输/有界保管中保持质量但遇权威规格或污染事实变化时重新判定；`time/environment-sensitive` 批次在有效边界内可一次成为 `applicable`，跨过声明边界后只能得到 `degraded`、`expired`、`quarantine` 或 `blocked` 中与 profile 一致的结果，且在首个 input sink/WIP/progress/reward 前不消费；缺失 custody、时间、条件或质量证据时 fail closed 并显示 primary blocker/复查点；`revalidate`、`rework`、`salvage`、`return`、`discard` 各自只产生一次、保留 parent provenance 和实际数量/损耗；拆分/汇合/转移不把失效批次混成适用输入；重复 submit、arrival、重连、retry、乱序、snapshot restore 与 replay 不刷新有效期、不复制 sink/处置/容量释放；玩家与 Agent 以及 Viewer/pure API 对质量状态、占用/损失、lineage、下一步和不可用原因保持同义。`test_tier_full` 的跨窗口、长时间与多阶段组合验证由 M4、runtime、gameplay、Viewer 与 QA 专业合同另行定义。

#### 多阶段工业流水线与中间品背压

单个配方任务完成不等于多阶段流水线已经连通。代表性流水线必须以有向无环的阶段关系声明 `上游阶段 -> 中间品边 -> 下游阶段`；每条边至少绑定可追溯的阶段/配方或能力版本、材料类别、来源与目的账本，以及适用的电力、物流和容量前置。替换决定产出因果的阶段、配方、设施或边后，属于新的候选流水线，不能继承旧候选的稳定窗口、未决资格或里程碑进度。循环生产若未来成为正式能力，必须由独立专业合同定义库存上限、终止与反套利规则，不能把隐式环当作普通流水线接受。

中间品必须区分**可用**、**已预留**、**加工中**与**已承诺但尚未到达**。下游只有在上游产出已由权威 receipt 结算，材料按适用物流规则实际进入下游可消费账本，并且该批次满足下游阶段声明的材料规格/品质适用范围后，才能取得开工资格；上游的计划、报价、已接受或加工中状态都不能提前生成下游库存、进度或里程碑。规格/品质适用范围是消费资格，不是产品层新增的数值公式：提交前的适用预览必须说明候选批次、可消费数量、缺口与主要不适用原因，提交时则按当前批次、配方和阶段条件原子重验；来源未知、证据不足或不兼容时只能在下游产生 sink 或进度前 fail closed，不能由 Agent、Viewer、模块元数据或同类材料名称自行推断为可用。一次中间品不能同时被多个下游承诺消费；拆分、混批或汇合必须声明边集合、确定性的分配/齐套规则，以及“只消费适用数量并保留其余批次”或“整笔原子拒绝”的明确策略，并分别验证各输入批次的适用性，不能依赖隐藏提交顺序、Agent 猜测、表现层缓存或把不同批次静默混合后视为合格。

需要两个或以上独立输入边的阶段必须为每个 candidate + recipe + executable cycle/batch 声明单一 canonical join identity，以及稳定有序的 required parent-edge/receipt set、各自数量/规格/版本和适用窗口。默认 **all-input atomic**：所有 parent receipt 必须已经结算并实际到达目标账本、在同一权威快照上适用且可独占消费，才能产生第一个 input sink、WIP、阶段进度或稳定计数；先到的 parent 保持在权威 ledger/有界 hold 的 `join_pending`，不能按材料同名、最新到达或客户端顺序猜测缺失 parent，也不能形成隐藏 partial kit。

只有专业 profile 明确支持 staged intake 时，才能在齐套前消费部分 parent；此时必须把每个已消费 parent 与 join identity 绑定，公开 remaining parent obligations、仍占用的 stage/buffer/hold、完成条件，以及缺失、无效、过期或换线 parent 的 release/retain/reject/rework/salvage/return 中真实受支持的处置。Late arrival、资格恢复或容量释放只重评 unmet parent set；parent reservation 只能归属一个 join 并至多释放/消费一次。Cutover 保留旧 join 与 parent lineage，新候选不得复用旧 receipt；retry、reconnect、到达乱序和 replay 不得重复 sink、拼出第二份 kit、单边伪造完成或增加稳定进度/奖励。

Staged intake 还必须显式防止**部分保留互等**：阶段图是 DAG 并不能证明 reservation 依赖无环，因为不同 join 可能分别持有已消费 parent、stage slot、edge 或 buffer，随后互相等待对方尚未满足的 parent/容量。若同一权威 allocation snapshot 发现这种 wait cycle，或无法证明当前依赖关系可继续收敛，系统必须在新增不可逆 sink、WIP 或稳定进度前 fail closed；不得以 retry、重连、网络到达顺序或“已接受”状态继续加 hold。该 profile 必须声明 cycle 的可读 root、当前主体自己的 held/unmet、下一次确定的重评边界，以及真实支持的 release、retain、defer、终止或专业补偿路径；每次重评只能产生一次 complete、继续等待、释放/终止或原子拒绝结果，不能无界延长 hold 或静默跳过仍有效的意图。已消费 parent 的处置仍须沿原 join lineage 单次结算，不能为打破互等而隐式回滚、混批、改道或同时退款与完成；retry、reconnect、恢复和 replay 必须重读同一 cycle disposition。

每条中间品边必须声明有界缓冲与背压结果：下游暂不可用或缓冲已满时，只能按专业合同保持上游未消费的投入、将已结算产出放入仍有容量的缓冲，或原子拒绝新的上游承诺。已经发生的加工、损耗或运输不能被静默丢弃、瞬移、自动改道、无限堆积或伪装成下游完成；任何溢出、返工、报废或 salvage 只有在专业合同明确存在时才能按其有界规则结算一次，并向玩家说明代价和恢复选择。

当两个或以上已经接受且仍有效的下游意图共同消耗同一阶段执行位、边吞吐或目的缓冲容量时，它们进入同一权威分配域；`已接受` 不等于每个意图都已经取得容量。每个裁决边界必须基于同一权威快照评估完整争用集，并为各意图产生**完整分配**、专业合同允许的**部分分配**、**延期**或**拒绝**之一；所有 hold 与实际消费之和不得超过可用容量，未获容量的意图不得产生隐性 hold、sink、进度或排队保证。裁决必须使用已声明的稳定依据和稳定身份解 tie，不能由客户端、网络到达、Agent 重试或表现层顺序决定；产品层不冻结权重、配额或队列算法。

容量释放、hold 过期/撤销、消费者失效或前置恢复后，仍有效的延期意图只能基于新的权威快照重评；未消费 hold 至多释放一次，已消费结果不能伪装成可重新分配容量。玩家必须能读到自己意图争用的是哪个阶段/边/buffer、已持有与未满足数量、完整/部分/延期/拒绝结果、适用的顺序依据、下一次重评条件，以及等待、释放、减量/拆分、改道、替代来源、暂停或重新规划中真实可用的动作；不得泄露其他主体的私密细节，也不得展示没有权威依据的队列位置或 ETA。同一意图的重连、重复提交与 replay 不能刷新优先级或复制 hold/效果；专业 profile 必须给出可观察的重评、过期、终止或重新规划边界，不能让仍有效的意图在容量反复可用时被无说明地无限跳过。

跨多个 tick/cycle 的工业路径只有在专业 profile 明确声明时，才能对玩家形成 end-to-end service window；否则必须标为 best-effort，不能把 recipe duration、transit `ready_at` 或某一段 hold 拼成到期保证。声明窗口时必须创建独立于 candidate/batch/join/output bundle 的 canonical window identity，绑定起始事件、目标 settlement、权威 start/end boundary，以及 stage execution slot、edge throughput、destination buffer 与 terminal capacity 中每项 mandatory obligation。每项容量必须明确属于 held-through-completion/arrival、到边界重新校验的 conditional reservation，或 non-reserved；未实际持有的未来容量不能被展示为承诺。

Service-window profile 必须声明 none/soft/hard 中适用的产品语义与 on-time/at-risk/late/expired 处置。提交和每个 stage finish、transit launch/arrival、buffer admission、terminal settlement 边界都按当前权威状态重验剩余窗口与 mandatory reservations：hard window 无法成立时在下一不可逆 sink 前延期/原子拒绝；soft window 可以显式 late 继续，但不得伪装 on-time。开始前 expiry/cancel 只释放未消费 hold 一次；已经产生 WIP、transit、buffer 或 production effect 后，只能按 profile 继续为 late、pause、hold/quarantine、reroute/requote、reduce、defer/reject 或专业合同支持的补偿，不能抹除既有 sink、自动退款、静默延长窗口或遗留 orphan hold。

Power-dependent profile 必须为每个 stage/cycle/branch/terminal obligation 声明 power mode：`start-only sink`、`held-through-completion/arrival`、`boundary-revalidated conditional` 或 `non-reserved`。End-to-end hard/soft service guarantee 只有在所有 mandatory power obligations 已按声明模式纳入 window 时才成立；否则必须明确为 power-best-effort，不能用 `electricity_after`、battery runway、预计电耗或当前余额冒充未来 reservation。Submit 与每个声明的 stage/transfer/buffer/terminal 边界只按该 mode 重验 owner-held power；hard shortfall 在下一不可逆 sink 前延期/原子拒绝，soft shortfall 明示 power-at-risk/late/blocked，已经发生的 WIP/receipt 继续服从既有 pause/hold/quarantine/reduce/compensation，不允许 overdraft、隐式续租、自动 top-up 或虚构退款。

当产线存在权威维护状态或计划维护窗口时，排产前必须提供只读的维护取舍，而不是把维护压力留到事后 receipt。玩家至少能比较专业合同真实支持的 `maintain_before_run`、`run_at_risk`、`reduce_load` 与 `defer`，并读到维护目标/作用范围、追加成本与仍占用、预计产出与复查时延、故障/停机或产出损失风险、可撤回性及推荐理由；稳定窗口或在制结果受影响时，必须明确其影响。若当前没有可验证的维护真值，surface 必须显示 `maintenance_not_tracked`，不能用电力余额、battery runway 或 `unchanged` 伪造安全继续。预览不推进维护、排程、tick 或资源账本；提交按当前维护状态重验。若在 submit boundary 维护真值仍为 `unknown`，任何会产生 irreversible sink、停机/稳定影响或交付承诺的候选都不得保持 selectable，必须 atomic reject 或保持有界 pending（不得产生隐式 hold）并给出下一次权威复查；只有专业合同真实支持且明确标注 `at-risk/unknown` 的路径才可继续展示，且 preview 仍无 authority。前置漂移原子拒绝并保留原任务/未结算状态，成功只产生一次可追溯维护/排程结果，不自动续期、退款或宣称交付；重连、重复提交与 replay 不得复制维护 sink、停机结果或稳定里程碑。

每次权威 power admission、sink、hold、release、revalidation 与 unmet disposition 必须绑定 root operation、owning revision/segment、cycle/branch 与 baseline expected power，并至多结算一次；它不是材料或 terminal settlement。诊断与 review 要把 power available/held/consumed/unmet/unknown 分开，缺证据时不能写成 `0`，并把 insufficient power 归为 primary root、下游积压归为 secondary。共享 power 争用继续服从同一权威 allocation snapshot、稳定顺序与单次释放/重评规则，不能 over-hold 或靠 retry 插队；玩家只看到有权限的 power scope、mode、复查边界与 reserve/best-effort/restore/reduce/defer 中专业合同真实支持的动作。

Renewal/extension 不能由重连、重试、Agent 推荐或 replay 隐式发生；必须从 fresh authoritative state 重新校验 owner、candidate、route/spec、容量与当前 lifecycle，并以链接 parent 的 continuation/new commitment identity 产生一次结果。Runtime replay 使用已记录的 world-time/order 与 expiry/renewal disposition，不能按恢复时墙钟重算；每项 expiry、release、renewal 与 fresh-snapshot re-evaluation 至多生效一次，也不能刷新共享容量顺位。稳定 `W` 与 terminal/delivery claim 只按已声明 window policy 计数，生产准时但 transit/terminal 迟到时保留 production receipt，却不能获得 on-time delivery receipt、奖励或里程碑。

工业性能与瓶颈诊断只能作为 read-only derived snapshot，不能由 Viewer 采样、客户端计时或运营面板成为第二套世界真值。每个 measurement/review window 必须绑定 canonical identity、candidate/recipe/factory/stage/edge/buffer/terminal config epoch、权威 world-time start/end boundary，以及 journal/state-root anchor；窗口只能在声明的 operation/window completion 或 checkpoint 关闭。Cutover 必须关闭旧候选窗口并为新候选开启新 identity，除非存在显式 handoff/conversion lineage，否则不得跨候选回填、合并或倒推指标；缺少 frozen plan 或 receipt 证据时显示 `plan_unavailable / unknown / not_tracked`，不能伪造为 `0`。

`plan baseline` 只能在权威 submit/allocation 边界从同一 canonical snapshot 创建一次，不能由 preview、quote、request、speculative hold 或表现层缓存生成。每个已经接受的 full/partial/deferred/denied 结果必须绑定 immutable baseline identity/revision、intent/candidate/config epoch、batch/join/output-bundle/service-window identity、snapshot/journal anchor，以及 requested、admissible、committed/allocated、executable、unmet/residual 数量和当时声明的 input/power/output/byproduct/stage/edge/buffer/terminal obligations；原子拒绝且未形成 accepted intent 时不创建 baseline。Deferred/denied baseline 记录当时计划结果但不代表取得容量、产生 sink 或进入 actual。

每个已经接受的流水线路径结果还必须在同一 submit/allocation 边界创建一次 immutable root operation identity；原子拒绝且没有 accepted intent 时不创建。Stage execution、input join、output bundle/branch、edge handoff/transit、buffer/terminal admission、service/measurement window、baseline revision、checkpoint、专业 product/module validation 与 production/delivery/compensating receipt 都必须同时绑定该 root、owning revision/segment 与直接 parent/child role。两个独立 accepted intents 即使 candidate、recipe、owner 与数量相同，也不能共享 root；fan-in/fan-out child 不能自行生成无 parent 的新 root，材料名称或 Viewer 推断不能替代权威链接。

Checkpoint continuation 与非因果 plan revision 保持同一 root，并创建链接的 child segment/revision；因果 cutover 创建 parent-linked child root/new candidate，不能继承旧 receipt、稳定 `W`、里程碑或 actual。每条 receipt 只能归属一个 root、owning revision/segment 和 child step/branch；缺失或冲突链接必须在 sink、credit、progress 或 reward 前 fail closed 为 `operation_identity_unavailable/unknown`。Root 在 terminal settlement 或显式 abort 后关闭，新的迟到效果只能服从既有 pending/compensating 合同，不能继续附着成普通完成；retry、reconnect、乱序、恢复与 replay 必须重读相同 root/child/finality，不能复制 sink、receipt、指标或奖励。

专业 Product/module validation 必须区分三层：quote/preview 只读且无 authority；validation decision 是 evidence-only child receipt；production/delivery settlement 才能产生材料 credit、用途结算或 finality。Validation decision 本身不得 sink/credit 材料、推进 `W`、减少需求、发奖或冒充 settlement。Profile 必须声明该 gate 对当前 root/branch 是 mandatory 还是 advisory、发生在 submit/start/completion 哪个边界，以及适用的 freshness/drift policy；advisory 结果不能被表现层包装成强制解锁或保证。

每个 decision 必须以 root+owning revision/segment+cycle/branch+validation epoch 唯一，绑定 exact parent batch/receipt、evaluated quantity、target stage/recipe/factory profile/config/spec epoch、validator/module release 与 proof/state/journal anchor，并得到 `applicable / not_applicable / pending / unknown / expired` 中一个权威结果。Mandatory gate 只有在 parent 已 settlement/arrival 且存在完全匹配的 `applicable` decision 时才能进入第一个 downstream sink；decision-only、缺失、负向、pending、unknown、expired 或 wrong-root/stage/batch decision 都 fail closed 为 hold/defer/atomic reject。Batch/spec/owner/ledger/route/profile/candidate/module drift 只使旧 decision immutable superseded，并按 profile 创建 linked child revalidation 或保持 pending/reject，不能改写旧 receipt、跨 root 复用或自动降级。Retry、恢复与 replay 重读同一 decision；output-bundle atomic/split policy 对每个 branch decision 至多消费一次。

已经 journaled 的 baseline 不能被后续库存/容量/profile 漂移、重连、retry、replay、reroute、renewal、reduce 或 replan 改写。专业合同支持调整时，必须在单一权威边界创建链接 immutable parent 的 child revision/new review，并把边界前后的 receipt/actual 分别归入其 owning revision；因果 recipe/factory/edge/terminal 变化继续触发现有 cutover/new candidate/`W=0`，非因果计划修订可以保留 candidate，但仍须分开 baseline 与 review。缺失或失效的 anchor 只能得到 `plan_unavailable/unknown`，不得用当前配置回填旧计划；重复提交、恢复或 replay 必须返回同一 baseline/revision/outcome，不能复制 hold、sink、指标或奖励。

Measurement checkpoint 只能由专业 profile 在声明的合法原子边界创建。每个 canonical checkpoint identity 必须绑定 operation/window、plan baseline/revision、权威 world-time/order、journal/state-root 与当时未决的 WIP、transit、buffer、terminal、hold 和 obligation；不能由 Viewer refresh、玩家反复打开 review 或任意 tiny window 生成，也不能切开尚未结算的 atomic join、executable cycle、output bundle/branch 或 source-to-destination handoff。边界暂不合法时，checkpoint 必须等待整个原子效果仍保持 pending 或完成后再关闭，不能生成半份 receipt、sink 或 actual。

Checkpoint 只关闭一次 immutable review segment，不追加、不重开，也不释放/续租 hold、改变稳定 `W`、发奖、结算交付或产生其他世界效果。Operation 尚未完成时，下一 segment 必须链接 parent checkpoint/review，并把边界上的 accepted backlog、join_pending、WIP、in-transit、buffer/terminal pending 与 holds 作为 opening state 延续，而不是重新计为 actual、重新接受或重新分配；默认保持同一 operation/baseline revision，只有显式 rebaseline/replan/cutover 才按既有 parent-linked revision 或新候选规则切换。缺失、冲突或不可验证的 checkpoint anchor 不得关闭窗口，只能显示 `checkpoint_unavailable / unknown / not_tracked`；retry、reconnect、恢复与 replay 必须返回同一 checkpoint、bucket 与 continuation disposition。

计划侧必须区分 requested、admissible、committed/allocated 与 executable units，实际侧必须区分 executed cycles、production-settled 与 delivered/terminal-settled；deferred、denied、unmet 与 residual 不能计入实际产出。同一 commitment/batch 在窗口边界只能落入一个 disjoint bucket：accepted-unstarted backlog、join_pending/held、active WIP、in-transit、buffer-held、terminal-pending 或 settled，并保留 lineage，不能在状态迁移时重复累计。Actual input sink、主产物/副产物、运输损耗、rework/salvage/return/compensation 与 unmet/residual 只从链接的 settlement receipts 对账；production 与 delivery 继续分开。

专业 profile 可以定义合法的 yield/loss/utilization 分类与 denominator 来源，但产品层不冻结公式或目标值。诊断至少能区分 stage slot 的 busy/idle/blocked、edge 已用与可用 throughput、buffer occupancy/peak/held、terminal held/admitted，并把 accepted-but-unstarted 与 WIP 分开。Bottleneck attribution 以 lineage 中最早的 canonical unsatisfied stage/edge/buffer/terminal blocker 为 primary root，派生缺料/积压标为 secondary；多个原因使用稳定 precedence 并保留 provenance，root 变化开启新的 attribution segment。重复 review、refresh、retry、reconnect、replay 或 compensating receipt 不得重复指标、改变 W/lease/receipt、发奖或产生世界效果；相同 journal、边界与 state-root 必须重建同一 snapshot。

改变展示或不影响产出因果的元数据不应误触发换线；改变阶段、配方、设施能力/版本、边目的地或其他消费资格与产出因果的变更，则必须创建新候选身份，并在一个权威 cutover 快照上逐项处置旧候选的**已预留**、**加工中**、**在途**与**已入缓冲**物料。边界前形成的 hold、在制品、运输承诺、批次与 receipt 继续绑定旧候选，边界后的新意图只属于新候选；材料同名、目的相同或表现层推荐都不能把旧状态静默重绑给新候选。新候选不得继承旧候选的稳定窗口、里程碑、排队资格、reservation 或 receipt。

每项旧状态只能取得一次可追溯处置：未消费 hold 可按专业 profile 保持、释放、延期或拒绝；加工中物料只有在旧前置仍有效且 profile 明确支持时才能排空/完成，否则进入暂停、终止、返工或 salvage 中真实受支持的结果；在途物料保留原边、目的账本、规格与已发生损耗，边失效时只能通过显式 return、hold、拒绝或新的 transfer/handoff 处理；已入缓冲批次继续保留旧 lineage，新候选仅能在显式兼容/迁移合同下重新校验后消费。没有专业处置能力时必须 fail closed，不能静默混入、重贴标签、瞬移、自动改道、同时退款与完成，或形成半旧半新的部分状态。若支持迁移/转换，必须产生一次链接旧 receipt、数量、损耗、变更原因与新候选身份的 conversion receipt。

换线前的玩家反馈必须比较当前真实可用的排空旧在制品、完成并交付、隔离、返工、salvage、放弃或延后换线；只有专业容量明确支持时才能提供并行新候选。反馈至少说明旧/新候选身份、当前阶段与 lineage、已消费/仍占用的投入和阶段/边/buffer 容量、保留与损失、交付延期、稳定窗口从 `0` 重启，以及下一复查点。重复换线、重连、重试与 replay 不得重复释放、交付、返工、salvage、退款或刷新优先级，也不能把旧产出或完成 receipt 计入新候选窗口/里程碑。

每条流水线的 terminal stage 必须声明有界的终端处置：进入 owner-bound 的有限 product ledger/buffer、进入具备明确 recipient/purpose 的交付承诺，或进入专业合同支持的 hold/quarantine。**生产完成不等于交付结算**：production receipt 只证明最终产物按实际数量与损耗进入合法终端账本或显式在途承诺；只有独立的 delivery/settlement receipt 才能减少需求、结算市场/服务/区域用途、发放交付奖励或完成终端里程碑。中间品边的 destination buffer、表现层的“已完成”标签或交易推荐不能隐式替代终端 owner、容量、资格与结算条件。

终端处置必须在排产前说明目的账本/recipient、可读的容量与已占用量、产品适用性/交付资格、生产后保留库存、交付是否仍待决，以及持有、预留容量、减量、改道、转本地用途、重报价或延期中真实可用的动作。终端容量不足、owner/recipient 失效、需求过期或路线不可用时，最终阶段只能按专业 profile 在产出 sink/稳定进度前阻塞或原子拒绝，或把已经合法生产的产物保持为有界、可追溯的未准入/待交付状态；不得无限堆积、免费销毁、自动转卖/降价/改道、静默没收，或把 production receipt 冒充 delivery receipt。稳定候选必须声明 terminal admission 是否属于 `W` 的有效周期条件：若属于，未准入周期不计进度；若只验证 production，则只能报告 production-stable，不能宣称已经交付或结算。

跨 stage/ledger handoff 的 preview、accepted/hold/allocation、in-transit/pending、released/rejected 都只是决策或待决证据，不能被 Agent、Viewer 或客户端提升为 settlement。每个 source -> edge -> destination operation 必须以同一身份和有序 journal 收敛为“有效 source effect + 至多一次 destination settlement”、明确 pending/hold，或原子拒绝/无效果之一；故障、乱序/重复到达、重连、retry 与 replay 不得造成来源已扣而目的丢失、目的免费 credit、pending 与 settled 并存，或重复下游 sink/奖励。若专业合同支持纠错/补偿，必须新增可追溯的 compensating receipt，不得修改历史 settlement 或用陈旧 receipt 解锁下游。

每个 recipe + factory capability 必须声明 canonical executable production unit，以及是否支持按合法 quantum/cycle 部分执行；材料批次、玩家请求量或共享容量的 partial allocation 都不能自动等同于可执行的 partial recipe。提交前与结果必须区分 requested、当前 admissible、committed/allocated、executable、实际 executed 和 unmet remainder。Full-only recipe 在输入、能源、阶段/边容量或终端准入不足一个合法执行单位时，只能在首个 sink 前延期或原子拒绝；不得静默降额/取整、部分扣料、生成半批产物/副产物、留下隐藏欠费或提前计入稳定进度。

只有专业 profile 明确支持 partial 时，才能按其声明的最小/合法单位、确定性量化/取整和 residual 处置执行；余量只能保持、返还、按合同发生有界损耗或原子拒绝，不能由客户端/Viewer 浮点取整、隐式生成/销毁材料或改变配方。共享容量的 partial 结果必须再次通过 batch admissibility，不足合法单位时仍是 deferred/denied。Settlement 必须按实际完成的 canonical cycles 记录输入 sink、主产物/副产物总量和 unmet/residual；副产物目的账本/容量不适用时按既有 hold/block/reject 规则处理，不能免费丢弃或把原请求量重复结算。批量大小本身不创建新候选；一个 `W` 计数单位只在合法执行 cycle 完成且满足已声明 output/terminal 条件时成立，将等价工作拆成小批不得增加稳定窗口、里程碑、容量优先级或奖励。

一个 executable cycle 同时产生主产物与一个或多个副产物时，必须创建单一 canonical output-bundle identity，绑定 candidate/recipe/cycle、完整输出 branch set、稳定 branch order、每条 branch 的数量、目的 ledger/edge/terminal route 与 parent receipt。默认 fan-out policy 是 **all-output atomic**：在第一个 output credit、production progress 或稳定计数前，必须从同一权威快照预检所有 branch 的 owner、适用性、容量与路线；任一 branch 不成立时，整个 bundle 只能延期、原子拒绝或进入 profile 明确支持的有界 hold/quarantine，不得先结算主产物再丢失/阻塞副产物，也不得静默改道、合并 branch 或把 production receipt 冒充 delivery。

只有专业 profile 显式支持 split fan-out 时，才允许各 branch 独立 settlement；profile 必须声明 parent bundle 完成条件、已结算/待决 branch、failed-leg residual 处置和对 stable/terminal 进度的影响。每条 branch 至多结算一次，未决 branch 的容量/资格恢复只从 fresh state 重评该 branch，不得重跑已结算 branch、再次扣输入或复制主/副产物、需求减少与奖励。玩家必须能读到 bundle 中每条 branch 的用途、数量、owner/destination、容量/资格、held/unmet 与 production/delivery 状态，并比较等待、扩容、减量、专业合同支持的改道/本地用途或延期；没有显式 split policy 时不能把部分成功包装成完成。

阻塞必须保留因果方向：surface 至少指出根因所在阶段/边、受影响的下游阶段、当前中间品数量与状态、上游是否因背压继续或暂停，以及真实可用的等待、补料、扩容、修复、改道、降载或重新规划动作。规格/品质判断至少要让玩家区分**适用**、**待验证/证据不足**与**不适用**，并指出不满足哪个阶段/边的消费条件；待验证或不适用不能伪装成普通缺料、在后台降级为较低规格，或自动转换、混料、报废。隔离、复验、返工、替代配方/去向、继续持有或 salvage 只有在专业合同明确支持时才是可选恢复动作，并须披露实际损耗、时间/容量机会成本与后续资格；不存在该能力时必须显示真实阻塞或放弃路径。派生的下游缺料不能覆盖根因，也不能把多个阶段压成一个无法定位的“生产失败”。玩家的取舍应围绕吞吐、缓冲占用、交付时机、批次适用性与恢复弹性；只让同一批次反复验证、加工、重连或重放而没有新增能力、选择或世界用途，不构成成长或新的稳定流水线里程碑。

同一批次跨阶段的 lineage 必须在持久化、恢复和 replay 后仍能关联各阶段承诺、边、中间品数量/预留、父级 receipt、规格/品质适用结论及其来源、实际损耗与 blocker。重复提交、Agent 重试、重连或事件重放对每个阶段至多产生一次 sink、产出与里程碑效果，也不能把未知/不适用批次重试成适用；已完成阶段可以按专业合同从已结算结果继续，但不得重新发奖、复制中间品或跳过尚未满足的下游前置。`game` 拥有阶段节奏、容量取舍、规格/品质带来的用途选择、返工收益和 anti-grind 平衡；`world-runtime` 与工业模块拥有图、账本、批次属性、适用性校验、预留、状态、守恒、去重与 replay 合同；Viewer 与 pure API 必须表达同一根因、适用结论、背压和下一步。产品层不冻结品质数值/公式、buffer 数值、配方、并行度、队列/图算法、runtime 枚举或 UI 布局。

#### 工业流水线生命周期到玩家决策的收口

现有工业合同分别定义了阶段、批次、物流边、配方版本、工厂能力和终端结算，但这些事实必须在产品层收束为一个可执行的玩家读法；玩家不能从 job、ledger 或 receipt 名称自行推断“现在能做什么”。对每个当前工业目标，正式玩家 surface 与 pure API 必须从同一权威快照提供以下四类产品结果（这是读面分类，不是新的 runtime 状态或字段）：

| 产品读面 | 必须让玩家看懂 | 不能误导成 | 当前决策边界 |
| --- | --- | --- | --- |
| `准备/待开工` | pinned recipe/capability、输入批次适用性、来源账本、物流边/容量、电力与终端前置，以及尚未消费的占用/缺口；工厂已建成但没有 active/作用域内 recipe 时必须明确显示 `factory-ready/recipe-missing` | 已取得产能、已开始或已计入稳定进度；`BuildFactory` receipt 不能冒充 recipe 已启用 | 选择补料/补证、等待容量或配方准入、修复前置、改用当前合法候选、延期或放弃；只有专业合同支持的动作可展示 |
| `运行中` | 当前 stage/WIP、已消费与仍占用的批次/容量、运输或缓冲位置、primary blocker 与下一复查点 | 已交付、已满足需求或必然按时完成 | 继续/等待、修复、改道、降载/拆分、重新规划或受支持的中止/处置；不得把重试当作新授权 |
| `产出待交付` | production receipt 的实际主/副产物、损耗、目的账本/终端、owner/recipient/资格与剩余交付前置 | delivery/settlement、需求减少、交付奖励或终端里程碑 | 等待/取得终端容量、交付、持有、改道/重报价或延期；没有 delivery receipt 时，只有 `production-only` profile 可按其稳定条件完成生产目标，声明 terminal admission/delivery 的目标仍不能完成 |
| `已结算` | 与当前目标匹配的 production/delivery/terminal receipt、实际结果、保留库存、稳定窗口与可打开的新用途 | 可以重复消费同一结果或重复获得成长 | 只在 profile 支持的范围内比较继续稳定、扩容/服务、合法换线或下一个目标；重复查看、重连和 replay 不产生第二次进度 |

每个读面至少同时说明 `factory/capability + recipe version`、关键输入批次/适用性、有效物流边、power/maintenance 风险、output bundle/副产物处置、`primary blocker`（无则 `none`）、已占用或已消费价值、`next_action`、`next_recheck` 与 `progression_effect`。任一权威事实缺失或漂移必须显示 `unknown/degraded`，不能以零成本、空缺口或“已完成”填充；`accepted`、`produced`、`buffer-held` 和 `terminal-pending` 都不自动打开下一个玩法目标。

`BuildFactory` 完成到第一次可排产之间必须存在一条明确的产品桥：建设 receipt 只证明工厂能力/设施结果，不自动授予 active recipe、原材料、物流容量或生产奖励。工厂处于 `factory-ready/recipe-missing` 时，玩家可比较当前专业合同支持的配方准入/启用、补齐输入与物流/电力前置、等待治理/证据、改用当前合法 recipe 或延期；配方准入/启用的 preview 只读，不产生 recipe 生效、输入 sink、排队、产出或稳定进度。只有取得权威的 active/作用域内 restricted recipe 结果，并在 fresh state 证明 factory fit、原料适用、物流、power 与 terminal 前置后，才可进入 ScheduleRecipe 的排程预览与提交；首次排产接受仍不等于首次生产，任何生产奖励/能力里程碑都必须至少等待一次 production receipt。目标随后只按 profile 声明的完成边界推进：`production-only` 还须满足其稳定窗口，terminal-admission/delivery 还须取得匹配的 delivery/terminal settlement receipt。缺少可用 recipe 或其准入证据时必须保持 blocked/unknown，不能自动挑选同名配方、把建设成本当作 recipe 解锁，或以“工厂已建成”打开下一目标。

当前目标必须按 profile 声明的完成边界判定：`production-only` 目标可在匹配的 production receipt 与稳定条件成立后完成，但仍须标明 `production-stable/undelivered`，不得减少交付需求或发放 delivery/terminal 奖励；声明 terminal admission/delivery 为因果条件的目标，只有其 purpose/recipient、数量与适用性由匹配的 delivery/settlement receipt 结算后才可完成。结算后若存在多个真实可达方向，玩家可以比较继续稳定当前能力、扩展/服务需求、合法换线或延期中的候选；候选的节奏、价值和数值取舍由 [`gameplay top-level design`](../../game/gameplay/gameplay-top-level-design.prd.md) 拥有，本产品层只要求每个候选说明第一动作、关键输入/工厂/物流前置、机会成本、主要风险、预计新选择或区域用途与回退路径。没有安全候选时必须明确停止并给出下一次决策点，不能自动生成新目标或把重复生产包装成成长。

若原料适用性、工厂能力、配方授权、物流边/容量、电力、终端 owner/资格或交付条件阻塞当前目标，目标保持 blocked 并沿同一 root 显示最早根因、仍占用/已损失价值和真实可用的恢复动作；不得静默换配方、改道、降规格或转移终端。改变 factory、recipe、输入来源、边或 terminal purpose 等产出因果内容时，必须建立 parent-linked 新目标/候选并从 `W=0` 重新计数；同一计划的修复只保留 root continuation。每次处置、交付或目标完成至多生效一次，重连、Agent retry、重复提交和 replay 只能返回相同结果。

该收口组合了 **world-first** 的真实材料/工厂/物流约束、**emergence-first** 的有代价路线选择、**persistent/auditable** 的目标与 receipt lineage，以及 **extensible** 的 profile 演进；它不新增 runtime schema、队列、配方公式、价格或 UI 布局。`game` 负责目标节奏、成长/反刷与候选平衡，M4 与 `world-runtime` 负责 lifecycle、receipt、守恒、幂等和 replay，Viewer/pure API 负责同义表达，QA 负责组合证据。

该闭环的 `test_tier_required` 至少覆盖：同一三阶段 `join → stage → transit → buffer → terminal` 路径下四类读面的正向样例；缺料、批次不适用、工厂/配方不兼容、边容量不足、停电和终端失效各自保持正确 primary blocker 与下一动作；同一 production receipt 在 `production-only` profile 中可完成生产目标但保持 undelivered 且不减少交付需求，在 terminal-admission profile 中则必须等待匹配的 delivery/settlement receipt 才完成目标并打开交付型下一候选；因果换线从 `W=0` 开始，非因果恢复保留 root continuation；Viewer 与 pure API 的 `next_action`、`next_recheck`、`progression_effect` 同义，重复提交、重连、乱序、Agent retry、snapshot restore 与 replay 不复制 sink、交付、奖励或目标完成。

#### 配方候选发现与选型（补齐 `factory-ready/recipe-missing` 到首次排产的产品缺口）

现有产品路径已经要求工厂建成后不能把建设 receipt 当作配方已启用，但还缺少“玩家如何找到值得比较的配方”这一段。`factory-ready/recipe-missing` 不能把整个 recipe catalog、同名版本或 Agent 推荐直接变成可排程列表；否则玩家看不出当前工业目标为什么需要某个配方，也无法区分“没有候选”“候选被前置阻塞”和“事实未知”。本节只定义候选发现、比较和选择的玩家语义，不新增配方生命周期状态、解锁树、runtime schema 或数值平衡。

配方候选是从当前主目标和同一权威快照派生的只读产品结果，而不是新的世界实体。每个候选至少绑定：当前目标/用途、recipe version 与 authority/scope、factory capability fit、所需输入批次及适用/缺口、物流边或 path/capacity、power/maintenance obligation、主产物与副产物的目的地/容量、首个可验证成果、机会成本、主要风险、下一次复查点和推荐理由。候选读面只使用当前有效的 `active` 或作用域内 `restricted` recipe；`pending_validation`、`validated_pending_admission`、`retiring`、`retired` 或无法证明 authority 的版本可以作为解释性 blocker 展示，但不能进入可选择排程。相同 recipe name 不合并版本、来源或历史 receipt。

候选发现至少收敛为三类产品结果（不是 runtime enum）：`candidate_available`（当前前置可证明且可进入排程预览）、`candidate_blocked`（版本/准入、工厂适配、原料规格/品质、物流/容量、电力/维护或终端条件中至少一项明确不足）和 `candidate_unknown`（关键 authority、批次 provenance 或复查事实缺失）。玩家可以比较 `inspect_candidate`、`prepare_inputs_or_route`、`wait_for_admission_or_recheck`、`use_another_legal_candidate` 与 `defer` 中当前真实支持的动作；没有安全候选时必须给出 `no_candidate_reason`、所需前置和下一次决策点，不能自动挑选同名配方、静默降级、把缺料伪装成“暂无任务”或由 Agent 代替玩家确认新的资源/权限承诺。

选择候选只产生一个可回看的、无世界效果的 `recipe_selection_preview`：它说明该候选如何服务当前目标、第一步投入/占用、预计产出与副产物去向、未满足前置、失败/损失风险、稳定窗口与交付影响，以及可回退的等待/改用/延期路径。选择不创建 recipe activation、hold、input sink、队列、物流 reservation、产出或稳定进度；只有玩家或具备对应范围的 Agent 明确确认后，`ScheduleRecipe` 才能在新鲜快照上重新校验并固定 recipe version/candidate identity。报价后任何产出因果事实漂移都必须重新发现/重报或原子拒绝，不能沿旧选择自动换配方。

玩家反馈必须回答“为什么推荐这个配方、当前缺什么、投入后先获得什么、失败后怎么恢复、何时再看一次”。成功选择后打开排程预览，不等于排程已接受；配方选择、排程接受、首次 production receipt、稳定里程碑和 delivery/terminal settlement 继续保持分层。Viewer、pure API 与 Agent 必须从同一快照给出同义的候选、blocker、机会成本、`next_action`、`next_recheck` 与 `progression_effect`；重连、重复查看、Agent retry、snapshot restore 和 replay 不复制选择、预留、排程或奖励。`game` 负责候选的目标价值、节奏与平衡，Recipe/Factory/M4/runtime 负责目录、授权、适配证据与权威版本，Viewer 负责表达，QA 负责组合验证。

该缺口的 `test_tier_required` 至少覆盖：工厂建成但无候选、单一可用候选、多个候选可比较、仅有 blocked/unknown 候选、同名不同版本、候选在预览后发生准入/原料/物流/电力/终端漂移；只有 `candidate_available` 能进入 `ScheduleRecipe` 排程预览，最终提交仍须在新鲜快照上重新校验，其余结果不产生 sink/hold/queue/W/奖励；选择预览保持只读，提交后只固定一次当前 recipe version，失败提供补料、补证、改路、改厂、等待或延期中的真实动作；重复查看、重连、retry、restore 与 replay 不自动选择、不复制效果，Viewer/pure API/Agent 对候选与下一步保持一致。

#### 代表性配方执行档案（把分散规则收成一条可验收工业链）

候选字段齐全不等于一条工业链已经设计完整。每个作为产品验收基准的代表性配方，必须提供一份**配方执行档案**，把同一 candidate 下的工厂能力、原材料批次、物流边、合法执行单位、输出分支和终端用途收成一个完整声明；任何一项只能从别处“猜到”或由 surface 临时拼接时，该档案为 `incomplete`，不得用于宣称流水线可排程、可稳定或可交付。档案是跨专业合同的只读投影与验收夹具，不是新的 runtime schema、recipe catalog 或第二套材料/物流权威。

最小代表性档案必须同时回答以下问题：

| 决策面 | 最小完整声明 | 玩家由此能判断什么 |
| --- | --- | --- |
| 工厂与配方 | pinned recipe version/authority、factory capability fit、作用域与当前 lifecycle state | 这座工厂为何能执行该版本，以及准入/退役或能力变化是否会阻止开工 |
| 原材料与齐套 | 至少两条 required input edge；每条绑定来源 ledger、batch/quality/custody 适用结论、数量与 join policy | 哪些原料已可消费、哪些仍缺失/未知，以及 full-only 或 staged intake 会占用什么 |
| 物流与容量 | 每条跨地点输入/中间品边的 effective path、预计实际到达量/损耗、吞吐或 buffer obligation 与复查边界 | 是立即调运、减量、换来源/路径还是等待容量，且“已发出”不等于“已到达” |
| 执行与能源 | canonical executable cycle/quantum、full/partial policy、stage capacity、power mode 与预计时间/风险 | 本次究竟承诺多少、哪些资源会在何时变得不可逆，以及不足一个合法单位时为何不能产出半批 |
| 产出与去向 | 一个 output bundle 内的主产物和每个副产物 branch、owner/destination、atomic 或 split fan-out policy | 某个副产物去向失效时是整批阻塞还是只保留未决 branch，不会出现主产物成功而副产物消失 |
| 终端与成长 | production-only 或 terminal-admission 完成边界、terminal owner/capacity/purpose、首次成果与后续目标 | 一次生产 receipt 是否只证明产出，何时才完成交付、减少需求、获得里程碑并打开下一选择 |

代表性最小样例固定为“一座具备目标 capability 的工厂 + 一个 active/restricted 配方版本 + 两类独立原材料 + 至少一条有损或有容量约束的物流边 + 一个主产物与一个必须处置的副产物 + 一个终端用途”。这里冻结的是覆盖形状，不冻结材料名称、配比、产率、价格、绝对容量、tick 数或字段编码。档案必须引用 M4、Recipe/Factory 与 `world-runtime` 的当前权威事实；缺少 authority、批次 provenance、路径、power mode、output policy 或 terminal boundary 时显示对应 `unknown/blocked`，不能用默认零值、同名材料、最近路径或“推荐配置”补齐。

玩家在同一档案上至少可以比较 `schedule_declared_cycle`、`prepare_or_source_missing_inputs`、`transfer_or_wait_for_capacity`、profile 支持时的 `reduce_to_legal_unit`、`repair_power_or_factory_fit`、`resolve_output_destination` 与 `defer`。每个候选必须说明即时收益、仍占用/将消费的价值、主要失败成本、对稳定窗口和交付的影响、下一次权威复查点与推荐理由；确认只固定所选 candidate/cycle 的一次承诺，未选路径不取得 hold、吞吐顺位、input sink 或稳定进度。任何改变配方版本、工厂能力、required edge、输出 branch 或 terminal purpose 的修复都是 parent-linked 新 candidate，并从 `W=0` 开始；仅补齐同一 candidate 的原料、容量或电力前置才保持 root continuation。

失败与恢复必须用这份完整档案做守恒判断：任一 required input 不适用、join 未齐套、effective path/容量不足、power/factory fit 漂移或 mandatory output/terminal 失效时，在首个不可逆 sink 前延期或原子拒绝；已经存在 WIP、在途或已结算 branch 时，只能沿 profile 支持的等待、释放、改道、隔离、返工、return、salvage 或终止路径各处置一次。不得静默换配方、把 full-only 降为 partial、让同一批原料加入两个 join、自动补货/退款，或在 output branch 失败时丢弃副产物并保留完整奖励。

该档案的 `test_tier_required` 至少以同一 deterministic fixture 覆盖：两类输入齐套后只启动一个合法 cycle 并产生一次 bundle；其中一类 input 为 `unknown/not_applicable` 时无 sink/WIP；物流满时保持可读 blocked，容量释放后仅重评未决 edge；power 在报价后不足时原子拒绝或按声明 mode 延期；mandatory 副产物 destination 失效时遵守 atomic/split policy；production-only 与 terminal-admission 分别只在各自声明边界完成目标。Viewer 与 pure API 必须从同一档案读出 factory/recipe、两类输入、path/loss/capacity、cycle、bundle/branch、terminal、primary blocker、机会成本、`next_action` 和 `next_recheck`；重复提交、重连、arrival reorder、Agent retry、snapshot restore 与 replay 不复制材料消费、hold、产出、交付、稳定进度或奖励。`test_tier_full` 再扩展到三阶段、三个以上输入/输出分支、cutover、长期持久化恢复和跨窗口 lineage。

#### 配方生命周期、版本兼容与受控退役

配方不是可被客户端或 Agent 就地修改的库存标签，而是具有来源、作用域和版本身份的受治理工业能力。配方生命周期使用六个且仅六个产品状态：`pending_validation`（待验证）、`validated_pending_admission`（已验证待准入）、`active`（已生效）、`restricted`（受限生效）、`retiring`（退役中）和 `retired`（已退役）。只有 `active` 或在声明作用域内的 `restricted` 版本，且当前授权、工厂能力、原材料规格/品质、物流路径、电力与终端前置均满足时，才能成为新的生产候选；提案、模拟结果、同名配方推荐或“已下载”状态都不产生预留、输入 sink、产出、稳定窗口或交付资格。

状态转换必须由权威条件驱动，不能由客户端缓存、Agent 推荐或重连隐式触发：

| 状态 | 进入条件 | 允许的下一状态与条件 |
| --- | --- | --- |
| `pending_validation` | 新配方或因果变更版本缺少完整确定性验证、来源/规格证据或 profile 证明 | 证据完整后进入 `validated_pending_admission`；提案撤回、验证失败或不可补足时进入 `retired`；不得直接进入 `active`。 |
| `validated_pending_admission` | 验证完成，但治理准入、作用域或激活授权尚未成立 | 当前授权明确激活后进入 `active`，仅有边界授权时进入 `restricted`，拒绝/撤回/授权失效时进入 `retired`。 |
| `active` | 当前治理授权、版本激活和必要专业证据均成立 | 作用域收缩或安全限制进入 `restricted`；已有 successor、撤销或计划退出的明确决定进入 `retiring`。 |
| `restricted` | 已生效版本被限制在明确的 owner、区域、用途、工厂或时间作用域内 | 新鲜授权扩大作用域可回到 `active`；继续收缩或退出进入 `retiring`；截止边界完成后进入 `retired`。 |
| `retiring` | `active`/`restricted` 版本收到明确的撤销、到期、替代或受控退出决定 | 完成声明的 cutoff 与存量处置后进入 `retired`；只有 cutoff 前取得新的权威授权，才可回到 `active`/`restricted`，不能靠旧 receipt 复活。 |
| `retired` | 拒绝、撤回、退出完成或 cutoff 到达 | 该版本是终态；只能创建新版本/新候选，不得重新激活或恢复旧排程资格。 |

状态与工业任务的动作边界如下；`accepted 未产生首个不可逆 sink` 不等于已经取得配方资格，WIP/在途/buffer 也不能因状态转换自动换绑新版本：

| 状态 | 新排程 | `accepted` 未产生首个不可逆 sink | 已有 WIP / 在途 / buffer |
| --- | --- | --- | --- |
| `pending_validation` | 不可选择，只能查看预览 | 保持无效果待决或原子拒绝，不产生 sink | 在下一不可逆边界前暂停；保留 lineage，仅可走 profile 明确支持的 hold、return 或 quarantine。 |
| `validated_pending_admission` | 不可选择，等待准入 | 保持无效果待决、重新报价或原子拒绝 | 不新增生产/交付效果；按 profile hold、quarantine 或 return，不能伪装为已准入。 |
| `active` | fresh state 满足前置时可选 | 重新校验后只启动一次 | 按 pinned version 继续，沿既有 lineage 结算。 |
| `restricted` | 仅在声明作用域内可选，越界拒绝 | 作用域内可重验并启动；越界只能 hold、重报价或拒绝 | 仅在作用域内继续；越界按 profile hold、quarantine、return、rework 或 terminate。 |
| `retiring` | 不可选择，不自动推荐或切换 successor | hold、重报价或拒绝；切换 successor 必须显式确认并新建 child candidate | 仅 profile 允许时 drain/finish；否则一次性进入 hold、quarantine、return、rework、salvage 或 terminate。 |
| `retired` | 不可选择 | 原子拒绝且无效果；successor 必须是新候选 | 不产生新效果，保留历史并仅按既有 profile 处置，不得复活旧版本。 |

每个已接受的工业任务必须固定一个不可变的配方版本/能力身份，并把它与 factory capability、输入批次、物流边/路径、输出 bundle 和 terminal purpose 关联。改变输入/输出/副产物、规格或品质适用范围、阶段关系、能源/物流前置、工厂能力或终端资格等产出因果内容，必须创建新的配方版本和 parent-linked 候选；新候选从稳定窗口 `W=0` 开始，不能继承旧候选的 receipt、reservation、里程碑、排队优先级或实际进度。仅改变不影响产出因果的说明/展示元数据时，才可保留原生产身份；无法证明为非因果变更时，按新版本处理并 fail closed，不能靠名称相同或当前缓存猜测兼容。

配方兼容不是“材料名称相同”或“有一座工厂”即可。提交前的只读预览至少让玩家看见配方版本的来源/授权作用域、输入与产出/副产物用途、匹配的 factory capability、每条必需物流边或路径、规格/品质适用结论、power/容量/terminal 前置、六状态之一和下一次复查点。预览状态必须与上表及 SC-24 使用同一组 `pending_validation`、`validated_pending_admission`、`active`、`restricted`、`retiring`、`retired`；`not_licensed`、`incompatible`、`expired`、`unknown` 只能作为 blocker/authority reason，不能伪装成新的生命周期状态或可选择的“降级替代”。

这些事实必须收成一条玩家可执行的产品循环：`查看配方状态 → 预览工厂/原料批次与品质/物流路径与容量/终端适配 → 在新鲜排程、等待/补证或复验、改用当前合法候选、处置旧任务与延期中做选择 → 提交时按当前权威状态重验 → 读取运行/阻塞/退役处置结果 → 进入下一个工业目标`。每一步都必须说明 primary blocker、已占用或已消费的价值、当前选择的机会成本、对稳定窗口 `W` 的影响与下一次权威复查点。任一环节只有一个合法动作时仍须说明原因；没有安全动作时必须 fail closed，不得用 Agent 推荐、同名 successor 或“自动优化”代替新的授权和玩家选择。

配方状态或前置在报价后漂移时，系统只能按当前权威版本重新评估并产生一次 receipt，或在首个不可逆 sink 前原子拒绝/保持无效果待决。至少以下情形必须 fail closed：版本身份或授权范围缺失；输入规格、owner、账本、路线、容量、power 或 terminal 资格不匹配；旧版本已经退役且没有当前有效的显式兼容/排空 profile；候选引用了错误 root、批次、stage 或 module/provenance。失败必须保留原任务/已结算历史的 lineage，说明最早 blocker、已占用/已消费与下一步；不得自动换配方、免费转换材料、静默降低规格、伪造退款或以“已接受”代替生效。

退役只禁止新任务继续取得该版本的生产资格，不能追溯删除已结算历史。退役时，新的排程必须拒绝并推荐当前有效版本或等待重新授权；已经 accepted 但未产生不可逆 sink 的任务按 profile 释放/保持/重新报价，已经产生 WIP、在途或 buffer 的任务只能完成旧版本、隔离、返工、salvage、return、改道或终止中真实受支持的一条处置路径。若存在兼容 successor，仍须在同一权威快照完成 fresh revalidation，并由玩家或具备范围授权的 Agent 明确确认；转换必须创建链接旧 receipt、实际数量/损耗与新候选身份的单次 handoff/conversion receipt，稳定窗口重新计数。旧版本的重连、重试、回放或陈旧批准不能重新激活、复制产出或刷新容量顺位。

该边界同时服务五项产品原则：**world-first** 让可用配方由真实授权、材料和设施能力决定；**emergence-first** 允许玩家在约束内选择来源、版本、路线与退役处置，而不是领取免费转换；**persistent** 让版本 pin、lineage、pending 与退役处置跨重连/恢复/replay 延续；**auditable** 让提案、准入、版本变更、兼容结论和单次 receipt 可追溯；**extensible** 允许未来增加配方/工厂 profile 而不改写既有批次和历史。配方治理不直接发放原材料、产能、库存或市场权利，也不规定配方比率、授权角色、版税、兼容算法、runtime schema 或 UI 实现。

配方生命周期验收至少包括：

- 一条提案→验证→准入→生效→退役样例；生效前无世界效果，玩家可读每个状态、阻塞与下一步。
- 同名但改变产出因果的两个版本；旧任务固定旧版本，新候选 `W=0`，两者不共享 receipt、reservation、进度或奖励。
- 授权、输入规格、factory fit、物流/电力/容量或 terminal 证据缺失/漂移的负例；首个 sink 前原子拒绝或无效果待决，且显示 primary blocker 与复查边界。
- 退役同时存在新排程、未开始任务、WIP/在途/缓冲批次的样例；新排程不可选，存量任务只能按 profile 取得一次明确处置，兼容 successor 需显式确认和 parent-linked conversion。
- 重复 submit、arrival、重连、Agent retry、snapshot restore 与 replay；同一版本最多产生一次 sink/产出/receipt，旧版本不能复活，Viewer 与 pure API 对状态、兼容结论、机会成本和下一步保持同义。

### 世界宪法级产品不变量

- 玩家通过目标、Agent、地点、设施、配方、关系与治理等受支持动作获得间接战略能动性；资源变化必须来自被授权的 source/sink 因果链，不能凭空生成或绕过成本。
- 每个权威行动都必须经过规则与权限校验并产生可审计后果；玩家能够读懂 target、action、cost、blocker、result、next decision 与 recovery，不靠隐藏状态猜测世界为何变化。
- 社会关系、组织、市场与制度可以在权限、治理和 anti-abuse 边界内产生有限涌现，但不会因此获得绕过权威规则的能力。世界保持持久、开放式演化，不要求强制终局。
- 系统性危机只能在受治理、可审计的 containment 边界内限制扩散并保护基本连续性；恢复必须留在同一权威世界时间线中，由玩家、Agent、组织或区域项目按既有资源、权限、治理、反滥用与申诉边界推进。containment 或恢复不得 reset 世界、追溯改写已确认因果、跳过申诉、选择性 bailout，或把局部运维/管理员操作包装成世界恢复；受影响主体必须能读到触发事实、作用范围、当前限制、恢复/申诉路径与下一步。
- 玩家前台始终围绕一个当前主目标：系统基于权威世界状态给出可达推荐与“继续”路径，玩家只在阶段性方向、主动换向或实际影响共同资源/权限的事项上作出必要选择。目标作用域、canonical 转译、资源/权限校验、共同治理、反支配与审计是后台护栏，只在实质改变当前选择时以原因和替代路径出现；不得把它们扩张为逐动作的表单或确认负担。
- 当前不支持的细粒度请求必须转换为 canonical 可执行替代动作；没有安全替代时明确停止并说明下一次可决策点。具体规则、确定性执行、Agent 行为与网络/治理合同分别由 [`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md) 与 [`doc/p2p/prd.md`](../../p2p/prd.md) 维护。

## 3. 权威与冲突处理

| 产品层拥有 | 专业域权威 |
| --- | --- |
| 玩家目标、核心循环的产品结果、成长、资源压力与跨域世界不变量体验 | `doc/game/prd.md` 拥有玩法规则、moment-to-moment loop、数值平衡和专题验收；`doc/world-runtime/prd.md` 拥有权威验证、确定性状态与审计；`doc/world-simulator/prd.md` 拥有 Agent/LLM 与交互模拟合同；`doc/p2p/prd.md` 拥有网络、共识与治理技术边界 |

产品层不用新细则或数值静默改写专业域权威。跨域冲突由 `producer_system_designer` 协调，并按受影响合同邀请对应的 `gameplay_designer`、`runtime_engineer`、`agent_engineer` / `world-simulator` owner 或 `blockchain_ops_engineer` / P2P owner 形成显式裁决；不得只以 gameplay 判断覆盖执行、Agent、网络或治理专业结论。

## 4. 路线图

1. 首局可读：目标、动作、阻塞、反馈和下一步可见。
2. 后引导承接：首局进入可持续的阶段目标与成长压力。
3. 世界参与：个人行动、Agent 和区域系统在一致规则下产生长期影响。
4. 成熟世界成长：小规模玩家通过可恢复能力、区域服务与有限区域影响形成独立 leverage，而不是被迫依附强组织或重复 grind；文明尺度共同项目保持自愿扩展。

## 5. Done：成功标准与验收

- SC-1：玩家在首局可识别当前目标、可执行动作、行动代价与下一步。
- SC-2：核心循环完整呈现行动接受、推进、阻塞、反馈和结果。
- SC-3：FirstSessionLoop 之后存在可达的 PostOnboarding 目标、压力与承接。
- SC-4：世界规则、资源消耗与玩家结果可映射到专业 PRD-ID 和验证证据。
- SC-5：玩家的间接战略动作具备授权资源因果、权威校验和可审计后果，界面或接口可读 target/action/cost/blocker/result/next/recovery；涌现关系与组织不绕过权限、治理或 anti-abuse。未支持细粒度请求的单个或多个 canonical 替代可比较其目标/范围、成本、风险、预计后果、可撤回性与推荐理由；确认前无世界效果，新增承诺或不可逆后果必须明确授权，拒绝、过期、权限变化、重连或重试不会静默执行、继承越界权限或产生重复结果，没有安全替代时明确停止。
- SC-6：代表性间接控制流程证明玩家意图进入 Agent/策略决策，经权威规则与资源校验产生世界后果，并返回可解释结果与可执行的打断、纠正、下一步或恢复动作；任一专业域的局部 green 不能替代组合闭环。
- SC-7：同一物理行动在 gameplay、runtime、Agent 与 Viewer 的粗粒度/表现映射中保持距离、顺序、成本和持久化结果一致，不产生第二条时间线或表现层真值；权威时间线在没有直接玩家输入时仍按当前世界规则继续推进，不冻结具体 tick 时长。
- SC-8：同一条代表性 Data 路径可端到端证明：预览不授予访问或收益；owner、recipient / 使用主体、purpose、scope 与授权状态可读；提交到结算之间授权过期、撤销、变更或无法证明有效时，未结算请求原子拒绝或保持可读待决，不产生 Data sink、访问收益或隐性义务，并提供重新授权或替代路径；有效结算只产生一次 receipt 支持的授权使用或转移。重试、重连、重复提交和历史 receipt 重放不产生第二次结果或复活失效授权，后续撤销不抹除既有合法结果的 provenance。具体许可状态机、时钟、结算规则、幂等实现与副作用矩阵由专业域拥有。
- SC-9：成熟世界样例证明小规模玩家在不立即依附 major power 的前提下，通过可归因贡献获得新选择、恢复弹性、议价位置或区域用途；失败保留 repair / rebuild / pivot，区域影响不越界为全局治理权。
- SC-10：产品样例证明世界没有强制通关条件，但每个阶段成果具有完成边界、可归因世界后果与下一阶段方向；长期成果只在新增选择、恢复弹性、局部议价/协调位置或区域用途时成立，不能以库存、吞吐或重复次数冒充成长。
- SC-11：代表性首局、后引导与成熟世界样例保持一个当前主目标与低负担的继续/分支/换向选择；作用域、canonical 转译、校验、治理、反支配与审计在后台执行，只有改变资源、权限、锁定、恢复或共同承诺时才以可读原因和替代路径进入前台。
- SC-12：代表性首局、持续目标与成熟世界样例证明玩家可通过短命令/复盘和有边界的离线授权维持普通成长，同时可自愿进入较长的区域、外交或治理会话；玩家无需先掌握全部系统深度或保持持续在线，且高风险窗口、授权范围与恢复路径可读。
- SC-13：代表性区域冲突与赛季样例证明攻击只在已声明的 charter 范围和登记参与者/暴露资产间发生，非参与者受保护；实体领地/战利品结算、可恢复损失、软赛季刷新和系统性恢复均保留同一世界时间线、身份与可审计因果。
- SC-14：代表性协作样例证明直接人类沟通不自动绑定，而 Agent 代表在有效授权、接受与权威校验后可形成可审计合同；持续服务的违约/救济、随机无冲突本地争端程序、情境化可更新声誉和预声明 R&D 归因均不产生永久污点、隐性权力或对个人政治 credential 的转让。
- SC-15：代表性组织解散与长期不活跃样例证明 charter 不越过个人资产、合同、退出、历史和 Agent 身份的保护底线；风险冻结、合同与托管处理、责任/成本、持续业务处置、剩余分配，以及通知、保护期、可恢复主张、申诉和有限后续处置均留在同一可审计世界因果链中，不产生静默没收、身份删除、历史重写或重复生效。
- SC-16：代表性资源、资格或持续义务路径证明预览不产生世界效果；已接受承诺可读出范围、未决后果和下一步；已结算结果有 receipt 支持。报价与提交之间发生竞争性状态变化时，提交按当前条件至多结算一次并产生 receipt，或原子拒绝且不产生 sink/义务；`electricity_after` 不被误读为运行 runway 或不停机保证。过期、撤销、失效、重连重试和历史 receipt 重放不会制造第二次 sink、免费资源、资格续期或隐藏欠费，且未结算部分与既有已发生后果按相应专业合同保留可读 provenance、拒绝/待决或恢复路径。
- SC-17：代表性共同决策样例证明普通治理只受理明示的政策、公共财库与既有 charter 日常事项；宪制、基本权利与系统安全事项被原子拒绝或路由到独立轨道，不产生部分世界效果。普通提案、财库、charter、技术升级、局部多数、历史声望与紧急状态均不能绕过保护底线；其他宪制变更只有在公开影响说明、延迟/复核、适用超多数、跨区域或受影响主体确认、独立审计与程序申诉全部成立时才生效。
- SC-18：代表性区域设施与受治理扩展样例证明玩家只能在可读压力、条件报价、有限作用域和维护/耗尽/退役边界内取得区域 leverage；创作者提案只有经治理审查和权威生效后才成为世界能力。报价后状态变化、重复/过期/重连或跨入口重试只产生一次权威结果或原子拒绝，待决请求不会被表达为已生效。
- SC-19：代表性系统性危机样例证明 containment 仅限制扩散并保护基本连续性；恢复项目在同一权威世界时间线内由玩家、Agent、组织或区域参与，并按既有资源、权限、治理、反滥用和申诉边界结算。不存在 reset、历史重写、跳过申诉或选择性 bailout；受影响主体可读触发事实、作用范围、当前限制、恢复/申诉路径与下一步。
- SC-20：代表性工业任务证明报价无世界效果，提交只会原子拒绝或创建一次可追溯承诺；任务在完成、阻塞或中止时按适用专业规则守恒处置投入、在制进度、产出与副产物。进入阻塞时，玩家能读懂最早根因、占用/消费、保留/损失、稳定窗口影响、下一次权威复查边界与真实可用的继续/等待、修复、改道、降载/拆分、重新规划或中止路径；每个展示候选逐项具备作用目标/范围、追加成本与仍占用、预计结果/复查时延、失败/损失风险、可撤回性及推荐理由，缺字段不得展示为可选；不存在安全路径时能明确停止并获得下一次可决策点。目标换向、停机、权限变化、重连、重试与 replay 不会静默取消/迁移任务、重复 sink/产出、释放或退款，或同时生成退款与完成结果；同一因果计划的恢复保持 root continuation，因果变更建立链接 revision/candidate 并从 `W=0` 重新计数。
- SC-21：代表性多阶段流水线证明下游只能消费已经结算、实际到达适用账本且满足该阶段规格/品质适用范围的中间品；阶段/边身份、可用/预留/加工中/在途状态、批次适用结论与来源、有限缓冲及根因/派生 blocker 可追溯。来源未知、证据不足或不兼容的批次在下游 sink/进度前 fail closed，并让玩家区分适用、待验证/证据不足与不适用；不得静默推断、降级、混料、转换或免费 salvage。多输入阶段以 canonical join identity 绑定完整 parent-edge/receipt set；默认在同一快照齐套前无 input sink/WIP/进度，只有 profile 明示 staged intake 时才能逐 parent 单次消费并保留可读 remaining obligations，缺失/无效/过期/换线 parent 不得形成隐藏 partial kit 或复用 receipt。多个已接受意图争用同一阶段/边/buffer 时，基于同一权威快照产生完整/部分/延期/拒绝结果，不超额 hold、不靠提交顺序或重试插队；释放或过期只触发一次 fresh-snapshot 重评，仍有效意图不会被无说明地无限跳过。声明 end-to-end service window 时必须绑定 stage/edge/buffer/terminal reservation chain、权威时界与 none/soft/hard policy；各边界重验剩余义务，expiry/release/renewal 只生效一次且不靠重连/replay 延期，production on-time 不能冒充 late/expired delivery。声明 performance/diagnostic window 时必须绑定 candidate/config epoch 与 journal/state-root；submit/allocation 为每个 accepted outcome 原子冻结一次 immutable plan baseline，preview/atomic reject 不创建，后续 replan 只能创建 parent-linked revision，actual receipt 不跨 revision 或按新配置回填；计划/承诺/执行/生产/交付分层，backlog/join/WIP/transit/buffer/terminal buckets 互斥，receipt 对账损耗与补偿，primary root blocker 不与派生影响重复计数，cutover/replay 不混窗，缺证据显示 unknown 而非伪造。共享容量的 partial 不自动允许 partial recipe：每个 recipe/factory profile 声明合法执行单位、full-only/partial 策略、确定性量化与 residual 处置，结果可追溯 requested、committed、executed、unmet 及主/副产物总量；不足合法单位不扣料、不产半批、不计稳定进度。一个 cycle 的主产物与全部副产物必须属于同一 output bundle；默认在任何 output credit/progress 前原子预检并结算全部 branch，只有 profile 明示 split fan-out 时才能逐 branch 单次结算并保留 failed-leg pending/residual，不能因一条 branch 失败而丢失/复制另一条或伪造 bundle 完成。下游不可用或缓冲满时，上游按声明的背压规则保持、入缓冲或原子拒绝，不静默丢弃、瞬移、无限堆积或伪造完成。阶段/配方/设施/边的因果替换必须在单一 cutover 边界为旧 hold、在制、在途和缓冲批次产生一次显式处置，新候选不继承旧稳定进度、reservation、receipt 或里程碑；不支持的迁移 fail closed，支持的 handoff/conversion 保留父 receipt、数量与损耗。终端阶段必须把 production 与 delivery/settlement 分开：最终产物只进入 owner-bound 有限账本、明确交付承诺或受支持的 hold/quarantine，非 settlement receipt 不减少需求、不发放交付奖励或终端里程碑；终端失效保持有界 blocked/待交付或原子拒绝，不产生无限库存、隐式销毁/转卖或虚假交付。拆分/汇合、跨账本运输损耗、支持的隔离/复验/返工/替代去向、重连、重试、恢复与 replay 保持确定性资源守恒，每阶段至多一次 sink、产出和里程碑效果，并让玩家读懂吞吐、输入齐套、批量、输出 branch、批次用途、共享容量、服务窗口、诊断窗口、换线与终端交付机会成本及恢复选择。该标准还必须覆盖 extraction/replenishment→refinement→source-to-ledger handoff→首个 canonical consumable ledger input 的玩家路径：preview/source-settled/in-transit/arrived/applicable/blocked 各状态可读且不互相冒充；实际数量、损耗、owner/账本、parent provenance 与适用结论守恒可追溯；失败可按专业合同等待补充、补电、重报价、改道、隔离、返工、返还、补偿或放弃，不能自动退款/免费转化；重复 submit、arrival、重连、乱序与 replay 不复制 source effect、目的 credit、首个 input sink、稳定进度或奖励。产品承诺止于首个 canonical input，字段、公式、物流/账本算法、runtime/Viewer 实现由专业 authority 负责。

- SC-22：代表性产线维护/计划停机样例证明排产前可比较 `maintain_before_run`、`run_at_risk`、`reduce_load` 与 `defer` 中真实支持的路径，并读到维护目标/范围、追加成本与仍占用、预计产出/复查时延、故障/损失风险、可撤回性与推荐理由；维护真值缺失时显示 `maintenance_not_tracked` 而不伪造安全继续。若 submit boundary 仍为 `unknown`，会产生 irreversible sink、停机/稳定影响或交付承诺的候选不得 selectable，必须 atomic reject 或保持有界 pending（不得产生隐式 hold）并给出下一次权威复查；只有专业合同真实支持且明确标注 `at-risk/unknown` 的路径可继续展示，preview 无 authority。报价无世界效果，提交按当前状态重验且前置漂移原子拒绝并保留原任务/未结算状态，成功只产生一次维护/排程结果，计划停机不伪造稳定/交付，重连、重试与 replay 不复制 sink、停机结果或里程碑。
- SC-23：代表性 `BuildFactory` 路径证明建设报价只读且绑定 owner/site/kind/id、candidate/config/world revision 及稳定 quote correlation；玩家可比较 build-now、补料、移站、补电和延期，并逐项读到 owner-held electricity 与全部 construction inputs 的 kind/quantity/ledger/before/after、power mode、profile 选定的 output boundary、主要 blocker、下一复查点与首个工业目标关联。每个可选项都说明目标/范围、追加成本与仍占用、预计结果/复查时延、风险/损失、可撤回性与推荐理由，未实现或前置不可证明的选择不得 selectable。提交按当前权威状态重验，竞争性漂移只能产生一次当前条件 receipt 或无 sink 的原子拒绝；任一输入不足、地点/chunk/owner/kind/existing-or-pending ID、未知 authority 与未冻结 mode 不得伪装为可支付或已预留；四类 mode 各自的 sink/hold/revalidation/best-effort 和争用/unknown 边界可验证，重连、重试、重复提交、恢复与 replay 不复制 sink、设施、激活、发电或奖励。建设成本、配方电力、maintenance 与 battery runway 分层，PowerPlant 仅在 profile 声明的 output boundary 后生效；Viewer 与 pure API 对上述事实保持一致。该 SC 只冻结玩家承诺与验收，不冻结公式、字段、队列、UI 或当前实现完成声明。
- SC-24：代表性配方生命周期证明 `pending_validation`、`validated_pending_admission`、`active`、`restricted`、`retiring`、`retired` 六状态及其转换条件可读，且生效前无世界效果；预览与本 SC 使用同一状态集，授权/兼容/证据问题作为 blocker reason 单独表达。每个 accepted 任务固定不可变 recipe version/authority，并与 factory capability、输入批次、物流路径、output bundle 和 terminal purpose 关联；`agent_engineer` 必须能验证 Agent 只能在授权作用域内提议/确认，不能以推荐或重试改变状态。改变输入/输出/副产物、规格/品质、阶段/边、power/logistics 或 terminal 资格等产出因果内容必须新建 parent-linked 版本/候选并从 `W=0` 开始，不能继承旧 receipt、reservation、队列优先级、进度或奖励；非因果元数据才可保留身份，无法证明则 fail closed。版本/授权/输入规格/factory fit/路线/容量/power/terminal 证据缺失或漂移时，在首个不可逆 sink 前原子拒绝或保持无效果待决，玩家可见 primary blocker、机会成本和复查点，不能同名替代、免费转换、静默降级或伪造退款。六状态动作矩阵必须分别覆盖新排程、accepted 未产生首个不可逆 sink、WIP、在途和 buffer：`active`/作用域内 `restricted` 才可 fresh revalidation 后排程；其余状态不可新排程；retiring/retired 不得自动迁移 successor。退役禁止新排程但保留历史；未开始、WIP、在途、buffer 任务只能按 profile 各产生一次 finish/hold/rework/salvage/return/改道/终止处置，兼容 successor 需 fresh revalidation、明确授权和 parent-linked conversion，重连/retry/replay 不复活旧版本或复制效果。权威追踪同时覆盖 `PRD-WORLD_SIMULATOR-047` / `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md` 的批次/适用性/处置语义，以及目标 `PRD-WORLD_RUNTIME-043` / `doc/world-runtime/prd.md` 的 identity、parent linkage、幂等 finality 与 replay 边界。`test_tier_required` 覆盖上述生命周期、转换、漂移、退役、兼容与单次效果，`test_tier_full` 覆盖多阶段在制/在途/缓冲、持久化/replay、Agent 重试和 Viewer/pure API parity；产品层不冻结 recipe ratio、授权角色、兼容算法、runtime schema、UI 或当前实现声明。
- SC-25：代表性已建成工厂能力生命周期证明，玩家能在工厂仍为当前能力候选时比较 `upgrade_existing_capability`、`reconfigure_recipe_fit`、`run_at_current_capability`、`reduce_or_defer`、专业合同支持的 `retire_and_rebuild` 与 `defer`；每个可选项必须说明目标/作用域、追加资源/维护成本、停机或容量机会成本、对吞吐/物流/终端承诺的影响、受影响的 accepted-unstarted/WIP/in-transit/buffer 工作、风险、可撤回性与下一次权威复查点。预览只读且不产生 hold、队列、输入 sink、退款或能力效果；提交必须按最新 owner、factory fit、recipe/规格、power、logistics、capacity、terminal 与治理 authority 重新校验，未知或未追踪 authority 只能显示 `unknown/not_tracked`，不得按零成本、安全或可兼容处理，涉及不可逆 sink 的候选必须原子拒绝或保持有界待决。
- SC-26：代表性 `BuildFactory → factory-ready/recipe-missing → recipe lifecycle decision/admission → ScheduleRecipe → join → stage → transit → buffer → terminal` 路径证明产品读面能把 factory/capability、recipe version、原料批次适用性、物流边/容量、power/maintenance、output bundle 与 terminal purpose 收束为 `准备/待开工`、`运行中`、`产出待交付`、`已结算` 四类玩家决策；每类都显示 primary blocker、已占用/消费价值、`next_action`、`next_recheck` 与 `progression_effect`，且 `BuildFactory` receipt、recipe preview/admission、`accepted`、`produced`、`buffer-held`、`terminal-pending` 不被误报为生产、交付或成长。任何生产奖励/能力里程碑至少等待首次 production receipt；`production-only` 目标可据匹配的 production receipt 与稳定条件完成但保持 undelivered 且不减少交付需求，terminal-admission 目标只有取得匹配的 delivery/terminal settlement receipt 才能完成并打开交付型下一候选；缺料、批次/配方/工厂不兼容、物流/电力/终端失效与 unknown authority 保持可读 blocked/fail-closed 及真实恢复路径。因果换线创建 parent-linked 新目标并将 `W` 归零；同一计划继续保持 root continuation；Viewer、pure API、Agent 与 replay 对状态、根因、下一步和一次性完成结果一致。产品层不冻结 runtime schema、队列、配方/产率/价格公式或 UI；`game` 拥有目标节奏与平衡，M4/runtime 拥有 lifecycle/receipt/守恒/幂等，QA 拥有组合验证。
  - 改变产出因果的能力、支持 recipe version、输入规格、阶段/边、output branch、power/logistics 或 terminal fit 必须创建 parent-linked 新 capability candidate，并在单一 canonical cutover 边界生效；cutover 前的 hold、accepted-unstarted、WIP、in-transit、buffer 与 receipt 保留旧 candidate 身份，cutover 后的新意图只能归属新 candidate，且新 candidate 从 `W=0` 开始，不继承旧 queue 顺位、reservation、稳定窗口、receipt、里程碑或奖励。只改变非因果展示元数据才可保留身份，无法证明时 fail closed。
  - `upgrade/reconfigure` 或 `retiring/retired` 不能静默迁移既有工作：accepted-unstarted 只能按 profile `hold/release/replan/terminate`，WIP 只能按 profile `finish/pause/hold/rework/salvage/terminate`，in-transit 只能使用已声明的 `finish/hold/return/reroute/reject`，buffer 只能使用已声明的 `hold/handoff/conversion/reject`；每项旧工作至多一个处置 receipt。`retiring/retired` 禁止新排程并保留历史；successor 不能自动继承旧任务，只有在明确授权、fresh revalidation 和 parent-linked conversion 成立时才可转换。不得免费补发产出、隐式退款、销毁/瞬移/转卖库存，或同时产生退款与完成。
  - 工厂能力的玩家可读状态类至少区分 `operational`、`reconfiguration_pending`、`reconfiguring`、`degraded`、`retiring` 与 `retired`；这些是产品表达，不冻结 runtime enum。`degraded` 只有在专业 profile 明确允许且标注 `at-risk` 时才能接受新工作。所有升级、退役、处置、释放和转换必须沿同一 factory/capability/recipe/lineage identity 至多生效一次；重连、重试、重复提交、Agent 推荐或 replay 不得重复 cutover、释放 hold、复制产出/奖励、改变稳定窗口或复活旧 candidate。该规则强化 world-first 的真实能力变化、emergence-first 的有边界玩家选择、persistent 的历史连续性、auditable 的 parent receipt/cutover/成本 provenance 与 extensible 的 profile/治理扩展边界。
  - `test_tier_required` 覆盖 preview 无效果、报价后 drift 的拒绝/重报价、上述四类旧工作 bucket 的逐项处置、升级/退役单次 cutover、recipe-fit 与 candidate/receipt/reservation/W 隔离、unknown authority、retired 不可复活，以及 Viewer 与 pure API 的状态/机会成本/下一动作一致性；`test_tier_full` 覆盖并发工作与共享容量、连续升级、cutover 部分失败、crash/restore/replay、持久化、多阶段 WIP/transit/buffer、Agent 重试和 successor conversion。产品承诺链接 `doc/game/prd.md`、`doc/world-runtime/prd.md`、`doc/world-simulator/m4/industrial-resource-flow-contract.prd.md` 与 `doc/testing/prd.md`，不复制其 runtime、WASM、物流算法或测试实现。

- SC-27：代表性 `factory-ready/recipe-missing → recipe candidate discovery → recipe_selection_preview → ScheduleRecipe` 路径证明候选只从同一权威快照派生，并区分 `candidate_available`、`candidate_blocked` 与 `candidate_unknown`；候选能同时说明当前目标、recipe version/authority、factory fit、原料适用/缺口、物流/容量、power/maintenance、output bundle/终端、机会成本、主要风险、`next_action` 与 `next_recheck`。同名版本不合并，只有 `candidate_available` 可进入 `ScheduleRecipe` 排程预览，最终提交仍须新鲜校验；选择 preview 不产生 activation、hold、input sink、queue、物流 reservation、产出、稳定进度或奖励；准入/原料/物流/电力/终端漂移必须重发现、重报价或原子拒绝，不能自动换配方。无候选时玩家获得明确原因与补料、补证、改路、改厂、等待或延期中的真实路径；选择、排程、生产、稳定与交付结果分层，重连、重试、恢复与 replay 不自动选择或复制效果，Viewer、pure API 与 Agent 对候选和下一步保持同义。产品层不冻结 recipe catalog、解锁树、兼容算法、runtime schema、UI 或当前实现完成声明。
- SC-28：至少一份代表性配方执行档案把同一 candidate 的 factory capability、pinned recipe、两类 required raw-material inputs 及其 batch/quality/custody、effective logistics path/loss/capacity、canonical executable cycle、power mode、主/副产物 output bundle、terminal purpose 与 progression boundary 收成完整只读投影；缺失任一 authority 时为 `incomplete/unknown/blocked`，不宣称可排程、稳定或交付。单一 deterministic fixture 覆盖齐套成功、input 不适用、物流满后重评、power 漂移、mandatory output destination 失效以及 production-only/terminal-admission 两种完成边界；失败在首个不可逆 sink 前延期/原子拒绝，已发生 WIP/transit/branch effect 只按 profile 单次处置。未选路径不取得 hold/优先级/进度，因果改变建立 parent-linked 新 candidate 并从 `W=0` 开始；retry/reconnect/replay 不复制材料、容量、产出、交付或奖励，Viewer/pure API 对完整档案、blocker、机会成本和下一步保持同义。产品层不新增 runtime schema，不冻结配比、产率、价格、容量、tick、队列或 UI。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | gameplay_designer | PRD-GAME-004 | `doc/game/prd.md` | 首局玩法与可读性证据 | test_tier_required |
| SC-2 | gameplay_designer | PRD-GAME-004 | `doc/game/prd.md` | micro-loop 端到端回归 | test_tier_required |
| SC-3 | gameplay_designer | PRD-GAME-007 | `doc/game/prd.md` | PostOnboarding 转换与持续游玩证据 | test_tier_required |
| SC-4 | qa_engineer | PRD-GAME-003 | `doc/game/prd.md` | PRD-ID 到发布验收证据的追踪检查 | test_tier_required |
| SC-5 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-004 / PRD-GAME-013 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | `test_tier_required` 覆盖单/多候选比较、确认前无效果、拒绝/过期/权限变化与重试不重复；`test_tier_full` 覆盖跨入口并发确认、授权收缩或替换、原请求到替代与唯一 receipt 的可审计关联 | test_tier_full |
| SC-6 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer | PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-WORLD_RUNTIME-001 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md` | 玩家意图、Agent 决策、权威后果与打断/纠正/恢复组合证据，含正式玩家 surface 的 S6 交互闭环 | test_tier_required |
| SC-7 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 无玩家直接输入时的持续时间线，以及物理真值与粗粒度/表现映射一致性审计，含 S6 表现层核对 | test_tier_required |
| SC-8 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | Data 预览无效果、授权中途失效无 sink/收益、单次结算、retry/reconnect/replay 不重复、既有 provenance 保留与恢复路径证据，含结构化 pure API 和 S6 正式玩家 surface 的状态/原因/下一步可读性 | test_tier_required |
| SC-9 | producer_system_designer / gameplay_designer / qa_engineer | PRD-GAME-015 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/testing/prd.md` | mature-world player leverage、anti-grind、恢复与有限区域影响 fresh sample；产品合同见本模块的 mature-world 专题分册 | test_tier_full |
| SC-10 | producer_system_designer / gameplay_designer / qa_engineer | PRD-GAME-007 / PRD-GAME-015 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/testing/prd.md`; 本模块的首局与成熟世界专题分册 | 阶段成果、三条长期抱负轴、anti-grind 与无强制终局的组合审计 | test_tier_required |
| SC-11 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer | PRD-GAME-004 / PRD-GAME-007 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 一个当前主目标、继续/分支/换向与仅在实质相关时显现的后台护栏组合证据 | test_tier_required |
| SC-12 | producer_system_designer / gameplay_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 分层信息、短命令/复盘、已授权离线推进、可选深度会话与高风险有界响应的组合体验证据 | test_tier_required |
| SC-13 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/chartered-conflict-soft-seasons-and-recovery.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 参战范围、离线防御、占领/提取、可恢复重建、赛季刷新与统一世界连续性的组合证据 | test_tier_full |
| SC-14 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 授权/接受合同、atomic 与持续服务、争端 receipt/程序申诉、声誉/转让与 R&D provenance/份额的组合证据 | test_tier_full |
| SC-15 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/organization-continuity-dissolution-and-dormancy-protection.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | charter 保护底线、解散 waterfall、通知/保护期、estate 或可撤销 delegation、reclaim/appeal、持续业务处置和历史/receipt 连续性的组合证据 | test_tier_full |
| SC-16 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同一条资源/资格/持续义务路径的预览、接受、结算、失效、重试与 receipt 重放负例组合证据；必须含“报价后竞争性状态变化”的接受一次/原子拒绝样例，以及正式玩家 surface 对余额、runway/停机风险、预览/待决/已结算的区分，不伪造自动恢复 | test_tier_full |
| SC-17 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/governed-common-decisions-and-constitutional-boundaries.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 普通事项白名单、排除事项原子拒绝/独立路由、宪制条件全满足/缺失、防绕过与玩家可读 receipt 因果的组合证据 | test_tier_full |
| SC-18 | producer_system_designer / gameplay_designer / runtime_engineer / wasm_platform_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-010 / PRD-P2P-002 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 区域设施生命周期、扩展治理准入、报价后状态变化、单次权威结果、待决表达与 replay/恢复身份一致性的组合证据 | test_tier_full |
| SC-19 | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-P2P-003 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/chartered-conflict-soft-seasons-and-recovery.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | containment 与恢复项目在同一世界时间线中的组合证据；无 reset/历史重写/选择性 bailout，保留 appeal、replay/provenance，并区分正式玩家 surface 的 containment、恢复中、已恢复与 blocked；详细 AC-6/AC-7 与实现合同由上述权威维护 | test_tier_full |
| SC-20 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-004 / PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 正常完成、提交前状态变化、开始前/后的适用中止或无中止能力、设施/维护/权限中断、目标换向、重连/重试/replay 的资源守恒与单次结算证据；Viewer 与 pure API 对任务阶段、投入/产出、保留/损失、稳定产线影响和下一步保持一致，Agent 打断保留旧意图 handoff | test_tier_full |
| SC-21 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-001 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | `test_tier_required` 覆盖两阶段正常结算、低于规格、规格来源未知、下游缺料/缺电/物流未到达、缓冲满背压，以及 2–3 条 parent edges 按不同顺序到达的 join：atomic profile 在全部 parent 齐套前无 sink/WIP/progress，缺失/无效/规格不符 parent 保留可读 join_pending/held/unmet；supported staged profile 明示逐 parent 消费、remaining obligation 与 disposition，late arrival 只重评未决 parent，重复到达/retry/replay 只产生一个 join receipt；staged-intake 部分 hold 形成 wait cycle 或无法证明可收敛时，在新增 sink/WIP/progress 前 fail closed，cycle root、held/unmet、重评边界与 release/defer/terminate/compensation 路径可读，重评/释放/replay 不产生无界 hold、隐式回滚或第二次效果。至少两个已接受意图对同一阶段/边/buffer 的容量不足争用须在同一快照和 contender 集下不受提交/网络顺序影响，不超额 hold，完整/部分/延期/拒绝、根因、剩余需求、机会成本、重评条件和恢复动作可读；释放/过期只重评一次，retry/reconnect/replay 不插队或重复效果；一条 multi-tick stage→transit→buffer→terminal 路径分别验证 none/soft/hard window，所有 leases 有效时只结算一次，任一边界的 owner/route/capacity/expiry 漂移产生一次 late/hold/reject 与一次 release，无 orphan hold/隐藏 sink，生产准时但 transit/terminal 迟到时只有 production receipt；同一 journal/state-root 与 measurement boundary 在 replay 后得到相同 diagnostic snapshot，requested/committed/executed/produced/delivered 不混计，backlog/join/WIP/transit/buffer/terminal buckets 互斥，损耗/补偿只对账一次，强制 parent/power/edge/buffer/terminal blocker 稳定归因 primary root/secondary，cutover 分窗且缺失 plan/证据显示 unknown；full-only 与 profile-supported partial recipe 分别验证合法执行单位、量化/residual、输入/容量漂移、requested/committed/executed/unmet、主/副产物总量，partial capacity 不绕过 batch admissibility，等价 full-vs-split 工作不多计稳定进度/里程碑；主产物+副产物 bundle 全 branch 成功只结算一次，任一 branch owner/路线/容量/资格失败在 atomic policy 下无 output credit/进度，supported split policy 明示 settled/pending/residual 且恢复只重评未决 branch；分别在旧 hold、在制、在途与缓冲批次存在时执行因果换线，证明单一 cutover、旧/新身份分离、唯一处置、新窗口归零且无静默迁移/退款/丢失/重复效果；并覆盖终端 buffer 可用/已满、owner/需求失效、production 与 delivery receipt 分离、非 settlement 不解锁需求/奖励、稳定窗口按声明的 terminal-admission/window 策略计数。该层还必须覆盖 extraction/replenishment→refinement→source-to-ledger handoff→首个 canonical consumable ledger input：`preview/source-settled/in-transit/arrived/applicable/blocked` 的状态、原因与下一步可读且不能互相冒充，数量/损耗/owner/账本/parent provenance/适用性守恒且可追溯，失败恢复只使用专业合同支持的补充/补电/重报价/改道/隔离/返工/返还/补偿/放弃，重复 submit/arrival/重连/乱序/replay 不复制 source effect、目的 credit、首个 input sink、稳定进度或奖励。`test_tier_full` 覆盖长 measurement window/rollover、3+ stages/edges 的跨窗口链、3+ input joins 与 3+ output branches、跨阶段不同 batch unit、不同规格批次的拆分/汇合与共享边、跨账本运输损耗及副产物交付、parent/branch/window 乱序/重复/故障与 late capacity release、长时量化无累计漂移/争用无静默 starvation、staged-intake wait-cycle injection 与持久化/replay 后不产生 orphan/隐式回滚/重复效果、支持的隔离/复验/返工/替代去向与显式 handoff/conversion、重复换线、source/transit/destination 故障窗口、支持的 compensating receipt、持久化、边界 checkpoint 恢复/replay、Agent 重试，以及 Viewer 与 pure API 的适用结论、diagnostic/window/lease/join/batch/bundle/branch/分配/换线/终端结算结果、lineage、守恒和单次效果一致性 | test_tier_full |

| SC-22 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 维护/计划停机前的可比较取舍、维护真值缺失的诚实表达、提交重验、bounded pending 无隐式 hold、原任务/未结算状态保留、单次结果与稳定/交付影响 | test_tier_required |
| SC-23 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-001 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | BuildFactory quote 的只读/确定性、逐项成本与余额、四类 power mode/未冻结 authority、choices/blockers、quote correlation、fresh submit revalidation、single sink/receipt、profile output boundary、retry/reconnect/replay 与 Viewer/pure API parity | test_tier_required |
| SC-24 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-001 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | 六状态转换与预览同步、玩家动作/恢复/机会成本映射、版本 pin、兼容性/授权/前置漂移 fail-closed、Agent 授权边界、退役对新旧任务的动作矩阵、parent-linked conversion、runtime identity/幂等 finality/replay、持久化与 Viewer/pure API parity | test_tier_full |
| SC-25 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | `test_tier_required` 覆盖工厂能力预览无效果、成本/停机/容量机会成本、authority drift/unknown、parent-linked candidate 与单一 cutover、accepted-unstarted/WIP/in-transit/buffer 逐项处置、retiring/retired 无新排程、successor 不自动迁移、W/queue/reservation/receipt/reward 隔离、retry/reconnect/replay 不重复或复活，以及 Viewer/pure API parity；`test_tier_full` 覆盖并发/连续升级、共享容量、部分失败、crash/restore/replay、持久化、多阶段工作与 successor conversion | test_tier_full |
| SC-26 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | `test_tier_required` 覆盖四类产品读面、同一三阶段路径的 blocker/占用/下一动作/复查/进度影响、production 与 delivery/terminal settlement 分离、结算后下一目标候选、因果换线 `W=0` 与 root continuation、unknown/fail-closed、Viewer/pure API/Agent/replay 一致及单次完成效果 | test_tier_required |
| SC-27 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | 配方候选发现的 `candidate_available`/`candidate_blocked`/`candidate_unknown`、同名版本隔离、目标/工厂/原料/物流/power/terminal 事实比较、只读 `recipe_selection_preview`、漂移后重发现或原子拒绝、仅可用候选进入 `ScheduleRecipe` 排程预览且提交前 fresh revalidation、选择/排程/生产/稳定/交付分层，以及 Viewer/pure API/Agent/replay 不自动选择或复制效果 | test_tier_required |
| SC-28 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | 一份两输入、一受限物流边、主/副产物与终端用途齐全的代表性配方执行档案；完整/unknown/blocked 判定、合法 cycle 与 power mode、join/物流/branch/terminal 失败恢复、production-only/terminal-admission 边界、parent-linked 因果换线、Viewer/pure API parity 及 retry/reconnect/replay 幂等 | test_tier_full |

## 6. Non-Goals

- 不在产品层冻结新的玩法细则、数值、掉落或成长曲线。
- 不把分布式执行、任意 WASM 或全局治理包装成当前玩家默认能力。
- 不复制 `game` 专题 PRD、project 任务或测试步骤。
- 不提供自由配方编辑器、客户端上传即生效、自动配方替换或跨版本免费材料转换。
- 不冻结配方比例、产率/损耗、授权/版税公式、兼容算法、审批角色、runtime schema、队列或 UI；本节也不宣称配方治理、退役迁移或当前工业闭环已经实现。
- 不冻结工厂能力状态的 runtime 枚举、升级/降级/退役成本、停机/吞吐/维护公式、自动迁移或补偿算法；工厂 candidate、cutover、receipt、幂等与恢复实现仍由 `world-runtime`、M4 工业合同及其他专业 authority 拥有。
