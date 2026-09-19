# 受治理的区域能力与扩展

## 文档身份

- 所属产品模块：世界规则与玩法系统
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：2026-09-13
- 专业域权威：[`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md)、[`玩家发布实体合同`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文承载区域 charter、地点 tenure、公共融资、受治理的区域设施与工业能力扩展的长期产品承诺：玩家以有限、可读和可审计的方式改变局部世界，授权创作者也只能经治理把新的可用能力接入同一权威世界。它不把每项设施或每个制成品写成独立产品入口，也不冻结实现合同、数值或当前可用性。

## 设计适用性与生命周期闭合

- 设计判定：`simple-topic-exemption`（`PRD-only-sufficient`）。
- 设计判定 task issue：#3680。
- 设计适用性理由：本 PRD 已表达区域能力选择、授权漂移和退出的产品合同；独立 design 不增加新的产品信息层级。
- 当前 GitHub task evidence：本次分类见 [Issue #3680 C4 设计判定](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652452993)，本次闭合要求见 [Issue #3680 accepted repair](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)。
## 1. 产品承诺

在完成基础能力后，玩家可以针对可读的区域压力评估并投入有限资源，获得有明确作用域、收益、维护或耗尽边界以及可追溯结果的区域能力。设施不是默认必点税、第一轮教学动作、自由建造或全局治理权。

世界可以在授权创作者和治理流程下扩展新的工业能力；扩展从提案到可用结果始终受同一权威世界、审计和恢复边界约束，不能覆盖既有事实、绕过治理或把技术提交伪装成默认玩家能力。

## 2. 玩家边界与组合关系

- 区域设施经历 `发现压力 -> 报价与取舍 -> 明确提交 -> 有限服务 -> 维护、耗尽、恢复或退役` 的可读生命周期；每一阶段说明成本、约束、预期价值、失败和下一步。
- 设施应在玩家已经理解基础控制权、资源压力和区域 blocker 后提供有限区域 leverage；低价值或不合适的选择必须仍可解释，而不被呈现为必选 buff。
- 受治理扩展让新制成品、配方、工厂或等价工业能力在批准后进入可用世界能力；玩家和创作者不能任意上传、直接写入权威状态、覆盖既有世界身份或取得默认全局治理权。
- 设施和扩展都必须保留世界唯一性：玩家报价、确认、权威执行、持久结果、恢复和重连后的可见结果属于同一条可审计链路。
- 报价只描述当前条件，不预留资源、设施容量、价格、资格或排队顺位；提交时必须按最新权威状态重新校验。报价后条件变化时，系统只能接受一次并产生一个权威结果，或原子拒绝并返回更新后的 blocker、取舍和下一步。
- 重复、过期、重连或跨入口重试不能产生第二次扣减、设施服务、发布或恢复结果。尚未获得权威确认的交接、发布或恢复保持待决；玩家和创作者不能把请求送达、技术构建成功或本地缓存当作已生效能力。
- **扩展授权不预留发布权：** 扩展提案获批只说明它在当时取得了受限的准入资格，并不使制成品、配方、工厂或等价能力自动进入世界。每次待决发布或激活都必须按当时仍有效的治理授权、已批准的能力范围和世界前置重新校验；若授权撤销、到期、收缩、被替代或能力范围已不再匹配，请求只能明确拒绝、过期、取消或在当前有效轨道重新提交，不能继承旧批准、静默迁移到新授权或以部分可用状态绕过审查。已经由权威 receipt 确认的结果保留其历史因果，但不授权补充发布、再次激活或追溯改变既有世界事实。
- 可扩展不等于已公开、无限容量或所有创作均已支持；当前公开状态与实际证据继续由根 README 和专业域拥有。

### 2.1 区域 charter、地点 tenure 与公共融资

区域不是任意圈地、组织标签或全局主权，而是由可审计的空间边界、设施/物流关系、持续服务能力及共同维护责任支撑的局部 charter。任何玩家或组织可以提案成立、合并或调整区域，但须提供持续能力、可验证需求或交付、Agent/资源、边界理由和可因虚假、放弃或违约按规则处理的 bond；不能仅凭提案人自报边界、短时资本持有或一次到访取得资格。本地运营者与受影响服务关系参与审查，邻区可就边界、外部性和通行提出异议；全局层只复核宪制底线、反圈地与跨区域权利，不替代本地日常经营判断。提案或审查通过不预留地点、资源、资格或权力；生效必须按当前事实取得一次权威结果。

成熟区域的 charter 日常事项受两类资格共同约束：锁定 OC、可撤回委托与实际控制人封顶形成的受限治理资格，以及持续运营、维护或交付形成的本地贡献资格。资本、短时到访或历史头衔均不能单独形成无限地方控制。权限只覆盖已声明的本地公共项目、服务、许可和受限竞争；普通事项依[共同决策边界](governed-common-decisions-and-constitutional-boundaries.prd.md)执行，触及受保护权利时拒绝或进入独立宪制轨道，不能取得任意排他、跨区控制或全局解释权。基本通行与独立恢复路线始终可用；稀缺容量只可采用透明可审计的结算或替代/重建路径，不能借 charter 拒绝、驱逐或封锁玩家。charter 不是首局进入或独立成长的强制门槛。

持续失去服务、维护、能力、正当性或 charter 条件时，区域依次进入观察与恢复、暂停高影响权限，再在无法恢复时解散并回归未成熟区域；一次短时失败不抹去合法历史。受影响者须看到触发事实、当前限制、恢复条件、异议/申诉与下一决策点。适用 bond 按已公开规则处理；设施、Agent、tenure、个人/组织身份、合同和历史 receipt 保留可审计身份，经规则化迁移、回收、重建或权利处理，不得静默没收或删除。技术停机、未审计处罚和运维便利不得伪装成有效 charter 解除；组织资产的连续性顺序另见[组织连续性](organization-continuity-dissolution-and-dormancy-protection.prd.md)。

地点是公共世界与通行权之上的可审计经营/建设 tenure，而不是可永续主权化的土地。玩家或组织可以取得、依维护和实际使用续期、按规则转让；仅到期、长期闲置、明确违约或预定义公共必要性可触发收回，且须有通知、理由、申诉，以及适用的迁移、可回收拆除或规则化补偿。竞争、区域退化或公共必要性不能自动取消未经规则化处理的资产、身份和历史；地点与设施不得用于囤地、驱逐或封路。

可排他设施/服务主要由透明的使用费、服务费、维护费或自愿合约筹资，付费人可读收益对象、用途、成本、服务范围和退出/替代路径。只有明确不可排他、跨受益范围的公共品才可使用 charter 预授权的区域 levy，并同时受目的绑定、受益范围、预算/费率上限、期限、公开账目和定期复核约束。levy 不是一般财富征收、隐蔽补贴、加入组织/专业化/独立路线的默认前置；无效、过期、越界或未经审计的征费不产生扣减、欠费、资格或通行限制等世界效果。常态与紧急市场边界另见[市场专题](market-normal-state-and-emergency-supply.prd.md)，后者不代替区域 levy 的主责。

以上是产品结果，不规定空间算法、投票/OC 公式、bond/费率/补偿数值、土地登记、签名、状态机或界面。`game` 拥有区域服务、竞争、经济和玩家动作；`world-runtime` 拥有空间、资格、bond、tenure、费用、receipt 与恢复的确定性执行；`p2p` 拥有身份、治理签名与分布式状态；QA 拥有实际候选验证。提案、授权或文档接收不证明功能已实现或已发行。

### 2.2 已生效扩展的撤销、替代与受控退出

授权撤销、到期、收缩或替代不仅影响待决请求，也必须明确说明对已经由权威 receipt 确认、正在世界中提供能力的扩展的效果。否则“撤销”可能被误解为追溯删除历史，也可能被用作让旧能力无限期继续运作的旁路。

- 每次改变已生效扩展授权的决定，必须在其自身有效范围内明确选择并记录以下一种或多种可组合结果：**仅停止新的发布/激活**、**限制既有能力的后续新效果**，或**进入受控退出**；组合结果还必须声明先后关系与共同生效边界。未声明既有能力效果的变更不完整，不能改变既有能力、授权范围或新发布/激活资格；它不能静默停止、扩大、重新授权或永久豁免已经生效的能力。
- 已确认的历史世界效果、来源和 receipt 始终保留；撤销或替代不能把它们表述为从未发生，也不能据此追溯没收独立资产、改写已结算合同或删除 Agent/设施身份。既有能力的后续效果必须服从该决定后当前仍有效的授权范围；它不能借旧 receipt、缓存、重连或自动重试取得已被限制、退出或未重新授权的新能力。
- 当决定要求限制或退出时，既有扩展只能在所声明的范围内完成维持安全、保存可识别价值、履行既有不可绕过义务或生成可读退出结果所需的最小动作；不得接受新发布、扩大作用范围、取得新控制权或把临时处置包装为常态能力。任何进一步动作仍须由当前专业合同的权威校验和 receipt 确认。
- 替代授权在权威 receipt 确认前始终是待决状态；同一重叠作用范围在任一权威状态中最多只有一个可执行授权。替代 receipt 必须原子地确认新授权并收缩或终止旧授权，旧授权下尚未确认的请求必须被拒绝，或由请求方在新授权下显式重新提交；缓存、重试、重连或中间节点不得制造旧、新授权并行产生效果的窗口。
- 受影响的玩家、创作者和相关主体必须能读到原授权、改变原因、当前适用范围、既有能力是继续受限、停止产生新效果还是退出中，以及可用的等待、重新提交、申诉或重新规划路径。待决退出、迁移或替代不等于旧能力已经安全停止或新能力已经可用。

本节只定义产品结果和默认 fail-closed 边界：`game` 仍拥有区域能力的玩法/经济后果，`world-runtime` 与 WASM 专业域仍拥有授权状态、依赖处理、执行、去重、receipt 与恢复合同，`p2p` 仍拥有治理授权与分布式状态边界。它不规定撤销理由、时钟、兼容策略、模块实现、数据迁移或退出补偿。

### 2.3 可验证的区域能力与扩展要求

<a id="req-wr-gr-001"></a>
### REQ-WR-GR-001：区域设施必须形成有界的玩家选择循环

- 要求：设施从区域压力发现到报价、明确提交、有限服务、维护/耗尽、恢复或退役，每一步都要让玩家看到成本、取舍、失败边界和下一步；报价不预留资源、容量、资格或排队顺位。
- 专业权威：[`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)
- 验收：[AC-WR-GR-001](#ac-wr-gr-001)

<a id="ac-wr-gr-001"></a>
### AC-WR-GR-001：设施提交按当前事实接受或原子拒绝

- 覆盖要求：REQ-WR-GR-001
- 给定：玩家针对一个区域 blocker 查看设施报价，报价包含作用域、资源投入、预期价值、维护和恢复边界。
- 当：玩家提交时资源、容量、资格或区域状态仍有效，或已发生漂移。
- 则：有效提交产生一次可审计的有限能力结果并显示后续维护/耗尽路径；漂移时原子拒绝并返回 blocker/取舍/下一步，不能预留或部分扣减。

<a id="req-wr-gr-002"></a>
### REQ-WR-GR-002：治理扩展的发布与激活必须逐次重验授权

- 要求：扩展批准只提供受限准入资格；每次发布或激活都必须按当前授权、能力范围和世界前置重新校验，不能以技术构建成功、旧 receipt 或本地缓存替代权威生效。
- 专业权威：[`player-published-entities.prd.md`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)
- 验收：[AC-WR-GR-002](#ac-wr-gr-002)

<a id="ac-wr-gr-002"></a>
### AC-WR-GR-002：扩展授权漂移时不静默发布

- 覆盖要求：REQ-WR-GR-002
- 给定：一个已经获批但尚未完成发布/激活的扩展，随后授权到期、撤销、收缩或能力范围不再匹配。
- 当：发布请求、重连或跨入口重试到达权威世界。
- 则：请求明确拒绝、过期、取消或要求在当前轨道重提；不继承旧批准、不产生部分可用能力，也不因本地构建或缓存表示为已生效。

<a id="req-wr-gr-003"></a>
### REQ-WR-GR-003：已生效扩展的限制或退出必须保持历史与单一授权

- 要求：撤销、到期、收缩或替代必须声明对既有能力的后续结果；历史 receipt 保留，同一重叠范围最多一个可执行授权，限制/退出期间只允许声明范围内的最小安全、价值保全或既有义务动作。
- 专业权威：[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)
- 验收：[AC-WR-GR-003](#ac-wr-gr-003)

<a id="ac-wr-gr-003"></a>
### AC-WR-GR-003：替代授权原子交接且不重放旧能力

- 覆盖要求：REQ-WR-GR-003
- 给定：一个已生效扩展进入限制或受控退出，并有新授权覆盖相同范围。
- 当：替代 receipt 尚未确认、确认或旧请求重连重试。
- 则：确认前保持待决且不并行产生效果；确认时新授权原子生效、旧授权在重叠范围收缩/终止，旧待决请求需拒绝或显式重提，历史效果保留且不新增未授权能力。

<a id="req-wr-gr-004"></a>
### REQ-WR-GR-004：区域成立或调整须有能力与边界证据及分层审查

- 要求：提案人提供空间锚定边界、持续设施/物流/服务能力、需求/交付、Agent/资源、边界理由及 bond；本地审查、邻区异议和全局宪制/反圈地复核各守其作用范围。批准不预留权力，当前条件失效时不得部分成立。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)
- 验收：[AC-WR-GR-004](#ac-wr-gr-004)

<a id="ac-wr-gr-004"></a>
### AC-WR-GR-004：区域提案不靠自报边界或资本取得生效结果

- 覆盖要求：REQ-WR-GR-004
- 给定：成立、合并或调整提案具有完整能力、需求、边界及 bond 证据，另有缺项、自报范围、邻区通行异议或执行前条件漂移的对照样例。
- 当：本地、邻区、全局层按各自权限审查，且提案尝试执行或重试。
- 则：有效提案在当前条件下只产生一次有作用域的 charter receipt；无效/漂移提案拒绝或待补证据，邻区与全局复核不能被跳过，也不能取代本地日常经营裁量或产生部分权力。

<a id="req-wr-gr-005"></a>
### REQ-WR-GR-005：成熟区域须双资格约束且权限不外溢

- 要求：受限 OC/委托/控制人资格和持续本地贡献共同约束 charter 日常决策；任何一侧都不能凭短时资本、到访或头衔独揽控制，基本通行与独立恢复不受任意排他。受保护事项遵守独立宪制轨道。
- 专业权威：[`共同决策边界`](governed-common-decisions-and-constitutional-boundaries.prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)
- 验收：[AC-WR-GR-005](#ac-wr-gr-005)

<a id="ac-wr-gr-005"></a>
### AC-WR-GR-005：本地权限不能成为封锁或全局控制

- 覆盖要求：REQ-WR-GR-005
- 给定：成熟区域的本地服务/许可事项，及资本集中、可撤回委托、贡献中断、受保护权利和容量稀缺样例。
- 当：本地决策、授权变动和通行请求发生。
- 则：两类资格及实际控制人封顶共同生效；范围外事项拒绝或进入宪制轨道，通行与独立恢复保留透明容量结算或替代路线，不会因 charter 任意拒绝、驱逐或取得跨区/全局权力。

<a id="req-wr-gr-006"></a>
### REQ-WR-GR-006：区域退化分阶段恢复且保留资产历史

- 要求：持续退化先观察与恢复、再暂停高影响权限，无法恢复才解散回归未成熟；bond、设施、Agent、tenure、身份、合同和 receipt 依已公开规则处理，提供触发、限制、恢复、异议/申诉和下一步，不因短时失败或技术停机静默清除。
- 专业权威：[`组织连续性`](organization-continuity-dissolution-and-dormancy-protection.prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)
- 验收：[AC-WR-GR-006](#ac-wr-gr-006)

<a id="ac-wr-gr-006"></a>
### AC-WR-GR-006：暂停、解散与恢复均有可审计下一步

- 覆盖要求：REQ-WR-GR-006
- 给定：短时服务失败、持续退化、恢复成功与无法恢复四种区域样例。
- 当：区域进入观察、暂停、恢复或解散。
- 则：短时失败不直接抹去 charter；持续退化按阶段收缩权限，无法恢复才回归未成熟，适用 bond 和资产有 receipt、规则化迁移/回收/重建、恢复或申诉路径，身份和历史不消失，停机或未审计处罚不被伪装成解除。

<a id="req-wr-gr-007"></a>
### REQ-WR-GR-007：地点 tenure 是可收回的建设权而非主权

- 要求：取得、维护续期、规则化转让和限定触发的收回均可审计；到期、长期闲置、违约或预定义公共必要性下的收回须通知、理由、申诉及适用迁移/回收/补偿，公共通行和既有资产身份不能静默取消。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)
- 验收：[AC-WR-GR-007](#ac-wr-gr-007)

<a id="ac-wr-gr-007"></a>
### AC-WR-GR-007：tenure 转让及收回不没收或封路

- 覆盖要求：REQ-WR-GR-007
- 给定：正常取得/续期/转让、到期或违约收回、区域退化、竞争和公共必要性样例。
- 当：地点权利变化、申请重试或通行受影响。
- 则：只有规则化结果确认权利变化且不能重复转让/没收；适用通知、申诉、迁移/可回收拆除/补偿可追溯，原有设施、Agent、身份、合同和通行不会被静默删除或任意阻断。

<a id="req-wr-gr-008"></a>
### REQ-WR-GR-008：服务费与公共 levy 必须区分并有界

- 要求：可排他服务以透明使用/维护费或自愿合同为主；不可排他公共品 levy 仅在 charter 预授权、目的与受益范围、上限、期限、公开账目和复核均有效时成立；无效征费不得扣减、产生欠费或限制资格/通行。
- 专业权威：[`共同决策边界`](governed-common-decisions-and-constitutional-boundaries.prd.md)、[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)
- 验收：[AC-WR-GR-008](#ac-wr-gr-008)

<a id="ac-wr-gr-008"></a>
### AC-WR-GR-008：失效或越界的 levy 无世界效果

- 覆盖要求：REQ-WR-GR-008
- 给定：可排他服务与不可排他公共品、有效 levy 和未授权/过期/超范围/无账目或审计的征费样例。
- 当：使用费或 levy 提交、到期、重连或重复执行。
- 则：玩家能区分收益、用途、成本、期限与退出替代；有效项目按当前授权最多一次结算，无效者原子拒绝且不产生扣费、欠费、资格或通行限制，也不变成一般税收或独立成长门槛。

## 3. 权威边界

| 层级 | 本产品分册拥有 | 下层专业域拥有 |
| --- | --- | --- |
| 区域价值 | 设施解决局部压力的范围、取舍和有限 leverage | `game` 拥有设施玩法、经济与专业验收 |
| 扩展治理 | 新工业能力必须经治理进入同一权威世界 | `world-runtime` 与 WASM 专业域拥有发布、校验、执行和审计合同 |
| 持久世界 | 结果可审计、可恢复且不形成第二世界 | `world-runtime` 与 `p2p` 拥有状态、回放、复制和恢复证明 |
| 验证 | 组合证据证明玩家可读价值和权威结果一致 | `testing` 与 QA 拥有测试矩阵、样本和当前 verdict |

区域 charter、地点 tenure 与公共融资的产品主责仅在本专题；[共同决策](governed-common-decisions-and-constitutional-boundaries.prd.md)、[组织连续性](organization-continuity-dissolution-and-dormancy-protection.prd.md)与[市场边界](market-normal-state-and-emergency-supply.prd.md)是消费或支撑条款，不重复定义区域成立、地点收回和 levy 的有效条件。

专业入口分别是 [`gameplay-regional-infrastructure-micro-depot-contract.prd.md`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md) 与 [`player-published-entities.prd.md`](../../world-runtime/module/player-published-entities.prd.md)。本分册不复制资源数值、设施库存、WASM/ABI、module hash、profile schema、审批角色、签名、SLA、状态机、receipt 字段或任务证据。

## 4. 组合验收

- GR-1：代表性区域设施流程贯通压力发现、报价与取舍、玩家确认、权威执行、有限服务或失败、可读 receipt，以及维护、耗尽、恢复或退役的下一步。
- GR-2：设施样例证明有限区域价值与可选择的专业化，而非首局强制动作、无条件 buff、自由建造或全局治理权。
- GR-3：代表性受治理扩展流程贯通创作者提案、授权审查、权威生效、世界可用能力、审计与异常恢复；任一阶段不能用技术旁路替代治理。
- GR-4：设施和扩展的同一身份在玩家表达、权威执行、持久化/replay、适用的复制或恢复以及重连可见结果中保持一致。
- GR-5：产品、game、runtime、WASM、P2P 与 testing 证据绑定同一候选；单独的传输 green、模块提交、文档迁移或局部 UI 不得代签端到端闭环。
- GR-6：报价后资源、容量、资格、治理状态或世界前置发生变化时，提交只能按最新权威状态接受一次或原子拒绝；扩展批准撤销、到期、收缩或被替代时，待决发布/激活不会继承旧批准、部分生效或静默迁移到新授权。重复、过期、重连与跨入口重试不会产生第二次扣减、服务、发布或恢复结果，待决状态不会被表达为已生效。
- GR-7：代表性已生效扩展在授权撤销、到期、收缩或替代时，授权决定明确其对既有能力的结果；未声明结果的变更不生效。历史 receipt 和已确认世界效果保持可追溯，既有能力不会借旧授权取得已被限制、退出或未重新授权的新能力；若进入限制或受控退出，只允许最小安全/价值保全/既有义务动作，并向受影响主体表达当前范围、真实状态和适用下一步。
- GR-8：区域成立/调整的能力、空间与 bond 证据经过本地、邻区、全局的不同层次审查，成熟区域受资本/委托/控制人和持续贡献双资格约束；通行、独立恢复及宪制保护不被局部 charter 取消。
- GR-9：持续退化依观察/恢复、权限暂停和解散/回归未成熟的阶段处理；地点 tenure 的取得、续期、转让、收回及适用补偿、公共服务融资的有效或无效结算，都保持身份、资产、历史、通知/申诉和下一步可追溯，且无重复/静默效果。

### 4.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| GR-1 / GR-2 | gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 | 设施报价到有限生命周期和玩家结果的组合证据 | test_tier_required |
| GR-3 | runtime_engineer / wasm_platform_engineer / gameplay_designer | PRD-WORLD_RUNTIME-010 / PRD-WORLD_RUNTIME-011 / PRD-WORLD_RUNTIME-012 | 受治理扩展从提案到可用能力、拒绝和恢复的证据 | test_tier_required |
| GR-4 | runtime_engineer / blockchain_ops_engineer / viewer_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-P2P-002 | 同一能力跨执行、回放、恢复和重连的组合证据 | test_tier_full |
| GR-5 | producer_system_designer / qa_engineer | PRD-TESTING-003 | 同候选跨域组合审计 | test_tier_full |
| GR-6 | gameplay_designer / runtime_engineer / wasm_platform_engineer / viewer_engineer / qa_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-010 / PRD-TESTING-003 | 报价后状态变化、扩展授权失效后的待决发布/激活、重复/过期/重连/跨入口提交与待决表达的负例证据 | test_tier_required |
| GR-7 | producer_system_designer / gameplay_designer / runtime_engineer / wasm_platform_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-010 / PRD-WORLD_RUNTIME-012 / PRD-P2P-002 / PRD-TESTING-003 | 已生效扩展的撤销/到期/收缩/替代样例：授权结果声明及组合边界、不完整变更无效、历史 receipt 连续性、替代授权的单一可执行范围与原子交接、旧授权待决请求处置、已被限制/退出/未重新授权的能力无新增效果、受限或退出中的最小动作、去重/恢复负例及正式玩家 surface 的状态与下一步可读性；`test_tier_required` 覆盖授权结果、替代交接、无新增效果和玩家可读状态，`test_tier_full` 覆盖依赖、持久化、replay/去重、恢复与跨节点一致性 | test_tier_full |
| GR-8 / [AC-WR-GR-004](#ac-wr-gr-004) / [AC-WR-GR-005](#ac-wr-gr-005) | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | 成立/调整边界与能力证据、分层异议/反圈地、双资格与通行/权利负例，同候选验证 | test_tier_full |
| GR-9 / [AC-WR-GR-006](#ac-wr-gr-006) / [AC-WR-GR-007](#ac-wr-gr-007) | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / viewer_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | 区域退化/暂停/解散、bond、tenure 收回/转让、通行、资产与身份历史的 receipt、申诉、恢复和重复提交负例，同候选验证 | test_tier_full |
| GR-9 / [AC-WR-GR-008](#ac-wr-gr-008) | producer_system_designer / gameplay_designer / runtime_engineer / qa_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | 排他服务费与不可排他 levy 的受益、用途、上限、期限、账目、复核及无效扣费/欠费负例，同候选验证 | test_tier_required |

## 5. Non-Goals

- 不承诺任意 WASM 上传、游戏内 IDE、自由建造、无限设施、自动补货或无成本持续能力。
- 不冻结设施资源成本、库存、吞吐、ROI、服务半径或回收规则。
- 不定义 WASM ABI、module hash、profile schema、审批角色、签名阈值、发布 SLA、状态机或 replay 实现。
- 不把技术提案、内部测试或文档迁移表述为已经公开可用或广泛发行。
- 不规定空间边界测量、资格或投票阈值、OC/委托权重、bond 金额、tenure 时长、费率/预算与补偿数值；不实现登记、签名、收款、罚没、申诉或地图界面，也不以区域 charter 改写战争、工业、市场及宪制程序。

## 全量语义追踪

| REQ / AC | 专业 owner | 专业权威 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| [REQ-WR-GR-001](#req-wr-gr-001) / [AC-WR-GR-001](#ac-wr-gr-001) | `producer_system_designer` | [`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md#目标)、[`玩家发布实体合同`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-WR-GR-002](#req-wr-gr-002) / [AC-WR-GR-002](#ac-wr-gr-002) | `producer_system_designer` | [`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md#目标)、[`玩家发布实体合同`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-WR-GR-003](#req-wr-gr-003) / [AC-WR-GR-003](#ac-wr-gr-003) | `producer_system_designer` | [`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md#目标)、[`玩家发布实体合同`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-WR-GR-004](#req-wr-gr-004) / [AC-WR-GR-004](#ac-wr-gr-004) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#目标)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 原区域 charter 成立项的能力/边界/bond 与分层审查、异议及失效负例 | `test_tier_full` |
| [REQ-WR-GR-005](#req-wr-gr-005) / [AC-WR-GR-005](#ac-wr-gr-005) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#目标)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 原区域 charter 成熟治理项的双资格、权限边界、通行和反圈地负例 | `test_tier_full` |
| [REQ-WR-GR-006](#req-wr-gr-006) / [AC-WR-GR-006](#ac-wr-gr-006) | `producer_system_designer` | [`doc/world-runtime/prd.md`](../../world-runtime/prd.md#目标)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 原区域 charter 退化项的恢复、bond、身份/receipt 和申诉负例 | `test_tier_full` |
| [REQ-WR-GR-007](#req-wr-gr-007) / [AC-WR-GR-007](#ac-wr-gr-007) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#目标)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 原区域 charter 地点项的 tenure 取得/续期/转让/收回、通知申诉、通行和补偿负例 | `test_tier_full` |
| [REQ-WR-GR-008](#req-wr-gr-008) / [AC-WR-GR-008](#ac-wr-gr-008) | `producer_system_designer` | [`doc/game/prd.md`](../../game/prd.md#目标)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md) | 原区域 charter 融资项的服务费/levy、无效扣费、退出和独立路线负例 | `test_tier_required` |
