# 组织连续性、解散与长期不活跃保护

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文定义组织持续经营、解散和长期不活跃时的玩家结果与制度边界。它补充成熟世界成长、Agent 所有权与冲突恢复分册；不定义治理权重、身份技术、时长、清算价格、estate schema、链上交易、runtime 状态机、运营处置或当前放行结论。

## 1. 产品承诺

组织可以围绕共同目标持续协作、重组或退出，但不能把成员、独立资产、Agent 或历史变成可任意抹除、秘密没收或失去因果记录的对象。解散和不活跃是同一持续世界中的可理解状态变化：玩家能知道触发原因、当前范围、保留价值、义务、异议或申诉，以及下一次可执行决策。

这项保护保持 world-first：结果以权威世界事实而非组织叙述为准；保持 emergence-first：组织可以在共同规则内协作、重组或恢复，而不能借不活跃取得旁路收益；保持 persistent / auditable：身份、来源、receipt 和已确认因果持续可追溯；保持 extensible：未来组织形态可扩展，但不得低于本文保护底线。

## 2. Charter 保护底线与解散顺序

- 组织 charter 可以定义宗旨、角色、内部工作流、成员资格、授权及收益或剩余分配规则，但不得排除成员对独立资产、已有合同、可理解退出、审计历史及 Agent 稳定身份/来源的最低保护。
- 解散、资不抵债或已授权的重组必须按公开、可审计的连续性顺序处理：先冻结进一步风险与越权处分；再履行、终止或结清适用合同；返还可识别的托管资产；处理债权、成本和责任；随后对 Agent、设施和持续业务作可读的转让、拍卖、重组或退休；最后才按 charter 和适用保护边界分配剩余。
- 每一步只处理已声明范围内的权利、义务和资产。组织角色、短时控制、内部多数或外部资产转移不能追溯消灭独立权利、已确认历史或他方已取得的权利。
- Agent、设施或业务的处置可以改变未来控制、经营角色或可用性，但不得删除 Agent 身份、来源、历史 receipt 或既有责任链，也不能把处置伪装成新的 owner 从未承诺过的历史。

## 3. 长期不活跃的保护与有限处置

- 长期不活跃本身不使个人或组织世界价值立即成为可任意夺取的公共财产。进入任何保护或后续处置前，必须有可读通知、保护期和可恢复主张入口；通知不可达、成员缺席或短时停摆不是自动放弃。
- 在保护期内，世界只可采取维持基本连续性、隔离新增风险或保全可识别价值所需的最小动作；不得静默重新分配控制权、清空资产、取消历史或把临时保护表达为最终没收。
- 保护期后，适用规则可以形成 estate 或可撤销 delegation，并按公开的阶段处理维护、风险隔离、有限重启或 reclaim。每个阶段都必须说明触发事实、当前范围、保留价值、未决义务、异议/申诉与下一次可决策点。
- reclaim 或申诉只按当时仍有效的资格、证据和保护边界重新评估。重复请求、重连、短暂转让或旧通知不得制造第二次恢复、重复分配或绕过仍未结清的义务。
- 只有不可识别、已履行或已按适用程序终结的剩余事项，才可以进入 charter 与宪制边界允许的后续处置；此后续处置不追溯改写此前 receipt、责任、合同结论或历史因果。

## 4. 玩家可读状态与相邻边界

组织成员、相关权利人和受影响 Agent 必须能区分：正常持续经营；通知或保护中；estate / 可撤销 delegation 下的受限持续；可 reclaim 或申诉的待决状态；以及已解决的重组、退出、处置或解散结果。任何未最终确认的保护、登记、请求或建议均不得表示为控制权、资产返还、义务解除或处置已经完成。

本文定义 `持续经营 -> 风险冻结或保护 -> 合同/托管/责任处理 -> 持续业务处置或有限恢复 -> 剩余分配或后续处置` 的产品语义。`game` 拥有玩法、经济和相关平衡；`world-runtime` 拥有资格、状态、执行、receipt、去重与恢复；`p2p` 拥有治理授权、签名和分布式状态边界；QA 拥有具体证据和 verdict。它不取代 Agent 所有权分册的控制权规则、冲突分册的战利品/恢复规则，或专业域的实现合同。

## 5. 组合验收

- OC-1：代表性 charter 样例证明组织配置不能越过成员独立资产、已有合同、可理解退出、审计历史和 Agent 身份/来源的保护底线。
- OC-2：解散或重组样例按风险冻结、合同处理、托管返还、债权/成本/责任、Agent/设施/持续业务处置和剩余分配的顺序运行；每一步保留 receipt、来源和责任链，不产生历史删除或越权处分。
- OC-3：长期不活跃样例证明通知、保护期与可恢复主张先于 estate 或可撤销 delegation；维持、风险隔离、有限重启或 reclaim 只在明确范围内发生，不将临时保护伪装为最终没收。
- OC-4：reclaim、申诉、重复提交、短暂转让和重连的负例不产生第二次世界效果，不恢复已失效授权，也不绕过未结清义务；已解决或被拒绝结果保持可追溯。
- OC-5：受影响主体能够区分正常、保护、受限持续、待决 reclaim/申诉和已解决状态，并获得适用的异议、申诉、等待、重新规划或安全停止路径，而不把未确认结果表述为已完成。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| OC-1 / OC-2 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-GAME-002 / PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | charter floors、解散 waterfall、Agent/设施处置、独立权利及历史/receipt/责任连续性的组合证据 | test_tier_full |
| OC-3 / OC-4 | producer_system_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-003 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | 通知/保护期、estate 或可撤销 delegation、有限处置、reclaim/appeal、去重和非追溯负例 | test_tier_full |
| OC-5 | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 正常/保护/受限/待决/已解决状态的权威对账与正式玩家 surface 可读性组合证据 | test_tier_required |

## 6. Non-Goals

- 不规定不活跃时长、通知渠道、保护期、资格评分、治理权重、清算/拍卖价格、债权顺序或剩余分配公式。
- 不实现 charter、estate、delegation、reclaim、申诉、资产处置、账户/身份聚合、签名、runtime/P2P 状态机、Viewer 控件或运营 runbook。
- 不允许组织、普通治理、紧急机制或运营便利越过本文保护底线，制造静默没收、历史重写、身份删除、无审计处置或重复世界效果。
- 不以本文、历史任务状态或局部证据声称组织连续性能力当前已实现、已平衡、可玩或可公开发布。
