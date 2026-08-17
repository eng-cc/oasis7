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

#### 在途工业任务的承诺、中断与结算

工业排程被接受后，不能继续沿用“报价可随时放弃”的语义，也不能把目标换向、停机或重连自动解释为取消。代表性配方任务必须让玩家在提交前读懂：接受时会消耗或占用的投入与能源、预期产出及副产物去向、形成结果所需的时间或阶段、是否存在取消/中止窗口，以及适用规则会保留、退回或损失什么。报价仍然只读；提交时只能原子拒绝且无新 sink，或创建一次可追溯的已接受工业承诺。

已接受任务必须保持可区分的 `已接受/待开始 -> 已开始 -> 进行中 -> 已完成 / 被阻塞 / 已中止` 产品结果；`被阻塞` 是等待恢复、继续或按专业合同转为终止的可决策状态，不是隐含完成。若专业合同允许开始前释放、撤回或过期，也必须将其表达为未开始的终止结果，不能伪装成已中止的在制任务。这里的分类不冻结 runtime 枚举或 UI 字段，但任何专业实现都不能把“请求送达”“目标已换向”“设施暂不可用”或“客户端已断开”伪装成开始、完成或中止。目标换向、Agent 打断、维护/权限/设施状态变化、重连、重复提交与事件重放均不得静默迁移旧任务、再次扣料、复制产出/副产物，或同时生成退款与完成结果。

取消或中止只在专业合同声明存在该能力时可用，并必须遵循当次任务适用的确定性处置规则：尚未开始的 hold 可以按规则释放；已经产生 sink、在制进度或世界效果的任务不得承诺全额自动退款。任何 salvage、部分返还、在制品保留、继续执行或安全停机都必须范围有界、只结算一次，并保留原投入、已发生结果和处置原因的 provenance；没有已实现且有证据的中止能力时，玩家只能看到真实可用的等待、修复、改道或重新规划路径，不能展示虚构的“取消”。

中断后的玩家反馈至少回答：原任务及当前阶段、已经投入与仍被占用的价值、已形成或仍待形成的产出/副产物、保留与损失、对稳定产线候选进度的影响，以及当前真实可用的继续、等待、修复、改道、中止或重新规划动作。`game` 拥有这些选择的玩法取舍与平衡，`world-runtime` 拥有事件、状态、资源守恒、去重与 replay，Agent 拥有意图打断/换向的可解释 handoff，Viewer 与 pure API 拥有同一权威结果的表达，QA 拥有组合证据；产品层不规定取消 action、退款公式、队列算法、状态 schema 或 UI 布局。

#### 多阶段工业流水线与中间品背压

单个配方任务完成不等于多阶段流水线已经连通。代表性流水线必须以有向无环的阶段关系声明 `上游阶段 -> 中间品边 -> 下游阶段`；每条边至少绑定可追溯的阶段/配方或能力版本、材料类别、来源与目的账本，以及适用的电力、物流和容量前置。替换决定产出因果的阶段、配方、设施或边后，属于新的候选流水线，不能继承旧候选的稳定窗口、未决资格或里程碑进度。循环生产若未来成为正式能力，必须由独立专业合同定义库存上限、终止与反套利规则，不能把隐式环当作普通流水线接受。

中间品必须区分**可用**、**已预留**、**加工中**与**已承诺但尚未到达**。下游只有在上游产出已由权威 receipt 结算，材料按适用物流规则实际进入下游可消费账本，并且该批次满足下游阶段声明的材料规格/品质适用范围后，才能取得开工资格；上游的计划、报价、已接受或加工中状态都不能提前生成下游库存、进度或里程碑。规格/品质适用范围是消费资格，不是产品层新增的数值公式：提交前的适用预览必须说明候选批次、可消费数量、缺口与主要不适用原因，提交时则按当前批次、配方和阶段条件原子重验；来源未知、证据不足或不兼容时只能在下游产生 sink 或进度前 fail closed，不能由 Agent、Viewer、模块元数据或同类材料名称自行推断为可用。一次中间品不能同时被多个下游承诺消费；拆分、混批或汇合必须声明边集合、确定性的分配/齐套规则，以及“只消费适用数量并保留其余批次”或“整笔原子拒绝”的明确策略，并分别验证各输入批次的适用性，不能依赖隐藏提交顺序、Agent 猜测、表现层缓存或把不同批次静默混合后视为合格。

每条中间品边必须声明有界缓冲与背压结果：下游暂不可用或缓冲已满时，只能按专业合同保持上游未消费的投入、将已结算产出放入仍有容量的缓冲，或原子拒绝新的上游承诺。已经发生的加工、损耗或运输不能被静默丢弃、瞬移、自动改道、无限堆积或伪装成下游完成；任何溢出、返工、报废或 salvage 只有在专业合同明确存在时才能按其有界规则结算一次，并向玩家说明代价和恢复选择。

阻塞必须保留因果方向：surface 至少指出根因所在阶段/边、受影响的下游阶段、当前中间品数量与状态、上游是否因背压继续或暂停，以及真实可用的等待、补料、扩容、修复、改道、降载或重新规划动作。规格/品质判断至少要让玩家区分**适用**、**待验证/证据不足**与**不适用**，并指出不满足哪个阶段/边的消费条件；待验证或不适用不能伪装成普通缺料、在后台降级为较低规格，或自动转换、混料、报废。隔离、复验、返工、替代配方/去向、继续持有或 salvage 只有在专业合同明确支持时才是可选恢复动作，并须披露实际损耗、时间/容量机会成本与后续资格；不存在该能力时必须显示真实阻塞或放弃路径。派生的下游缺料不能覆盖根因，也不能把多个阶段压成一个无法定位的“生产失败”。玩家的取舍应围绕吞吐、缓冲占用、交付时机、批次适用性与恢复弹性；只让同一批次反复验证、加工、重连或重放而没有新增能力、选择或世界用途，不构成成长或新的稳定流水线里程碑。

同一批次跨阶段的 lineage 必须在持久化、恢复和 replay 后仍能关联各阶段承诺、边、中间品数量/预留、父级 receipt、规格/品质适用结论及其来源、实际损耗与 blocker。重复提交、Agent 重试、重连或事件重放对每个阶段至多产生一次 sink、产出与里程碑效果，也不能把未知/不适用批次重试成适用；已完成阶段可以按专业合同从已结算结果继续，但不得重新发奖、复制中间品或跳过尚未满足的下游前置。`game` 拥有阶段节奏、容量取舍、规格/品质带来的用途选择、返工收益和 anti-grind 平衡；`world-runtime` 与工业模块拥有图、账本、批次属性、适用性校验、预留、状态、守恒、去重与 replay 合同；Viewer 与 pure API 必须表达同一根因、适用结论、背压和下一步。产品层不冻结品质数值/公式、buffer 数值、配方、并行度、队列/图算法、runtime 枚举或 UI 布局。

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
- SC-20：代表性工业任务证明报价无世界效果，提交只会原子拒绝或创建一次可追溯承诺；任务在完成、阻塞或中止时按适用专业规则守恒处置投入、在制进度、产出与副产物。目标换向、停机、权限变化、重连、重试与 replay 不会静默取消/迁移任务、重复 sink/产出或同时生成退款与完成结果；玩家能读懂保留、损失、稳定产线影响与真实可用的恢复/退出下一步。
- SC-21：代表性多阶段流水线证明下游只能消费已经结算、实际到达适用账本且满足该阶段规格/品质适用范围的中间品；阶段/边身份、可用/预留/加工中/在途状态、批次适用结论与来源、有限缓冲及根因/派生 blocker 可追溯。来源未知、证据不足或不兼容的批次在下游 sink/进度前 fail closed，并让玩家区分适用、待验证/证据不足与不适用；不得静默推断、降级、混料、转换或免费 salvage。下游不可用或缓冲满时，上游按声明的背压规则保持、入缓冲或原子拒绝，不静默丢弃、瞬移、无限堆积或伪造完成；阶段/配方/设施/边替换不继承旧候选进度。拆分/汇合、跨账本运输损耗、支持的隔离/复验/返工/替代去向、重连、重试、恢复与 replay 保持确定性资源守恒，每阶段至多一次 sink、产出和里程碑效果，并让玩家读懂吞吐、批次用途、机会成本与恢复选择。

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
| SC-21 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-002 / PRD-GAME-012 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/game/gameplay/gameplay-top-level-design.prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | `test_tier_required` 覆盖两阶段正常结算、低于规格、规格来源未知、下游缺料/缺电/物流未到达、缓冲满背压、阶段或边替换和重复提交：下游只在中间品结算到达且适用性成立后开工，不适用/待验证不产生下游 sink 或里程碑，根因、状态、机会成本和真实恢复动作可读；`test_tier_full` 覆盖不同规格批次的三阶段拆分/汇合、跨账本运输损耗、支持的隔离/复验/返工/替代去向、持久化、恢复/replay、Agent 重试，以及 Viewer 与 pure API 的适用结论、lineage、守恒和单次效果一致性 | test_tier_full |

## 6. Non-Goals

- 不在产品层冻结新的玩法细则、数值、掉落或成长曲线。
- 不把分布式执行、任意 WASM 或全局治理包装成当前玩家默认能力。
- 不复制 `game` 专题 PRD、project 任务或测试步骤。
