# 区域冲突、软赛季与可恢复损失

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：2026-09-14
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)

本文定义成熟世界中区域冲突、实体损失、软赛季与系统性恢复的长期产品结果。它不定义战斗数值、评分、时长、占领算法、资产字段、离线执行、赛季周期、匹配、runtime 状态机或当前战争 MVP 的实现结论。

## 设计适用性与生命周期闭合

- 设计判定：`simple-topic-exemption`（`PRD-only-sufficient`）。
- 设计适用性理由：本 PRD 已完整表达授权冲突、损失连续性和 repair/rebuild/pivot 恢复边界；具体冲突交互由 gameplay/runtime/Viewer authority 负责。
- 当前 GitHub task evidence：本次分类见 [Issue #3680 C4 设计判定](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652452993)，本次闭合要求见 [Issue #3680 accepted repair](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)。
## 1. 产品目标

竞争应制造真实的区域风险、战利品和重建选择，而不是让所有玩家永久安全或把失败等同于删除身份和历史。冲突是有界、可预期且可恢复的区域事件；玩家可以主动承担风险或支持参战者，非参与者、新玩家和无关区域仍保有基本发展与恢复空间。

赛季用于刷新竞争机会和区域制度窗口，而不是创建新服务器、抹掉世界历史或让运营方重写胜负。即使出现系统性危机，恢复也以同一世界中的宪制保护和玩家/Agent 项目为中心，不以 reset 或偏袒性 bailout 取代既有因果。

## 2. 宣战、参战范围与离线防御

- 攻击只能发生在先行宣告的区域 charter 冲突中。参战者必须是符合 charter 的注册组织或其他已授权主体，并在宣战时明确登记参战范围与可暴露的资产；未登记的资产和主体不能被事后追溯纳入普通冲突收益。
- 非参与者、新玩家、尚未成熟区域和其他未被宣告的区域不因邻近、同一世界或市场关系自动成为合法目标。冲突范围不得借组织关系、物流经过或临时资产转移扩张为无边界掠夺。
- 玩家离线时，已登记的防御 Agent 和设施可按可读、有界、会到期且可撤销的防御授权 envelope 运行。它只支持预先允许的防御/保护行为，不把离线状态变成主动扩大战争、无限反击或不可解释的自动损失。

## 3. 冲突结果、占领与战利品

- 冲突有明确时间窗口，但不要求预设胜利目标、总分赢家或强制通关。冲突期间实际取得并保有的有资格战利品、可转移领地或局部权利可以构成结果；到期冻结符合规则的结果，而不是制造追溯性新赢家。
- 领地或地点 tenure 的转移必须经过可验证的 contested occupation、持续占有和到期结算；最后时刻触碰、单次进入或未维持的占据不能取得长期权利。
- 实体战利品在法律/经济结果最终成立前，必须经物理路线提取并到达受规则保护的目的地存储。取得、发现或短时占有不等同于已安全交割、已完成所有权转移或可忽略物流风险。

## 4. 实质损失与重建

- 冲突可摧毁 chassis、装备、设施、库存、工作进度、区域控制或局部经营能力；这些损失必须有可读原因、范围、保留价值和后续选择，不能被包装为无后果事件。
- Agent 的稳定身份、来源和审计历史不会因 chassis 或经营资产损失而被删除、捕获为他人身份或洗去历史。重建需要世界内资源、时间、物流、地点/设施和适用授权，不能成为即时免费复活或经济旁路。
- 失败后的常态路线包括 repair、rebuild、pivot、撤离或重建区域能力；外部援助可以存在，但不应成为恢复身份、基本独立参与或重新开始经营的唯一正常路径。

## 5. 软赛季与世界连续性

- 软赛季可以刷新竞争窗口、公共项目、排行榜和部分区域性权利/资格，以防止既有优势永久锁定全部机会。
- 世界时间线、玩家与 Agent 身份、核心能力、可迁移价值、历史 receipt 和已确认世界因果在赛季间保持连续。赛季不创建新的权威世界、不执行全局资产清零，也不把历史责任或来源重写为不存在。
- 刷新对象、条件、生效边界和恢复/申诉必须可读、可审计且遵守宪制与区域 charter；它们不能成为运营便利下的临时没收或对非参与者的隐性惩罚。

## 6. 系统性危机与恢复项目

- 系统性危机通过宪制化 containment 限制扩散、保护基本连续性并保持可解释的权限边界；containment 不是重写历史、跳过申诉、把损失转嫁给任意玩家或宣布新世界的旁路。
- 玩家、Agent、组织和区域可提出或参与恢复项目，重新建立服务、物流、能力或受损公共条件。项目结果留在同一审计世界中，接受同一资源、资格、治理和反滥用边界。
- 不存在选择性 admin bailout：恢复支持必须按预先公开、可审计且可复核的规则提供，不能以身份、关系或临时运营裁量抹去已确认的竞争/经济因果。

### 6.1 恢复项目生命周期与结果边界

- 恢复项目遵循 `containment -> recovery_open -> recovery_in_progress -> recovery_completed | recovery_expired | recovery_blocked` 的产品生命周期。每次转换必须由当时的权威世界事实、项目作用域、资格、资源、治理与反滥用条件共同决定，并以可重放 receipt 或等价权威结果证明；本状态名不冻结 runtime schema。
- 项目开放时必须让受影响主体读到触发事实、目标与作用域、受影响对象、准入和退出条件、所需贡献、里程碑、截止或复核边界、申诉入口，以及失败、到期或阻断后的常态下一步。项目范围不能在参与后静默扩大。
- 玩家、Agent、组织或区域可以查看符合资格的项目并提交有界贡献；只有里程碑 receipt 确认后，贡献才产生该项目声明范围内的一次世界效果。提交、排队、在途、本地缓存或客户端显示成功都不等于贡献已生效、损失已恢复或补偿已取得。
- 专业合同可以允许尚未生效的贡献撤回、改投或重新提交，但必须明确其资源、顺位和机会成本后果。授权、政策、资格或作用域变化后，旧待决请求只能明确继续、拒绝、到期、保持待决或由主体重新显式提交，不得静默迁移到新授权或新项目。
- 恢复项目不得跳过 containment，不得越过未授权区域、未参与者保护或既有权利边界，也不得制造免费资源、资格、债务豁免、永久特权或 admin bailout。未确认贡献不能追溯改写既有损失、责任与历史；项目关闭后仍保留贡献来源、结果、申诉与 receipt 历史，重连、重试或跨入口重放不能重复生效。

## 7. 范围与权威边界

产品层定义 `宣战/参战登记 -> 有界离线防御 -> 冲突窗口 -> 占领或实体提取 -> 冻结结果与可恢复损失 -> 软赛季刷新或系统恢复项目` 的玩家和制度语义。

`game` 拥有战斗、战争、占领、奖励与平衡；`world-runtime` 拥有冲突资格、资产、物流、存储、身份、重建、时序、receipt 与恢复执行；`p2p` 拥有世界连续性、治理/签名和分布式状态；Agent/Viewer 专业域拥有防御执行与玩家表达；QA 拥有具体验证。当前战争 MVP 是更窄的专业实现与证据面，不得由本长期产品目标外推为已经满足本专题。

## 8. 产品要求与组合验收

<a id="req-wr-cc-001"></a>
### REQ-WR-CC-001：冲突范围必须由玩家可读的授权边界决定

- 性质：`目标要求`
- 适用条件：区域冲突宣战、参战登记和离线防御授权。
- 要求：产品必须让玩家在承担冲突风险前识别 charter、参战主体、暴露资产、时间窗口和非参与者保护；未被授权的主体或区域不得因邻近、物流或关系自动变成合法目标。
- 理由：玩家需要知道风险为何成立、可以保护什么以及何时可以退出，避免不可解释的扩大战争。
- 上位承诺：区域冲突、软赛季与可恢复损失。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)。
- 验收：[AC-1](#ac-1)、[AC-2](#ac-2)。

<a id="req-wr-cc-002"></a>
### REQ-WR-CC-002：损失必须保留身份连续性并提供可比较的恢复路径

- 性质：`目标要求`
- 适用条件：冲突、区域危机或经营能力受损后。
- 要求：产品必须说明损失范围、保留的身份/历史、世界内恢复投入和可用的 repair、rebuild、pivot 或撤离路径；不能把资产损失表达为身份删除或免费即时复原。
- 理由：失败应产生有意义的风险与选择，同时保留玩家继续参与的基础。
- 上位承诺：实质损失与重建。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)。
- 验收：[AC-4](#ac-4)。

<a id="req-wr-cc-003"></a>
### REQ-WR-CC-003：恢复项目只能按确认结果产生一次有界世界效果

- 性质：`目标要求`
- 适用条件：系统性危机、恢复项目开放或项目贡献提交。
- 要求：产品必须让受影响主体看到项目作用域、资格、里程碑、失败/到期/阻断后的下一步，并且只有权威 receipt 确认的贡献才能产生一次项目效果；重连、重试或授权变化不得静默越界或重复生效。
- 理由：恢复要修复可玩能力与世界连续性，不能成为免费资源、权限旁路或选择性 bailout。
- 上位承诺：系统性危机与恢复项目。
- 专业权威：[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)。
- 验收：[AC-6](#ac-6)、[AC-7](#ac-7)。

<a id="req-wr-cc-004"></a>
### REQ-WR-CC-004：占领与安全提取必须分别结算

- 性质：`目标要求`
- 适用条件：冲突时间窗口到期，且同时存在 contested occupation、持续 hold、战利品提取和物流中的结果。
- 要求：产品必须分别判断 contested occupation、持续占有、到期冻结、物理提取和受保护目的地存储；单次进入、最后时刻触碰、发现或短时占有不能替代满足占领条件或安全交割。结算结果必须让玩家读到哪些权利已经成立、哪些物品已安全交割、哪些仍在风险或物流中。
- 理由：占领和提取承担不同的玩家风险与收益，必须让结算保留可理解的选择和因果，避免把临时占有误报为长期权利或最终战利品。
- 上位承诺：冲突结果、占领与战利品。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)。
- 验收：[AC-3](#ac-3)。

<a id="req-wr-cc-005"></a>
### REQ-WR-CC-005：软赛季刷新必须保持世界连续性

- 性质：`目标要求`
- 适用条件：软赛季刷新竞争窗口、公共项目、排行榜或部分区域权利/资格时。
- 要求：产品可以刷新竞争机会和区域性资格，但必须保持同一世界时间线、玩家与 Agent 身份、核心能力、可迁移价值、历史 receipt 和已确认世界因果；不得创建新的权威世界、执行全局资产清零或追溯重写历史。刷新对象、条件、生效边界和恢复/申诉路径必须可读且可审计。
- 理由：赛季刷新应重新打开竞争机会，同时保留玩家投入、身份和责任的连续性，避免用 reset 或临时运营裁量替代世界因果。
- 上位承诺：软赛季与世界连续性。
- 专业权威：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)。
- 验收：[AC-5](#ac-5)。

## 9. 组合验收

<a id="ac-1"></a>
### AC-1：冲突范围保持在已声明授权内

- 覆盖要求：REQ-WR-CC-001。
- 给定：一个已声明区域 charter、参战主体和暴露资产的冲突。
- 当：冲突样例执行攻击或防御并遇到邻近但未登记的主体或区域。
- 则：样例先证明宣战、参战范围和授权边界，再产生攻击/防御结果；非参与者、新玩家与其他区域不会被扩张为合法目标。

<a id="ac-2"></a>
### AC-2：离线防御授权可读且有界

- 覆盖要求：REQ-WR-CC-001。
- 给定：玩家离线且存在或不存在有效的防御 envelope。
- 当：授权到期、撤销或允许的防御行为被触发。
- 则：玩家能读到授权范围、到期、撤销、可执行防御和拒绝/升级路径；无有效 envelope 时不发生静默自动防御或主动升级。

<a id="ac-3"></a>
### AC-3：冲突到期区分占有与安全交割

- 覆盖要求：REQ-WR-CC-004。
- 给定：冲突时间窗口内存在 contested occupation、hold、提取和物流中的不同结果。
- 当：时间窗口到期并结算结果。
- 则：样例可区分已满足的 contested occupation/hold/expiry、尚未完成的占据、已安全提取的战利品和仍在风险/物流中的物品。

<a id="ac-4"></a>
### AC-4：损失保留身份并给出恢复选择

- 覆盖要求：REQ-WR-CC-002。
- 给定：冲突摧毁 chassis、装备、设施、库存、进度或局部经营能力。
- 当：玩家查看损失并选择后续路径。
- 则：样例同时说明被毁资产、保留身份/历史、所需世界内重建投入和 repair/rebuild/pivot 等恢复选择；不把 chassis 破坏写成身份删除或免费即时复原。

<a id="ac-5"></a>
### AC-5：软赛季刷新不切断世界连续性

- 覆盖要求：REQ-WR-CC-005。
- 给定：相邻软赛季存在竞争窗口、公共项目、排行榜或部分区域权利刷新。
- 当：刷新边界生效并允许玩家继续参与。
- 则：竞争机会可以刷新，同时保持同一世界时间线、身份、核心能力和历史因果。

<a id="ac-6"></a>
### AC-6：系统性危机以 containment 和规则化恢复限制扩散

- 覆盖要求：REQ-WR-CC-003。
- 给定：危机影响多个区域或公共条件。
- 当：containment 和玩家/Agent 恢复项目启动。
- 则：样例限制扩散、保留审计和申诉，不产生 reset、历史重写或选择性 bailout。

<a id="ac-7"></a>
### AC-7：恢复项目在范围变化和重试下至多生效一次

- 覆盖要求：REQ-WR-CC-003。
- 给定：至少两个作用域不同的恢复项目及一项待决贡献。
- 当：项目开放、推进、完成、到期或阻断，并发生重连、重试、跨入口提交或授权/范围变化。
- 则：结果可读；同一贡献至多产生一次权威效果，待决请求不会静默越界，未参与或不合格主体不会取得恢复效果，失败项目仍提供 repair、rebuild、pivot、申诉或其他适用的常态下一步。

## 9. 验收追踪

| REQ / AC | owner | authority | evidence | evidence tier |
| --- | --- | --- | --- | --- |
| [REQ-WR-CC-001](#req-wr-cc-001) / [AC-1](#ac-1) | gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | [`doc/game/prd.md`](../../game/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/p2p/prd.md`](../../p2p/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | 宣战、参战主体、暴露资产、时间窗口和非参与者保护均在承担风险前可读，且未授权主体不会因邻近、物流或关系成为合法目标 | test_tier_full |
| [REQ-WR-CC-001](#req-wr-cc-001) / [AC-2](#ac-2) | agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | [`doc/world-simulator/prd.md`](../../world-simulator/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | 防御 envelope 的范围、到期、撤销、可执行防御和拒绝/升级路径可读；无有效 envelope 时不发生静默自动防御或主动升级 | test_tier_required |
| [REQ-WR-CC-004](#req-wr-cc-004) / [AC-3](#ac-3) | gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | [`doc/game/prd.md`](../../game/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/p2p/prd.md`](../../p2p/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | contested occupation、持续 hold、到期冻结、安全提取、目的地存储和仍在风险/物流中的物品得到分离结算，且没有把短时占有误报为最终权利或战利品 | test_tier_full |
| [REQ-WR-CC-002](#req-wr-cc-002) / [AC-4](#ac-4) | gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | [`doc/game/prd.md`](../../game/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/world-simulator/prd.md`](../../world-simulator/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | 资产损失、身份/历史连续、世界内重建投入和 repair/rebuild/pivot 恢复选择同时可读，且 chassis 损坏不被表达为身份删除或免费即时复原 | test_tier_full |
| [REQ-WR-CC-005](#req-wr-cc-005) / [AC-5](#ac-5) | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / qa_engineer | [`doc/game/prd.md`](../../game/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/p2p/prd.md`](../../p2p/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | 竞争窗口、公共项目、排行榜或区域资格可以刷新，同时保留同一时间线、身份、核心能力、历史 receipt 与已确认因果，且无新世界或全局清零 | test_tier_full |
| [REQ-WR-CC-003](#req-wr-cc-003) / [AC-6](#ac-6) | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / agent_engineer / viewer_engineer / qa_engineer | [`doc/game/prd.md`](../../game/prd.md); [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/p2p/prd.md`](../../p2p/prd.md); [`doc/world-simulator/prd.md`](../../world-simulator/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | containment 限制扩散，恢复项目保留审计和申诉，并且不产生 reset、历史重写或选择性 bailout | test_tier_full |
| [REQ-WR-CC-003](#req-wr-cc-003) / [AC-7](#ac-7) | producer_system_designer / gameplay_designer / runtime_engineer / blockchain_ops_engineer / agent_engineer / viewer_engineer / qa_engineer | [`doc/world-runtime/prd.md`](../../world-runtime/prd.md); [`doc/p2p/prd.md`](../../p2p/prd.md); [`doc/testing/prd.md`](../../testing/prd.md) | 恢复项目生命周期、范围变化、重连/重试/跨入口去重、非参与者保护、失败后的 repair/rebuild/pivot/申诉与至多一次权威效果均有组合证据 | test_tier_full |

## 10. Non-Goals

- 不规定战斗、战争、占领、战利品、排行榜、赛季、重建或恢复项目的数值、动作、字段、算法或具体阈值。
- 不承诺当前战争 MVP、任何当前赛季、离线防御、实体提取或系统恢复机制已经实现、平衡或可发布。
- 不允许冲突、赛季或恢复越过世界规则、安全边界、基本玩家保护或未参与者的保护范围。
- 不定义 runtime/P2P/Agent/Viewer 实现、运营 runbook、管理员裁量或外部赔偿。

## 11. 未决问题与证据边界

- 尚未决定：当前战争 MVP 何时具备足以验证本分册长期冲突/恢复承诺的完整候选；影响 AC-1 至 AC-7 的当前 verdict，决策负责角色为 `producer_system_designer` 联合 gameplay、runtime、Agent、Viewer 与 QA，触发条件是进入该产品承诺的实现或公开 claim 审查前，解决前临时不承诺本分册已实现。
- 本分册证据只能证明指定候选、入口、版本/窗口和环境中的组合行为；文档建档、局部专业 green 或历史样本不能证明当前发行就绪、真实玩家留存或所有战争/恢复机制已通过。
