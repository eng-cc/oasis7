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
- [`区域冲突、软赛季与可恢复损失`](chartered-conflict-soft-seasons-and-recovery.prd.md)：宣战、有限参战范围、实体战利品/占领、可恢复重建和不重置世界的软赛季边界。
- [`沟通、合同、声誉与 R&D 连续性`](communication-contracts-reputation-and-rd-continuity.prd.md)：人类沟通与 Agent 合同的边界、持续服务争端、情境声誉及研究归因/份额的长期产品语义。
- [`常态市场与有界紧急保供`](market-normal-state-and-emergency-supply.prd.md)：常态价格形成、系统性必需品危机的最小授权包、受限干预和可审计退出边界。
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

本产品层只定义上述玩家承诺和端到端边界；玩法动作粒度由 [`doc/game/prd.md`](../../game/prd.md) 与其核心玩法骨架拥有，物理/执行真值由 `world-runtime`，表现真值由 `world-simulator` 的对应专业域文档拥有。

世界只有一条持续、权威的时间线和一套可测量的物理真值。厘米级距离、顺序、成本与持久化结果可以由工业、物流、治理等粗粒度子系统消费，也可以由 Viewer 做可读性抽象，但任何映射都必须确定、可追溯，不能因子系统分辨率或视觉夸张而改写权威结果。

间接控制不等于旁观：对当前受支持的玩家意图，系统必须呈现意图是否被接受、Agent 如何解释并执行、主要世界后果，以及玩家可用的打断、重排、纠正、fallback 或恢复动作；不能以 Agent 自主性为由隐藏因果或让玩家失去下一次决策权。

### Data 所有权与授权边界

Data 是有归属、有获取成本且受授权边界约束的世界资源。未经授权的使用必须原子失败，不产生未授权收益；产品体验需要说明成本、归属、用途、授权状态和可恢复的授权或替代路径，且可读性层或 Agent 自动化不能静默绕过权限。

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

### 世界宪法级产品不变量

- 玩家通过目标、Agent、地点、设施、配方、关系与治理等受支持动作获得间接战略能动性；资源变化必须来自被授权的 source/sink 因果链，不能凭空生成或绕过成本。
- 每个权威行动都必须经过规则与权限校验并产生可审计后果；玩家能够读懂 target、action、cost、blocker、result、next decision 与 recovery，不靠隐藏状态猜测世界为何变化。
- 社会关系、组织、市场与制度可以在权限、治理和 anti-abuse 边界内产生有限涌现，但不会因此获得绕过权威规则的能力。世界保持持久、开放式演化，不要求强制终局。
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
- SC-5：玩家的间接战略动作具备授权资源因果、权威校验和可审计后果，界面或接口可读 target/action/cost/blocker/result/next/recovery；涌现关系与组织不绕过权限、治理或 anti-abuse，未支持细粒度请求有 canonical 替代或安全停止。
- SC-6：代表性间接控制流程证明玩家意图进入 Agent/策略决策，经权威规则与资源校验产生世界后果，并返回可解释结果与可执行的打断、纠正、下一步或恢复动作；任一专业域的局部 green 不能替代组合闭环。
- SC-7：同一物理行动在 gameplay、runtime、Agent 与 Viewer 的粗粒度/表现映射中保持距离、顺序、成本和持久化结果一致，不产生第二条时间线或表现层真值；权威时间线在没有直接玩家输入时仍按当前世界规则继续推进，不冻结具体 tick 时长。
- SC-8：Data 的获取和一次授权使用路径可端到端验证；未经授权的使用原子失败且不产生旁路收益，并向玩家提供可理解的原因和恢复或替代路径。具体许可状态机、结算规则与副作用矩阵由专业域拥有。
- SC-9：成熟世界样例证明小规模玩家在不立即依附 major power 的前提下，通过可归因贡献获得新选择、恢复弹性、议价位置或区域用途；失败保留 repair / rebuild / pivot，区域影响不越界为全局治理权。
- SC-10：产品样例证明世界没有强制通关条件，但每个阶段成果具有完成边界、可归因世界后果与下一阶段方向；长期成果只在新增选择、恢复弹性、局部议价/协调位置或区域用途时成立，不能以库存、吞吐或重复次数冒充成长。
- SC-11：代表性首局、后引导与成熟世界样例保持一个当前主目标与低负担的继续/分支/换向选择；作用域、canonical 转译、校验、治理、反支配与审计在后台执行，只有改变资源、权限、锁定、恢复或共同承诺时才以可读原因和替代路径进入前台。
- SC-12：代表性首局、持续目标与成熟世界样例证明玩家可通过短命令/复盘和有边界的离线授权维持普通成长，同时可自愿进入较长的区域、外交或治理会话；玩家无需先掌握全部系统深度或保持持续在线，且高风险窗口、授权范围与恢复路径可读。
- SC-13：代表性区域冲突与赛季样例证明攻击只在已声明的 charter 范围和登记参与者/暴露资产间发生，非参与者受保护；实体领地/战利品结算、可恢复损失、软赛季刷新和系统性恢复均保留同一世界时间线、身份与可审计因果。
- SC-14：代表性协作样例证明直接人类沟通不自动绑定，而 Agent 代表在有效授权、接受与权威校验后可形成可审计合同；持续服务的违约/救济、随机无冲突本地争端程序、情境化可更新声誉和预声明 R&D 归因均不产生永久污点、隐性权力或对个人政治 credential 的转让。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | gameplay_designer | PRD-GAME-004 | `doc/game/prd.md` | 首局玩法与可读性证据 | test_tier_required |
| SC-2 | gameplay_designer | PRD-GAME-004 | `doc/game/prd.md` | micro-loop 端到端回归 | test_tier_required |
| SC-3 | gameplay_designer | PRD-GAME-007 | `doc/game/prd.md` | PostOnboarding 转换与持续游玩证据 | test_tier_required |
| SC-4 | qa_engineer | PRD-GAME-003 | `doc/game/prd.md` | PRD-ID 到发布验收证据的追踪检查 | test_tier_required |
| SC-5 | producer_system_designer | PRD-GAME-002 / PRD-GAME-004 / PRD-GAME-013 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-P2P-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/p2p/prd.md` | 授权资源因果、权威后果、玩家可读闭环、有限涌现与替代动作跨域审计 | test_tier_required |
| SC-6 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer | PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-WORLD_RUNTIME-001 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md` | 玩家意图、Agent 决策、权威后果与打断/纠正/恢复组合证据，含正式玩家 surface 的 S6 交互闭环 | test_tier_required |
| SC-7 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 无玩家直接输入时的持续时间线，以及物理真值与粗粒度/表现映射一致性审计，含 S6 表现层核对 | test_tier_required |
| SC-8 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | Data 获取、授权使用、未经授权原子失败与玩家恢复路径证据，含 S6 拒绝/恢复可读性 | test_tier_required |
| SC-9 | producer_system_designer / gameplay_designer / qa_engineer | PRD-GAME-015 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/testing/prd.md` | mature-world player leverage、anti-grind、恢复与有限区域影响 fresh sample；产品合同见本模块的 mature-world 专题分册 | test_tier_full |
| SC-10 | producer_system_designer / gameplay_designer / qa_engineer | PRD-GAME-007 / PRD-GAME-015 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/testing/prd.md`; 本模块的首局与成熟世界专题分册 | 阶段成果、三条长期抱负轴、anti-grind 与无强制终局的组合审计 | test_tier_required |
| SC-11 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer | PRD-GAME-004 / PRD-GAME-007 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 一个当前主目标、继续/分支/换向与仅在实质相关时显现的后台护栏组合证据 | test_tier_required |
| SC-12 | producer_system_designer / gameplay_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 分层信息、短命令/复盘、已授权离线推进、可选深度会话与高风险有界响应的组合体验证据 | test_tier_required |
| SC-13 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/chartered-conflict-soft-seasons-and-recovery.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 参战范围、离线防御、占领/提取、可恢复重建、赛季刷新与统一世界连续性的组合证据 | test_tier_full |
| SC-14 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/product/world-rules-core-gameplay/communication-contracts-reputation-and-rd-continuity.prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 授权/接受合同、atomic 与持续服务、争端 receipt/程序申诉、声誉/转让与 R&D provenance/份额的组合证据 | test_tier_full |

## 6. Non-Goals

- 不在产品层冻结新的玩法细则、数值、掉落或成长曲线。
- 不把分布式执行、任意 WASM 或全局治理包装成当前玩家默认能力。
- 不复制 `game` 专题 PRD、project 任务或测试步骤。
