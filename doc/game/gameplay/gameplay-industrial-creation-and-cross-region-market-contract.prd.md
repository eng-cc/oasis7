# Gameplay 工业创建与跨区市场结算合同 PRD

- `PRD-ID`：`PRD-GAME-018`
- 上层产品映射：工业提案、模拟、试点与准入消费 [`REQ-WR-GR-009`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-009) / [`AC-WR-GR-009`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-009)；创作者许可与开放消费 [`REQ-WR-GR-010`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-010) / [`AC-WR-GR-010`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-010)；OC 转让与世界资格隔离消费 [`REQ-WR-GR-011`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-011) / [`AC-WR-GR-011`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-011)。需求、报价与生产/交付结算消费 [`REQ-SC31-009`](../../product/world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#req-sc31-009) 及 [`AC-SC31-010`](../../product/world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-010) / [`AC-SC31-011`](../../product/world-rules-core-gameplay/industrial-demand-goals-and-settlement.prd.md#ac-sc31-011)；常态市场与危机边界消费 [`REQ-WR-ES-004`](../../product/world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#req-wr-es-004) / [`AC-WR-ES-004`](../../product/world-rules-core-gameplay/market-normal-state-and-emergency-supply.prd.md#ac-wr-es-004)。
- 主题 authority：本文件拥有工业能力创建、跨区市场交易与结算的玩家动作、机会成本、失败恢复、progression 和可玩性验收；产品层拥有世界承诺与保护边界，`world-runtime` / `world-simulator` / `p2p` / QA 各自拥有执行事实、确定性、资产状态和验证证据。
- 设计适用性：`simple-topic-exemption`（`PRD-only-sufficient`）。本文只承载 Why / What / Done、玩家循环、风险与验收，不新增 API、schema、状态机、结算算法或 UI 布局；配对例外在 [`doc/game/prd.index.md`](../prd.index.md) 登记。
- 当前执行：可变 task 状态与当前实现证据由 GitHub Project task truth 和 issue evidence comments 拥有；本文件不宣称当前已实现、已平衡、已可交易、已发布或已满足 release readiness。

## 1. 目标与范围

工业创建和跨区交易都应给玩家留下可归因的选择，而不是一个“提交后世界自动变好”的黑箱。玩家要能理解一项新工业能力如何从提案走到受限试点和治理准入，也要能理解一批货物如何从可见报价走到真实路线、目的地接收和分阶段结算。

本合同覆盖四个相连但不可混淆的玩家问题：

1. 如何提交可审计的工业提案，比较确定性模拟与原型/试点的风险和收益，并等待有范围的治理准入。
2. 如何在期限、用途、区域和开放条件明确的创作者许可/版税之间做选择，不把一次创新变成永久垄断。
3. 如何把 OC 的链上转让与世界内资源、信用、合同、资格和高影响权利分开理解，避免用外部资产持有绕过世界条件。
4. 如何从有新鲜度边界的全球情报走到报价/合同、物理路线、目的地存储、escrow 里程碑和争议恢复。

工业创建、许可、OC 隔离、跨区结算与紧急保供的产品消费均已闭合到上列 active `GR-009..011`、`SC31-009/010/011` 与 `ES-004` `REQ/AC` 锚点。普通治理的事项快照与额度桥消费由 [`PRD-GAME-019`](gameplay-ordinary-governance-subject-and-oc-rights-contract.prd.md) 的 [`AC-GAME-019-04`](gameplay-ordinary-governance-subject-and-oc-rights-contract.prd.md#ac-game-019-04) / [`AC-GAME-019-05`](gameplay-ordinary-governance-subject-and-oc-rights-contract.prd.md#ac-game-019-05) 承接，不在本工业/市场合同中重复定义。产品层的“工业规则、市场边界和不作实现/合规承诺”仍优先于本合同，本合同不把紧急保供扩展为新玩法系统。

OC 与世界资格分离现在有明确的 active 产品接收边界：工业/世界资格消费 [`REQ-WR-GR-011`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#req-wr-gr-011) / [`AC-WR-GR-011`](../../product/world-rules-core-gameplay/governed-regional-capabilities-and-extensions.prd.md#ac-wr-gr-011)。治理快照与额度桥消费 [`REQ-WR-GCB-005`](../../product/world-rules-core-gameplay/governed-common-decisions-and-constitutional-boundaries.prd.md#req-wr-gcb-005) / [`AC-WR-GCB-005`](../../product/world-rules-core-gameplay/governed-common-decisions-and-constitutional-boundaries.prd.md#ac-wr-gcb-005) 由 [`AC-GAME-019-04`](gameplay-ordinary-governance-subject-and-oc-rights-contract.prd.md#ac-game-019-04) / [`AC-GAME-019-05`](gameplay-ordinary-governance-subject-and-oc-rights-contract.prd.md#ac-game-019-05) 承接；GAME-018 在此保留 GR-011 的工业/世界资格隔离，不把 GCB-005 的快照/额度桥验收重新定义为工业或市场规则。本合同只定义目标玩法隔离和失败可读性，不把产品锚点写成当前实现、当前可用或 release 证据；执行与实现仍由对应专业 authority 收口。

## 2. 玩家循环：从创造机会到可复盘结算

所有预览、报价和治理请求都是只读的：它们不预留资源、能力、资格、库存、路线、结算顺位或版税。玩家确认后，权威执行必须以同一份新鲜事实重验；事实漂移时只能要求重报价、补证、缩小范围、改走替代路线或原子拒绝。

| 阶段 | 玩家动作与比较 | 玩家获得的即时价值 | 机会成本、失败与下一步 |
| --- | --- | --- | --- |
| 1. `inspect_opportunity` | 查看待解决的需求/瓶颈、现有能力、资源与资格边界；判断提出新能力还是使用既有生产/交易路线 | 能区分“值得创新的问题”与可由现有路线解决的问题，并看到可能影响范围 | 花费注意力、证据准备和试点窗口；证据不足时补证、缩小目标或放弃，不进入不可逆提案 |
| 2. `submit_auditable_proposal` | 提交来源、假设、适用范围、预期影响、风险和退出条件；比较继续、延期、拆分或撤回 | 提案取得可读的审查位置和缺口清单，不取得世界写入权 | 提案准备会占用时间/材料/押注机会；重复、越界或无法复核时只保留可读拒绝与补证路径 |
| 3. `compare_simulation_and_prototype` | 先看同一确定性输入下的模拟结果，再选择受限原型/试点、继续观测、改参数假设或停止 | 看见预期收益、资源压力、失败后果、适用范围和下一审查条件 | 模拟失败不产生世界效果；试点会占用限定资源/窗口且可能暴露损失，但成功也只产生证据，不自动获得全局能力 |
| 4. `request_governed_admission` | 提交模拟/试点证据，比较有范围准入、补证、拒绝或重提；读取准入范围与复核点 | 获得可追溯的局部能力使用条件、许可候选或开放倒计时/条件 | 治理等待、复核和合规成本可能延误收益；准入只作用于声明范围，缺授权、过期或范围漂移时停止并返回补证/重提 |
| 5. `inspect_market_information` | 查看跨区供给、需求、报价、信誉、路线与信息时间戳；比较本地生产、远端购买、延期和替代用途 | 形成带新鲜度和不确定性的交易判断，不把发现当成库存或到货 | 情报会过期、私有或不完整；错误判断会消耗机会窗口、信息成本或预备资金，玩家可改走本地/替代路线 |
| 6. `commit_route_and_escrow` | 比较报价/合同、路线、时间、损耗、目的地容量、escrow 锁定与里程碑；确认一条可追溯物理交付承诺 | 取得明确的进行中承诺、下一里程碑和取消/争议边界 | 资金、库存、运力和时间被占用；提交前条件漂移只能重报价/拒绝，不得静默换货、换路或提前结算 |
| 7. `observe_delivery_and_settlement` | 逐阶段查看出发、在途、到达、目的地存储与里程碑；选择等待、补救、争议、退回或替代用途 | 每个里程碑产生一次可归因结果；最终接收后才形成相应用途与 progression 方向 | 延误、损耗、目的地拒收或对手违约保留已占用/已损失价值与证据；下一步是恢复、争议、重提或回到本地路线，而不是重复领取奖励 |

循环完成的标准不是“看到一个报价”或“完成一次模拟”，而是玩家能回答：我改变了哪项能力或合同、投入和锁定了什么、哪一事实让它失败、还保留什么、下一次回来应做什么。任何模型建议、投票支持、旧缓存、市场热度或链上 receipt 都不能单独被表达为世界能力、到货或结算完成。

## 3. 工业能力创建：提案、模拟、试点与准入

### 3.1 四个阶段必须可区分

| 阶段 | 玩家可读承诺 | 不能推断的结果 | 可恢复动作 |
| --- | --- | --- | --- |
| 可审计提案 | 来源、假设、边界、预期影响、风险、证据缺口和撤回条件 | 没有世界写入权、全局可用性、默认配方、收益或治理资格 | 补证、缩小范围、拆分目标、撤回或改走既有能力 |
| 确定性模拟 | 在同一权威输入和声明范围内比较资源消耗、产出预期、失败路径与敏感假设 | 不扣库存、不占容量、不创造 recipe/设施、不发奖励或能力 | 修改假设后重新模拟、选择观测、停止或进入受限原型 |
| 原型/受限试点 | 在明确 scope、期限、资源上限和退出条件下验证“能否运行”和风险 | 不取得永久许可、全球市场资格、世界规则写入或外部价值 | 降低范围、暂停、回收未消费投入、记录失败证据并重提 |
| 治理准入 | 证据、审查结果、允许用途/区域/期限、复核点和撤销条件 | 不取得范围外权力、永久垄断或绕过公共安全/既有权利 | 补证、申诉、重提、等待复核或回退到既有能力 |

玩家至少能比较 `continue_proposal`、`refine_scope`、`run_deterministic_simulation`、`start_limited_pilot`、`request_admission`、`defer` 与 `abandon` 的时间、资源、锁定、风险和保留价值。准入或试点成功只让声明范围内的下一动作可用；任何表面、Agent 或离线文本不得把其中一个阶段跳写成最后阶段。

### 3.2 失败恢复和滥用边界

- 输入缺失、模拟不可复现、原型超过资源/范围、审查证据失效或治理资格不足时，失败必须保留提案来源、已消费/未消费投入、失败原因和下一次复查条件；不得自动重试、自动扩大范围或把失败试点当成正面 evidence。
- 模拟与试点的重复提交、重连、Agent retry、快照恢复和 replay 至多产生一份同义结果；不重复扣材、不重复发证、不复制准入、不继承旧奖励。
- 任何“模型生成配方”“高票支持”“历史先例”“一次试点成功”都不是单独的准入凭证。玩家需看到仍缺少什么、补齐后的下一步和不补齐时的安全停止。
- 受限准入的失败恢复是 `补证 -> 缩小 scope -> 再审查` 或回到已有生产/交易路线；没有安全的替代路径时，应显示 `no_safe_creation_recovery`，保留已有事实而不是自造默认能力。

## 4. 创作者许可、版税与开放

获准能力的收益权是可读、可审计、有限期的玩法奖励，不是永久产业封锁。玩家在确认许可或开放路径前，必须能比较：适用能力/用途/区域、开始和结束条件、版税/收益分配、维护责任、撤销/争议、开放条件以及不能保证的未来价值。

- `inspect_license_terms`：读取 scope、期限、用途、收益/版税、审计与复核点；这一步不锁定能力或资金。
- `accept_scoped_license`：接受一次明确范围的使用权，承担许可成本、合规和维护机会成本；超出范围必须重新准入。
- `choose_open_condition`：在已声明的到期或开放条件达到时，比较继续有限许可、转入公共开放或重新提出维护版本；开放不追溯取消已合法产生的历史收益或 receipt。
- `challenge_or_renew`：对错误归属、范围漂移、到期判断或收益结算提出可读争议/续期请求；待决请求不延长旧许可、不产生第二次权利。

许可到期、既定开放条件满足、撤销或 scope 缩小时，玩家看到通知、理由、当前可用动作、已有工作如何处置以及不失真的迁移/替代路线。获得许可权本身不等于版税已经结算：没有绑定能力使用/交付事实的 verified receipt 或有效 escrow 里程碑，不得发放、扣除或宣称版税收益。开放条件不能被单方隐藏地延长，创作者收益不能覆盖公共安全、玩家独立资产、身份或其他宪制保护；版税数字、期限长度、汇率和算法属于产品/经济/执行 authority，不在此冻结。

## 5. OC 转让与世界资格分离

玩家可以在链上转让 OC，但该动作只改变 OC 的链上所有权/结算事实。受限 OC 锁定、可撤回委托或控制人快照仅在已声明的区域 charter/地方治理规则要求时，才与持续运营、维护或交付形成的本地贡献共同评估；工业提案/准入、普通市场合同和紧急保供不自动继承该双资格门槛。它们不单独自动取得：

- 世界内区域资源、服务信用、库存、合约、市场信誉或交付资格；
- 工业提案的作者身份、治理准入、试点资格、区域 charter、tenure 或公共 levy 权利；
- 高影响竞争、生产、治理、紧急保供或任何外部兑换/收益承诺。

反向地，世界内的资源、信用、合同和资格也不自动变成 OC、可链上转让资产或可赎回外部价值。玩家比较 `transfer_oc`、`qualify_world_capability`、`earn_world_credit` 和 `enter_contract` 时，必须看到它们各自的条件、收益、不可替代之处和机会成本。OC 交易成功但世界资格不足时，系统应保留 OC receipt、明确资格 blocker 和补资格路径；资格成功但 OC 余额/来源不满足时，保留世界记录并返回资金/合同替代路线，不能把一侧成功伪造成另一侧成功。

## 6. 跨区发现、物理交付与 escrow 里程碑

### 6.1 发现不是到货

市场发现应同时显示供给/需求来源、报价或合约状态、信息时间戳/新鲜度、可见范围、不确定性、路线候选、预计损耗、目的地容量和下一复核时间。陈旧、私有或不完整情报必须显式标记，不能伪装成全局实时库存。

玩家必须能比较本地替代、远端报价、等待新情报和放弃交易。`discover_global_offer` 只产生可读情报；`accept_quote` 或 `commit_contract` 才产生范围明确的交易承诺；这两者都不表示货物已出发、已到达或需求已满足。

### 6.2 物理路线和目的地状态

确认合同后，玩家看到真实可审计的出发、在途、边/路线、时间、损耗/风险、目的地准入与存储占用。途中失败可以选择等待修复、有限改道、退回、交接、取消或争议；每个选择必须显示剩余货物/资金、已损失价值、重新占用的时间与容量，以及不允许该动作的原因。发现结果、报价、出发 receipt、到达、目的地存储和最终用途是不同的状态，前序不能覆盖后序。

到达、存储接收和所有权/合同结算必须分别可读：`arrival` 只证明货物抵达目的地边界；`storage_acceptance` 还要求目的地准入、容量与保管责任成功；`ownership_settlement` 只有在合同声明的接收事实与 escrow 里程碑均验证后才可成立。货物已到达但未被目的地接收，或已入库但所有权里程碑仍待决时，不得发放最终交付收益、版税或把它们合并成一个完成状态。

### 6.3 Escrow、里程碑与争议

资金或等价结算可以按声明的里程碑释放，但 escrow 不是物理交付的替代物。玩家能看到当前里程碑、释放/保留的理由、双方责任、证据窗口、争议入口和下一复查时间。

- 里程碑只有在与其绑定的物理/接收事实满足时结算一次；报价、链上转账、离线截图或单方 receipt 不足以完成里程碑。
- 争议待决时保留已确认的货物、资金、责任、证据和占用边界；不自动给任一方最终收益，不重复扣款/释放，不追溯改写此前已结算的合法里程碑。
- 交付失败或目的地拒收时，玩家比较补救、退回、改道、重谈、索赔/争议与放弃的时间和损失；没有合法路线时安全停止，并保留可恢复的 escrow/物理状态。

## 7. 进度、收益与平衡风险

本主题的长期收益不是无条件增加库存或货币，而是新增可复用选择：一项可审计的工业能力、一段有范围的生产/服务权、一个更可靠的跨区供应路线、一次争议后仍可恢复的合同信誉，或一条从创新回到独立经营的替代路径。奖励必须与实际完成边界对应：模拟/提案只给信息与 evidence 进度，受限试点只给试点结果，准入/许可只给其声明权利，物理接收和相应里程碑才给交易用途或交付 progression。

平衡与滥用风险至少包括：

- `grind_only`：玩家反复提交模拟/投票只换取进度，缺少新的决策、能力或恢复弹性。
- `pay_to_qualify`：OC 持有量直接成为世界资格，绕过贡献、证据、准入或目的地条件。
- `permanent_lock-in`：许可、版税或市场优势没有到期/开放/替代路线，创新者变成永久垄断者。
- `information_fiction`：陈旧报价被展示为实时库存，或者全球发现掩盖路线、损耗、容量和时延。
- `escrow_bypass`：报价、链上转账或客户端 receipt 直接触发交付奖励/最终结算。
- `route_multiplication`：重连、重试、并发争议或旧 receipt 重复释放资金、库存、奖励或需求减少。

验证样例必须证明创造路线不会成为首局唯一解，跨区物流不会取代本地专业化，失败后仍有独立可选路线，且玩家能判断继续投入是否值得。具体成本、价格、版税、阈值和奖励数值由相应专业 authority 另行决定。

## 8. 专业验收

### <a id="ac-game-018-01"></a>AC-GAME-018-01：工业创建阶段形成可恢复的玩家循环

给定一项新工业能力机会，玩家能够依次读取可审计提案、确定性模拟、受限原型/试点和治理准入的差异，比较继续、补证、缩小范围、延期、停止和既有路线；任一阶段均不自动取得世界能力。输入缺失、模拟不一致、试点失败、准入拒绝或条件漂移时，结果保留原因、投入/损失、scope、下一复查和安全替代，不产生第二次世界效果。

### <a id="ac-game-018-02"></a>AC-GAME-018-02：许可和开放奖励受范围、期限与公共边界约束

给定一个获准工业能力，玩家能看到用途/区域、期限、版税/收益、维护、撤销/争议与到期/开放条件，并在继续许可、续期、开放、替代路线之间作出有机会成本的选择。到期或开放条件达到后按规则开放；许可不能永久封锁、覆盖玩家独立资产/身份或扩大到未准入范围。重复、旧缓存和待决请求不延长、复制或追溯改写许可。

### <a id="ac-game-018-03"></a>AC-GAME-018-03：OC 转让与世界资格保持双向语义隔离

给定 OC 链上转让、世界资源/信用、工业准入或区域资格的任一组合，玩家能分辨各自的成功、失败、条件和下一步；OC 持有/转让不自动取得世界资格，世界资格也不自动铸造、转让或兑换 OC。任一侧失败都不抹掉另一侧已确认的 receipt，不产生绕过资格、治理、竞争或外部价值承诺的旁路。

### <a id="ac-game-018-04"></a>AC-GAME-018-04：市场发现、报价和物理交付有新鲜度与状态切线

给定跨区供给/需求，玩家能看到情报来源、时间戳/新鲜度、不确定性、报价/合同、路线、时间、损耗、目的地准入与容量，并区分发现、承诺、出发、在途、到达、存储和用途。陈旧或私有情报不会伪装成实时事实；前序状态不能表示货物已到达、需求已满足或最终结算。

### <a id="ac-game-018-05"></a>AC-GAME-018-05：Escrow 里程碑和争议可恢复且 exactly-once

给定一项带 escrow 的跨区合同，玩家能看到每个里程碑的物理/接收条件、可释放或保留的资金、证据窗口、争议状态和下一步。只有绑定事实满足时才产生一次里程碑结算；延误、损耗、拒收、争议、重连、重试、并发到达和 replay 不重复释放、扣款、发奖、减少需求或改写已结算历史，并保留合法的补救、退回、改道、重谈或安全停止路径。

### <a id="ac-game-018-06"></a>AC-GAME-018-06：跨 surface 玩家语义和 progression 边界一致

同一权威快照下，Viewer、pure API 和 Agent 对提案阶段、准入范围、许可状态、OC/世界资格、情报新鲜度、物流/escrow 状态、primary blocker、机会成本、`next_action`、`next_recheck` 和 `progression_effect` 给出等义解释。推荐不能替玩家创建新能力、改道、争议或完成结算；缺 authority 时统一返回 `incomplete / unknown / blocked`，不能用默认值补齐。

以上验收是目标玩法合同，不是当前实现或 release 证据。`test_tier_required` 应覆盖每一阶段的可读预览、fresh revalidation、原子拒绝、替代路径、OC/资格隔离、情报过期、路线失败、escrow 里程碑与重复请求；`test_tier_full` 再覆盖跨节点授权、持久化、并发、replay、争议恢复和跨 surface parity。

## 9. 权威边界与证据切线

| 语义 | 本合同拥有 | 其他 authority |
| --- | --- | --- |
| 玩家循环与动词 | 提案/模拟/试点/准入选择、许可取舍、市场发现/路线/交付/争议动作、收益和失败恢复 | 产品专题拥有世界规则和玩家承诺边界 |
| 工业与市场事实 | 只定义玩家需要理解的阶段与完成边界，不定义配方、价格、订单簿、物流、库存或 escrow 算法 | `world-runtime` / `world-simulator` 拥有确定性执行、资源、物流、状态、receipt 和 replay |
| OC 与身份 | 明确 OC 链上转让与世界资格不可互推 | `p2p` / blockchain authority 拥有链上资产、身份、签名和分布式状态 |
| 许可与治理 | 定义范围、期限、开放/争议的玩家选择及不可永久封锁边界 | `producer_system_designer` 与产品层拥有世界治理、公共安全和长期经济总原则 |
| 表达和验证 | 规定玩家应看到的原因、机会成本、下一步和 parity 目标 | Viewer/API/Agent 拥有表达实现，QA 与 `doc/testing/prd.md` 拥有验证与 release 判断 |

当前代码、局部 fixture、单项历史 green、产品迁移完成或本 PRD 本身都不能单独把本合同标为 current playable、balanced、available 或 released。只有对应专业任务在同一候选上提供 fresh implementation / QA / playtest 证据，并通过统一 claim gate，才能更新公开状态。

## 10. Non-Goals

- 不规定配方、材料、产率、价格、税费、版税数值、汇率、订单簿、库存、物流寻路、损耗公式、容量、路线费率、escrow 释放公式或危机阈值。
- 不实现工业提案、确定性模拟、试点、治理准入、许可、市场撮合、物流、目的地存储、结算、争议或任何 runtime/API/schema/action。
- 不把模型生成、投票、链上持有、客户端 receipt、报价或历史先例变成世界写入、玩家资格、最终交付或 reward 旁路。
- 不新增第五产品模块、不改变现有目录 slug 或产品 PRD-ID，不扩展 `PRD-GAME-015`/`016` 的 authority，也不替代常态市场与紧急保供产品专题。
- 不宣称外部兑换、支付服务、OC 价值、投资收益、合规资格、当前可用性、发行或 release readiness。
