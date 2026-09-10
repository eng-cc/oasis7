# 首局与持续游玩产品设计

## 文档身份

- 配对产品 PRD：[`first-session-and-continuation.prd.md`](first-session-and-continuation.prd.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`gameplay 工业 walkthrough 合同`](../../game/gameplay/gameplay-industrial-representative-execution-walkthrough.prd.md#3-五阶段玩家-walkthrough)、[`gameplay 首产物结算合同`](../../game/gameplay/gameplay-industrial-starter-completion-contract.prd.md#3-五阶段结算表)、[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md#4-technical-specifications)、[`world-runtime 专业 PRD`](../../world-runtime/prd.md#consumer-compatibility-industrial-profile-and-stage-execution)、[`testing 专业 PRD`](../../testing/prd.md#6-validation-decision-record)

本文说明玩家怎样理解并继续一条首局工业因果链。配对 PRD 拥有产品要求和验收；专业 authority 拥有玩法顺序、结算、状态、receipt、接口和测试实现。本设计不把目标态写成当前实现或发行承诺。

## 1. 设计命题

目标体验是让第一次进入世界的玩家从一个可解释的工业目标走到真实交付，并在阻塞、重连或完成后仍保有下一次选择。玩家需要知道目标、当前权威状态、已保留或消耗的价值、主要 blocker、完成边界和下一步。

核心决策是：在 feasibility card 与当前事实支持的范围内，选择等待、补足、改源、改配方、改道、延期或继续推进哪一条路径。选择必须有可读的机会成本；预览和推荐只提供比较信息，不产生世界效果。

设计假设是先建立一条可归因的因果链，再展开持续能力分支，能帮助玩家判断是否继续。该假设需要对应入口的体验证据验证，结构检查或局部自动化不能替代玩家体验结论。

## 2. 代表性片段与阶段信息

首局的代表性片段是：玩家查看工厂、比较配方、准备原料、等待或改道物流、确认多输入齐套、排程、观察生产 receipt，再等待独立的交付 receipt。任何缺少关键 authority 的环节都保持 unknown/blocked，并返回补证、修复、等待或重新定目标的路径。

| 阶段 | 玩家知道什么 | 可以选择什么 | 成本与承诺 | 反馈与下一步 |
| --- | --- | --- | --- | --- |
| Gate 与目标 | candidate 是否由同一份 fresh authority snapshot 支持，以及完成边界、主要风险和复查点 | 接受候选、查看阻塞、改候选或回到其他目标 | Gate 不扣资源、不锁容量、不排程；unknown 不被填成可达 | `candidate_available` 或 `no_safe_starter_chain`；后者给出最早 blocker、保留价值与复查点 |
| 工厂与配方 | 工厂能力、配方适用性、原料来源和运输风险 | 比较可用配方、处理前置或延期 | 比较不占库存；提交后仍须按当前 authority 重验 | 接受、拒绝或重新报价；下一步是准备真实输入 |
| 原料与物流 | source/refinement 是否形成带 lineage 的 `ready_for_logistics` 批次；在途和到达是否已结算 | 获取、精炼、补充、减量、等待容量或改道 | 消耗、占用、损耗和未满足量保持可见；source 结果不等于到达 | `ready_for_logistics`、in-transit、arrival、applicable 各自独立；下一步是齐套或恢复 |
| 齐套与排程 | 所有独立输入是否已 settlement、到达且适用；排程是否只是接受 | 等待缺失输入、补齐、重排、换候选或结束意图 | 先到输入可保持有界等待；排程接受不等于生产开始 | 主 blocker、已保留价值和下一复查点可读 |
| 生产与交付 | `accepted/scheduled`、执行、`produced/undelivered`、delivery/terminal settled 的差别 | 等待、处理输出去向、交付或延期 | production receipt 只证明生产结果；交付用途等待匹配 settlement | 独立 receipt 可回指同一因果链；交付完成才打开目的地后果 |
| 阶段承接 | 已形成的能力、仍在的约束和当前主目标 | 选择扩张、稳定/恢复、专业化/服务或主动换向 | 新方向承担新的资源、容量、风险和机会成本 | 下一 session 有明确第一动作；旧目标的义务和结果仍可追踪 |

状态优先级先保护安全、权利和授权，再保护不可逆损失、资源扣减和锁定，再处理可恢复前置与可选信息。状态不一致时只能给出复核、恢复、安全停止或重新定目标；状态变化后旧动作必须重新判断。

## 3. 机制与体验关系

| 体验目标 | 玩家决策 | 权威约束 | 预期互动结果 | 失败信号 |
| --- | --- | --- | --- | --- |
| 建立信任 | 是否采用当前 starter candidate | Gate、profile、factory、recipe、input、path、power 与 terminal authority | 一条有稳定 identity 和完成边界的可解释链路 | `no_safe_starter_chain`、unknown 或最早 blocker |
| 理解投入 | 是否获取、精炼、等待或改源 | source/owner ledger、规格、容量、电力和物流合同 | 资源去向、保留、消耗和未满足量可回看 | 证据不足、争用、低电、容量或适用性失败 |
| 归因结果 | 是否继续、恢复、交付或换向 | production receipt 与 delivery/terminal settlement 分层 | 玩家知道生产完成与目的地后果不是同一结果 | produced/undelivered、pending、delivery 未确认 |
| 保持选择 | 首局后走哪一类持续分支 | 当前世界状态和专业 profile | 后续两个 beat 有实质差异，并给出回访 hook | 没有安全路径时停止并返回决策面 |

## 4. 循环与成长

首局循环采用“目标 → 接受/拒绝 → 推进/阻塞 → 权威后果 → 下一决策或恢复”。原材料准备是来源评估、获取/精炼结算、`ready_for_logistics`、运输、到达重验和齐套的子循环；它不能被一个“获取原料”动作掩盖。

首次持续能力必须表现为可继续运转、可恢复并能打开新选择的能力。扩张增加覆盖或产出并引入吞吐压力；稳定/恢复保住能力并放弃一部分即时扩张；专业化/服务把能力转成对本地需求或协作的用途。每个方向都要说明即时收益、后续两个 beat、风险或锁定和下次会话第一动作；适用时还要说明回退窗口、代价、保留和失去的价值。

## 5. 行为与后果

玩家意图只表达目标和授权范围，不能直接保证 Agent 行动或世界事件成功。入口展示的 accepted、执行、阻塞、生产和交付状态必须来自同一权威事实；推荐、缓存、叙事或计数不能替代 receipt。

报价或展示后发生事实漂移时，系统只能重新评估、无副作用拒绝或进入专业合同明确的有界 pending。重连、重复、乱序、Agent retry、snapshot restore 和 replay 只能重读原 disposition，不得复制材料、资格、receipt、进度、奖励或目的地后果。Viewer 与 pure API 的布局可以不同，但动作、主 blocker、完成边界、下一步和复查点必须同义。

## 6. 资源与机会成本

该设计只约定资源来源、用途、持有、流转和损失应可归因：原料可能被保留或消耗，电力和容量可能成为阻塞，物流可能带来占用和损耗，产物可能停留在生产后但未交付状态。正式配方、产率、批量、价格、物流、容量、tick 和结算公式由专业 authority 决定；产品层不填默认值，也不把 unknown 当作零成本、免费输入或无限容量。

## 7. 失败、阻塞与恢复

恢复反馈要指出最早且可行动的根因、已保留/已消费/已损失价值、下一复查边界和真实可用的等待、补充、减量、改源、改配方、改道、延期或停止路径。来源耗尽且无补充、规格未知、低电、输入未齐套、路径或容量失效、输出去向缺失、终端资格变化时，不得无限等待、免费补料、静默改道或把 target/unknown 写成完成。没有安全路径时，返回重新定目标或安全停止。

## 8. 入口与可访问性边界

Viewer 与 pure API 共享权威事实和玩家语义；每个入口分别证明 cold start、进行中、重连/续玩以及空/阻塞状态仍有有效动作或真实恢复动作。设计不冻结控件位置、字段 schema、提示文案或窄屏布局。入口需让状态接受、执行、失败、等待和恢复可辨识，并避免用原始枚举、日志或隐藏字段要求玩家猜测。

## 9. 取舍、验证与证据边界

选择“先通过可行性 Gate、再逐节点推进、最后独立结算交付”，牺牲了一步完成的表面顺滑，但保留资源守恒、失败恢复和玩家归因；选择保留生产与交付的双 receipt，牺牲了自动化简化，但避免把中间结果伪装成目的地成果。

设计覆盖的产品要求和场景见：[`工业因果链要求`](first-session-and-continuation.prd.md#req-first-industrial-002)、[`原料与物流边界`](first-session-and-continuation.prd.md#req-first-industrial-003)、[`生产与交付边界`](first-session-and-continuation.prd.md#req-first-industrial-004)、[`漂移与重复`](first-session-and-continuation.prd.md#req-first-industrial-005)、[`阻塞恢复`](first-session-and-continuation.prd.md#req-first-industrial-006)、[`持续承接`](first-session-and-continuation.prd.md#req-first-industrial-008)、[`正向链路验收`](first-session-and-continuation.prd.md#ac-first-industrial-001)、[`安全恢复验收`](first-session-and-continuation.prd.md#ac-first-industrial-002) 和 [`生产未交付验收`](first-session-and-continuation.prd.md#ac-first-industrial-005)。跨入口与回流的证据边界见 [`回流与入口验收`](first-session-and-continuation.prd.md#ac-first-industrial-004)。

`test_tier_required` 应覆盖正向链、各主要 blocker、arrival order、多输入齐套、生产未交付和重复/重连无副作用；`test_tier_full` 再覆盖跨窗口、争用、损耗、终端故障、持久化恢复及多入口 parity。产品文档通过、自动化检查或合成/Agent 样例只能证明结构和合同可判定性，不能证明当前 runtime、Viewer、Agent、玩家留存、可玩性或发行就绪。

## 10. 相邻权威与未决边界

详细玩法顺序、profile 完成边界、receipt 与稳定窗口由 [`工业 walkthrough 合同`](../../game/gameplay/gameplay-industrial-representative-execution-walkthrough.prd.md#3-五阶段玩家-walkthrough) 和 [`首产物结算合同`](../../game/gameplay/gameplay-industrial-starter-completion-contract.prd.md#3-五阶段结算表) 拥有；批次、物流、容量、终端和守恒由 [`M4 合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md#4-technical-specifications) 拥有；实现和入口证据由 [`world-runtime 专业 PRD`](../../world-runtime/prd.md#consumer-compatibility-industrial-profile-and-stage-execution)、Viewer、Agent、[`testing 专业 PRD`](../../testing/prd.md#6-validation-decision-record) 等专业 authority 拥有。任何专业规则变化都应重新核对本设计的产品边界和证据窗口。
