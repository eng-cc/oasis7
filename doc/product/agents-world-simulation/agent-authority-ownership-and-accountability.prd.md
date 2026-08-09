# Agent 权限、资产与责任连续性

## 文档身份

- 所属产品模块：智能体与世界模拟
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)

本文定义玩家或组织拥有 Agent 的长期产品承诺：授权如何持续、资产如何扩张和转让、Agent 如何提出异议，以及结果如何归因。它不定义 runtime 状态机、授权字段、签名、模型行为、界面、数值成本或测试步骤。

## 1. 产品目标

玩家经营的不是只能逐项等待确认的遥控单位，也不是无法纠正的黑箱。一个 Agent 可在玩家选择的高自治或有界授权模式下持续推进世界目标；玩家始终能理解授权范围、主要风险、实际结果与下一次纠正或撤销机会。

Agent 是可长期持有、扩张和转让的经营资产，但其身份、来源和已影响世界的历史不能被交易、组织重组或竞争损失洗除。控制权、经济权和责任必须在同一条可审计世界因果链中表达。

## 2. 授权与自治边界

### 2.1 可选自治模式

- 玩家或具有相应授权的组织可以为 Agent 选择高自治或有界模式；两者都受同一世界规则、资源、权限、预算和治理边界约束。
- 高自治允许 Agent 在已声明的目标和风险范围内自行安排受支持行动，不等于获得额外世界权力、跳过权威行动节奏，或把 Agent 的推测写成已发生结果。
- 有界模式允许玩家把目标、行动类型、对象范围、资源/风险上限和持续期间收窄；它不是要求玩家逐动作手工确认的默认伪装。

### 2.2 严重后果的提前授权

- 会造成重大资源暴露、权利移交、不可逆损失、冲突升级或其他高后果承诺的行动，必须有一份可读、范围明确、会到期且可撤销的提前授权，或在行动前获得同等有效的确认。
- 提前授权至少限定允许的行动/目标、预算或数量上限、风险类别、有效期间和撤销路径；超出范围、授权到期、撤销生效或世界硬边界阻断时，Agent 必须暂停、说明原因并给出升级、改道或恢复选择。
- 对会消耗资源、占用可争用容量、转移经济权利或累积风险的提前授权，预算或数量上限必须表达为该授权有效期间内的**累计**可用范围，并说明适用的资源/权利来源及对象范围；它不能仅被理解为“每次行动各自不超过上限”。每次提交和权威接受都必须按当时有效的专业合同重验该授权的剩余范围、来源与其他前置条件。拆分请求、并发、自动重试、重连、切换 Agent/owner 或局部缓存均不得把同一授权扩展成多份上限、隐藏债务或额外优先权。
- 授权已存在不代表必然成功：权威世界仍可因资源、权限、治理、安全或竞争条件拒绝、改道或延后行动。

### 2.3 待决高后果行动与授权变更

- Agent 可以为有效授权内的高后果行动准备方案或发起待决请求，但在权威世界确认结果前，这不构成资源已暴露、权利已移交、损失已发生或旧 owner 可继续控制的承诺。客户端排队、Agent 记忆、局部预留或“已送出”提示都不能绕过该边界。
- 授权到期或撤销、控制权转让生效，或出现新的世界/安全硬边界时，尚未产生权威世界效果的待决行动必须按当时有效的授权和前置条件重新评估。它只能明确地继续、被拒绝、过期、取消，或在有权主体作出新的确认后以新请求继续；不得静默沿用失效授权、自动重放，或把旧请求伪装成新 owner 的选择。
- 已有权威世界结果不会因后续撤销、到期或转让被改写为未发生。玩家能查看该结果所依据的授权/确认、实际生效时点和后续授权变化；撤销是面向未来的控制，不是删除历史、逃避已生效义务或追溯收回他人已取得的权利。
- 当待决请求因授权变化而无法继续时，玩家能做的是等待可验证结果、明确取消，或在仍有相应权限时重新确认/调整目标；玩家不能通过重新连接、切换客户端、复制请求或短暂转让 Agent 来保留旧授权的优先权、预算或世界效果。具体待决状态、去重、确认、取消和 receipt 字段仍由 `world-runtime`、`world-simulator` 与 P2P 专业合同定义。

## 3. 团队规模与长期资产

- 早期体验以一个可理解的主 Agent 为经营锚点；后续可以形成团队、设施角色和授权层级。
- 额外 Agent 是可持有、可调度且可转让的长期经营资产，但取得与维持团队必须承担显著的时间、资源、维护、授权和协调复杂度；产品不得把 Agent 数量扩张成无成本的线性囤积权。
- 玩家或组织取得额外 Agent 的常态路径是向世界内工业能力下单制造；订单消耗显著资源、占用适用产能并经历可读交付时间。供给主要受工业、物流、设施与交付约束，而不是由普通治理 quota 直接发放或扩张；适用资格与安全边界仍可限制订单，但不能把治理影响力伪装成无成本 Agent 供给。
- 组织层级可以提高经营范围和协调能力，却不能取消小规模玩家的独立路线、绕过区域/治理资格，或把成员数量自动转为额外世界权力。

## 4. 转让后的连续性与重配置

- Agent 的可转让权利包括受规则约束的控制、经济、经营角色和关联资产权益；转让不会抹去其稳定身份、来源、审计历史或已影响既有世界决定的记忆/历史。
- 新 owner 可以自转让生效点起设定新目标、角色策略、Prompt override 和工作上下文；既有配置保留为可追溯快照，不能被伪装成新 owner 的原始选择。
- Agent 的退休、报废或失去可操作性必须保留可审计的历史结果；产品不把删除身份、来源或责任记录作为资产处置、竞争损失或组织重组的正常结果。

## 5. 异议、override 与责任

- Agent 可以提出风险、异议、证据和替代方案，并在授权范围外、需要新的高后果承诺或遇到世界/安全硬边界时请求升级或暂停。
- Agent 的偏好、预测或异议不会单独否决一项已在 owner 权限和有效授权范围内、且未被世界规则或安全硬边界阻止的行动。有效 owner override 必须执行，并留下包含异议、授权/override 与结果的 receipt。
- 每项高影响结果应能区分 Agent 的建议或执行、owner 的确认/override、组织策略或指令，以及设施/其他执行载体的实际作用。owner 对其有效 override 负主要责任；组织在授权、强制或实质受益时承担相应共同责任；Agent 不得被用作掩盖 owner 或组织责任的替罪对象。

## 6. 范围与权威边界

产品层定义 `资产/身份连续性 -> 自治或授权范围 -> Agent 建议/异议 -> owner 或组织决定 -> 权威世界校验 -> receipt 与责任归因 -> 纠正、撤销或恢复` 的玩家和制度语义。

`game` 拥有玩法成长、团队成本与反支配平衡；`world-runtime` 拥有授权、转让、执行、receipt、审计与恢复的确定性规则；`world-simulator` 拥有 Agent 行为、Prompt、provider 与玩家 surface；QA 拥有具体证据和验证方法。产品层不以本承诺声称任何机制当前已实现或可对外发布。

## 7. 组合验收

- AC-1：代表性目标可在高自治和有界授权两种模式下运行，玩家均能读到授权范围、当前状态、主要风险、实际结果及撤销、纠正、改道或恢复的下一步。
- AC-2：高后果行动缺少有效提前授权或确认、超出范围、到期或撤销后，不会静默执行；正式结果能区分授权拒绝、世界规则拒绝、资源/竞争阻塞和可用替代路径。
- AC-9：代表性资源消耗、可争用容量、经济权利或累积风险行动证明，高后果授权的预算/数量上限在整个有效期内按同一资源/权利来源和对象范围累计执行；拆分、并发、重试、重连或控制权切换不能重置、复制或绕过剩余额度，也不会产生隐藏债务、额外优先权或第二次世界效果。余额不足、来源/对象不匹配或授权失效时，玩家能读到实际 blocker 与升级、改道、等待或重新确认中的适用下一步。
- AC-8：待决高后果行动样例能区分“已发起但尚未生效”与权威世界结果；授权到期/撤销、控制权转让和新硬边界仅使未生效请求重新评估或明确终止，不能静默续行、重放或追溯改写已生效结果。玩家可读到后续可执行的取消、重新确认或调整路径及其 blocker。
- AC-3：从一个主 Agent 扩张到团队的样例表明额外 Agent 带来取得、维护、授权和协调约束，而非无成本的自动产能或世界权力；小规模独立路线仍可继续。
- AC-7：额外 Agent 的样例证明玩家或组织通过世界内工业订单承担资源、产能和交付时间，并在取得后继续承担维护、授权与协调成本；普通治理 quota 既不直接发放 Agent，也不替代工业供给约束，且早期一个主 Agent 的经营锚点保持不变。
- AC-4：转让样例同时证明控制/经济/角色策略可从生效点重配置，以及身份、来源、审计历史和既有决策历史保持可追溯；处置不会伪装成历史删除。
- AC-5：异议、世界/安全硬阻断和有效 owner override 在正式世界结果中可区分；有效 override 的 receipt 关联异议、授权、执行结果和恢复路径。
- AC-6：高影响结果可追溯到 Agent 建议/执行、owner 决定、组织策略和执行载体中的适用因素；责任表达不把 Agent 当作 owner 或组织的替罪对象。

## 8. 验收追踪

| 产品承诺 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| AC-1 / AC-2 | agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 授权范围、到期/撤销、严重后果升级、世界拒绝与恢复的组合证据 | test_tier_required |
| AC-9 | producer_system_designer / agent_engineer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同一提前授权下的累计上限、资源/权利来源和对象范围再校验；拆分/并发/重试/重连/控制权切换负例、无隐藏债务或第二次效果，以及玩家可读 blocker 与恢复路径的组合证据 | test_tier_full |
| AC-8 | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / blockchain_ops_engineer / qa_engineer | `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 待决与 committed 的区分、授权变化后的重新评估/取消/重确认、去重/非重放、转让边界、receipt 时点与硬边界负例 | test_tier_full |
| AC-3 | gameplay_designer / agent_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 主 Agent 到团队的成本/维护/协调约束与小规模独立路线样例 | test_tier_required |
| AC-7 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / qa_engineer | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 工业订单、资源/产能/交付、非治理 quota 供给、维护/授权/协调与早期单 Agent 锚点组合证据 | test_tier_full |
| AC-4 / AC-5 | agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 转让前后策略连续性、历史不可洗除、异议/override receipt 与硬阻断负例 | test_tier_required |
| AC-6 | producer_system_designer / agent_engineer / runtime_engineer / qa_engineer | `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 高影响因果 receipt 的责任分层、审计与纠错/申诉证据 | test_tier_full |

## 9. Non-Goals

- 不规定具体授权 envelope schema、风险分级、时长、成本、团队规模、签名或 receipt 字段。
- 不定义 Agent 的模型架构、价值判断、记忆存储、Prompt patch、训练算法或 provider 准入。
- 不决定具体战争、市场、组织或设施玩法平衡，也不将 owner override 扩张为越过世界规则、安全边界或他人权利的能力。
- 不实现 Viewer 控件、告警、审批流、审计界面或测试脚本，也不声明当前发布或可玩性结论。
