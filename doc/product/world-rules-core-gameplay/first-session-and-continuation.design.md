# 首局与持续游玩产品设计

## 文档身份

- 配对产品 PRD：[`first-session-and-continuation.prd.md`](first-session-and-continuation.prd.md)
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：2026-10-12
- 专业域权威：[`gameplay 工业 walkthrough 合同`](../../game/gameplay/gameplay-industrial-representative-execution-walkthrough.prd.md#3-五阶段玩家-walkthrough)、[`gameplay 首产物结算合同`](../../game/gameplay/gameplay-industrial-starter-completion-contract.prd.md#3-五阶段结算表)、[`M4 工业资源流转合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md#4-technical-specifications)、[`world-runtime 专业 PRD`](../../world-runtime/prd.md#consumer-compatibility-industrial-profile-and-stage-execution)、[`testing 专业 PRD`](../../testing/prd.md#6-validation-decision-record)

本文说明玩家怎样理解并继续一条首局工业因果链。配对 PRD 拥有产品要求和验收；专业 authority 拥有玩法顺序、结算、状态、receipt、接口和测试实现。本设计不把目标态写成当前实现或发行承诺。

## 1. 设计命题

当前首轮路线复用 `starter-industrial-smelter-to-assembler-v1` / `production_only`：合法 Smelter 按所选配方与合法输入/电力完成真实 cycle，第一笔匹配的 owner-bound 正量 `iron_ingot` 结算与持久首产物里程碑一致，仅打开 Assembler 候选。首产物不要求建成 Assembler；持续能力和交付保留独立条件。再次进入先读取同一世界与归属下的成果、有效目标、未完成事项和原请求身份；pending 按恢复时授权、资源与 manifest 重新裁决。过期/撤销授权只提供重新授权、缩小范围或安全停止等真实选项，不自动继承旧条件。玩法阻塞恢复、节点灾备与回访接续各自取证，技术故障注入不要求真人强制经历事故。

目标体验是让第一次进入世界的玩家从一个可解释的工业目标走到当前 `starter_completion_profile` 声明的真实完成边界，并在阻塞、重连或完成后仍保有下一次选择。`production_only` 在匹配 production receipt 后完成首产物并进入 `produced/undelivered`；`terminal-admission` 才继续到匹配 delivery/terminal settlement。玩家需要知道目标、当前权威状态、已保留或消耗的价值、主要 blocker、完成边界和下一步。

核心决策是：在 feasibility card 与当前事实支持的范围内，选择等待、补足、改源、改配方、改道、延期或继续推进哪一条路径。选择必须有可读的机会成本；预览和推荐只提供比较信息，不产生世界效果。

设计假设是先建立一条可归因的因果链，再展开持续能力分支，能帮助玩家判断是否继续。该假设需要对应入口的体验证据验证，结构检查或局部自动化不能替代玩家体验结论。

## 2. 代表性片段与阶段信息

<a id="des-first-profile-001"></a>
### DES-FIRST-PROFILE-001：按选定 profile 展开节点

承接 [REQ-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#req-first-industrial-001)、[002](first-session-and-continuation.prd.md#req-first-industrial-002)、[003](first-session-and-continuation.prd.md#req-first-industrial-003)、[004](first-session-and-continuation.prd.md#req-first-industrial-004)、[005](first-session-and-continuation.prd.md#req-first-industrial-005)、[006](first-session-and-continuation.prd.md#req-first-industrial-006) 与 [PRD profile 适用性](first-session-and-continuation.prd.md#starter-profile-applicability)。当前片段为合法 Smelter、配方与输入/电力、排程、真实生产结算、持久里程碑与 Assembler 候选；来源/精炼、handoff、物流和多输入节点仅在所选配方/profile 明确依赖时展开。后续 terminal-admission 专题独立走到 delivery receipt；当前 production_only 保持 `produced/undelivered`，且不生成交付用途或终端奖励。

下表按这套适用范围读取。不适用必须由所选配方/profile 和现行专业 authority 支持；实际必需 authority 缺失仍保持 unknown/blocked，并返回补证、修复、等待或重新定目标的路径，不能以“不适用”绕过。产品与 gameplay owner 拥有范围和完成边界，runtime/M4 提供真实依赖与结算，Viewer/Agent 消费，QA 按[VAL-FIRST-PROFILE-001](#val-first-profile-001)分别取证。

| 阶段 | 玩家知道什么 | 可以选择什么 | 成本与承诺 | 反馈与下一步 |
| --- | --- | --- | --- | --- |
| Gate 与目标 | candidate 是否由同一份 fresh authority snapshot 支持，以及完成边界、主要风险和复查点 | 接受候选、查看阻塞、改候选或回到其他目标 | Gate 不扣资源、不锁容量、不排程；unknown 不被填成可达 | `candidate_available` 或 `no_safe_starter_chain`；后者给出最早 blocker、保留价值与复查点 |
| 工厂与配方 | 工厂能力、配方适用性、合法输入与电力，以及适用的原料来源和运输风险 | 比较可用配方、处理前置或延期 | 比较不占库存；提交后仍须按当前 authority 重验 | 接受、拒绝或重新报价；下一步是准备真实输入 |
| 来源与物流（依赖适用时） | source/refinement receipt 是否已结算，以及是否已完成 source/owner ledger handoff 并形成带 lineage 的 `ready_for_logistics` 批次；在途和到达是否已结算 | 获取、精炼、补充、减量、等待容量或改道 | 消耗、占用、损耗和未满足量保持可见；`source-settled` 不等于 `ready_for_logistics` 或到达 | `ready_for_logistics`、in-transit、arrival、applicable 各自独立；不依赖物流的合法输入不伪造这些阶段 |
| 输入前置与排程 | 实际输入/电力是否满足；需要多输入 join 时，各 parent 是否已 settlement、到达且适用；排程是否只是接受 | 等待缺失输入、补齐、重排、换候选或结束意图 | 适用 join 的先到输入可保持有界等待；排程接受不等于生产开始 | 主 blocker、已保留价值和下一复查点可读 |
| 生产与交付 | `accepted/scheduled`、执行、`produced/undelivered`、delivery/terminal settled 的差别，以及当前 profile 的完成边界 | 等待、处理输出去向、交付或延期 | `production_only` 在 matching production receipt 后完成首产物；`terminal-admission` 才等待匹配 delivery/terminal settlement；production receipt 不能自动产生交付用途或终端奖励 | 独立 receipt 可回指同一因果链；只有适用 profile 的交付完成才打开目的地后果 |
| 阶段承接 | 已形成的能力、仍在的约束和当前主目标 | 选择扩张、稳定/恢复、专业化/服务或主动换向 | 新方向承担新的资源、容量、风险和机会成本 | 下一 session 有明确第一动作；旧目标的义务和结果仍可追踪 |

状态优先级先保护安全、权利和授权，再保护不可逆损失、资源扣减和锁定，再处理可恢复前置与可选信息。状态不一致时只能给出复核、恢复、安全停止或重新定目标；状态变化后旧动作必须重新判断。

## 3. 机制与体验关系

| 体验目标 | 玩家决策 | 权威约束 | 预期互动结果 | 失败信号 |
| --- | --- | --- | --- | --- |
| 建立信任 | 是否采用当前 starter candidate | Gate、profile、factory、recipe、input、power，以及明确依赖的 path/terminal authority | 一条有稳定 identity 和完成边界的可解释链路 | `no_safe_starter_chain`、unknown 或最早 blocker |
| 理解投入 | 是否获取、精炼、等待或改源 | source/owner ledger、规格、容量、电力和物流合同 | 资源去向、保留、消耗和未满足量可回看 | 证据不足、争用、低电、容量或适用性失败 |
| 归因结果 | 是否继续、恢复、交付或换向 | production receipt 与 delivery/terminal settlement 分层 | 玩家知道生产完成与目的地后果不是同一结果 | produced/undelivered、pending、delivery 未确认 |
| 保持选择 | 首局后走哪一类持续分支 | 当前世界状态和专业 profile | 后续两个 beat 有实质差异，并给出回访 hook | 没有安全路径时停止并返回决策面 |

## 4. 循环与成长

首局循环采用“目标 → 接受/拒绝 → 推进/阻塞 → 权威后果 → 下一决策或恢复”。所选配方/profile 明确要求从来源获取/精炼并经 handoff、物流供应输入时，才展开来源评估、获取/精炼结算、source/owner ledger handoff、`ready_for_logistics`、运输、到达重验以及适用齐套的子循环。在这条子循环中，`source-settled` 不等于 `ready_for_logistics`，不能被一个“获取原料”动作掩盖，也不能把 source receipt 直接当作下游输入；它不为无需这些阶段的 canonical Smelter 增加新前置。

首次持续能力必须表现为可继续运转、可恢复并能打开新选择的能力。扩张增加覆盖或产出并引入吞吐压力；稳定/恢复保住能力并放弃一部分即时扩张；专业化/服务把能力转成对本地需求或协作的用途。每个方向都要说明即时收益、后续两个 beat、风险或锁定和下次会话第一动作；适用时还要说明回退窗口、代价、保留和失去的价值。

当主目标是本地或区域专业化时，分支详情继续解释生产能力服务什么需求。模块、工厂核心与合金可以是合法下一步；它们的收益和风险不能突然宣称玩家已进入治理或冲突阶段。当前不可执行的候选不作为推荐，信息不完整时返回可读的不可用原因。

回退是有期限的选择。窗口关闭时，玩家应立即看到原因、已保留的结果和义务，以及仍可执行的修复、重排或换向；后续恢复成功不会重新打开旧窗口。各条路线按自身权威条件判断，展示样例的阶段只解释样例，不代替真实世界的回退能力。首产物完成继续按既定 profile 判断，不能为延长回退窗口暗中加入交付条件。

## 5. 行为与后果

玩家意图只表达目标和授权范围，不能直接保证 Agent 行动或世界事件成功。入口展示的 accepted、执行、阻塞、生产和交付状态必须来自同一权威事实；推荐、缓存、叙事或计数不能替代 receipt。

目标换向按 [PRD 的三阶段边界](first-session-and-continuation.prd.md#goal-redirection-pending-and-settled-boundary)表达：预览只用于比较；已接受的承诺 / 待决请求按专业合同保留其有限 hold、排队或未决义务，尚无 committed receipt 时仍为待决且未生效；只有权威 receipt 支持且实际生效的后果才显示为已结算结果。已结算范围不能因换向被抹除或再次执行，未结算范围也不能被同一目标的部分 receipt 提前标成完成。新主目标可以取得前景位置，旧请求的待决状态和合法取消、过期、重评或恢复路径仍须可读；receipt 的具体含义与结果范围由原专业合同决定。

报价或展示后发生事实漂移时，系统只能重新评估、无副作用拒绝或进入专业合同明确的有界 pending。重连、重复、乱序、Agent retry、snapshot restore 和 replay 只能重读原 disposition，不得复制材料、资格、receipt、进度、奖励或目的地后果。Viewer 与 pure API 的布局可以不同，但动作、主 blocker、完成边界、下一步和复查点必须同义。

## 6. 资源与机会成本

该设计只约定资源来源、用途、持有、流转和损失应可归因：原料可能被保留或消耗，电力和容量可能成为阻塞，物流可能带来占用和损耗，产物可能停留在生产后但未交付状态。正式配方、产率、批量、价格、物流、容量、tick 和结算公式由专业 authority 决定；产品层不填默认值，也不把 unknown 当作零成本、免费输入或无限容量。

## 7. 失败、阻塞与恢复

恢复反馈要指出最早且可行动的根因、已保留/已消费/已损失价值、下一复查边界和真实可用的等待、补充、减量、改源、改配方、改道、延期或停止路径。来源耗尽且无补充、规格未知、低电、输入未齐套、路径或容量失效、输出去向缺失、终端资格变化时，不得无限等待、免费补料、静默改道或把 target/unknown 写成完成。没有安全路径时，返回重新定目标或安全停止。

## 8. 入口与可访问性边界

Viewer 与 pure API 共享权威事实和玩家语义；每个入口分别证明 cold start、进行中、重连/续玩以及空/阻塞状态仍有有效动作或真实恢复动作。设计不冻结控件位置、字段 schema、提示文案或窄屏布局。入口需让状态接受、执行、失败、等待和恢复可辨识，并避免用原始枚举、日志或隐藏字段要求玩家猜测。

## 9. 取舍、验证与证据边界

选择“先通过可行性 Gate、再逐节点推进、最后按 profile 独立结算 production 或 delivery”，保留资源守恒、失败恢复和玩家归因；`production_only` 的交付后置与 `terminal-admission` 的交付前置都必须在结果和下一动力中可读，且生产与交付仍由独立 receipt 表达。

设计覆盖的产品要求和场景见：[`工业因果链要求`](first-session-and-continuation.prd.md#req-first-industrial-002)、[`原料与物流边界`](first-session-and-continuation.prd.md#req-first-industrial-003)、[`生产与交付边界`](first-session-and-continuation.prd.md#req-first-industrial-004)、[`漂移与重复`](first-session-and-continuation.prd.md#req-first-industrial-005)、[`阻塞恢复`](first-session-and-continuation.prd.md#req-first-industrial-006)、[`持续承接`](first-session-and-continuation.prd.md#req-first-industrial-008)、[`正向链路验收`](first-session-and-continuation.prd.md#ac-first-industrial-001)、[`安全恢复验收`](first-session-and-continuation.prd.md#ac-first-industrial-002) 和 [`生产未交付验收`](first-session-and-continuation.prd.md#ac-first-industrial-005)。跨入口与回流的证据边界见 [`回流与入口验收`](first-session-and-continuation.prd.md#ac-first-industrial-004)。

<a id="val-first-profile-001"></a>
### VAL-FIRST-PROFILE-001：当前首产物与后续专题分别验收

本验证承接 [DES-FIRST-PROFILE-001](#des-first-profile-001)、[AC-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#ac-first-industrial-001)、[002](first-session-and-continuation.prd.md#ac-first-industrial-002)、[003](first-session-and-continuation.prd.md#ac-first-industrial-003)、[005](first-session-and-continuation.prd.md#ac-first-industrial-005) 与 FS-15；专业 owner 和原始候选记录沿用 PRD 验收追踪，不创建另一套通过状态。

| 验证范围 | `test_tier_required` 判定 | 适用边界 |
| --- | --- | --- |
| 当前 canonical `production_only` | 合法 Smelter/recipe、实际输入与电力、matching production receipt、owner-bound 正量铁锭、持久里程碑与 Assembler 候选一致；主 blocker 与恢复可读，重复/重连/replay 不复制 receipt、产出或里程碑 | 不要求建成 Assembler、稳定窗口、物流/multi-input 或 terminal-admission 的独立能力；当前选定配方的真实依赖仍必须验证 |
| 当前生产/交付负边界 | `produced/undelivered` 可完成首产物，却不产生 delivery-only 用途、需求减少或终端奖励；production receipt 不能代替 delivery receipt | 保留当前 required 负例，无需先实现完整终端正向路径 |
| 来源/handoff/物流/多输入 | 依赖适用时验证 `source-settled`、`ready_for_logistics`、arrival/applicable 与 join 分层、各主 blocker、适用 arrival-order 和无重复效果；缺必需 authority 为 unknown/blocked | 仅按所选配方/profile 明确依赖加入；无依赖不伪造通过，未知或不支持的必需依赖不得标为不适用 |
| 后续 `terminal-admission` | 独立证明 delivery/terminal settlement 正向完成，以及生产成功但交付失败/未确认仍为 `produced/undelivered` | 专题启动且选定该 profile 后执行，不作为当前首产物完成前置 |

`test_tier_full` 按选定范围覆盖跨窗口、争用、适用损耗/终端故障、持久化恢复及多入口 parity。记录固定候选、profile/revision、实际依赖、证据层级和未覆盖项；文档、自动化或合成/Agent 样例只证明其实际范围，不能宣称当前 runtime、Viewer、Agent、真人留存、可玩性或发行就绪。

目标换向的 [FS-13](first-session-and-continuation.prd.md#fs-13) 验收保留三类对照：已接受但无 committed receipt 的请求不能显示为已结算；权威 receipt 支持的已结算结果不因换向、重连或重试消失或重复；部分已结算与仍待决范围分别可读。该语义核对不代表本次已执行真实 runtime/Agent 换向或跨入口游玩验收。

分支比较承接 [REQ-FIRST-BRANCH-001](first-session-and-continuation.prd.md#req-first-branch-001) / [AC-FIRST-BRANCH-001](first-session-and-continuation.prd.md#ac-first-branch-001)：比较主指引和每个推荐的完整承诺，并保留有效的工业用途。回退承接 [REQ-FIRST-BRANCH-002](first-session-and-continuation.prd.md#req-first-branch-002) / [AC-FIRST-BRANCH-002](first-session-and-continuation.prd.md#ac-first-branch-002)：分别观察截止前、截止时和恢复后的选择与后果。具体投影、样例边界及回归方法由 [Viewer 专业设计](../../world-simulator/viewer/viewer-gameplay-release-experience-overhaul.design.md#branch-preview-consistency) 承接；结构正反例不能代签玩家实玩。

## 10. 相邻权威与未决边界

详细玩法顺序、profile 完成边界、receipt 与稳定窗口由 [`工业 walkthrough 合同`](../../game/gameplay/gameplay-industrial-representative-execution-walkthrough.prd.md#3-五阶段玩家-walkthrough) 和 [`首产物结算合同`](../../game/gameplay/gameplay-industrial-starter-completion-contract.prd.md#3-五阶段结算表) 拥有；批次、物流、容量、终端和守恒由 [`M4 合同`](../../world-simulator/m4/industrial-resource-flow-contract.prd.md#4-technical-specifications) 拥有；实现和入口证据由 [`world-runtime 专业 PRD`](../../world-runtime/prd.md#consumer-compatibility-industrial-profile-and-stage-execution)、Viewer、Agent、[`testing 专业 PRD`](../../testing/prd.md#6-validation-decision-record) 等专业 authority 拥有。任何专业规则变化都应重新核对本设计的产品边界和证据窗口。
## PRD REQ/AC fragment mapping

This design delegates product requirements and acceptance semantics to its paired PRD. Every row below is a navigable fragment mapping; this design does not create a second requirement authority.

| PRD requirement | PRD acceptance |
| --- | --- |
| [REQ-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#req-first-industrial-001) | [AC-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#ac-first-industrial-001) / [AC-FIRST-INDUSTRIAL-002](first-session-and-continuation.prd.md#ac-first-industrial-002) |
| [REQ-FIRST-INDUSTRIAL-002](first-session-and-continuation.prd.md#req-first-industrial-002) | [AC-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#ac-first-industrial-001) |
| [REQ-FIRST-INDUSTRIAL-003](first-session-and-continuation.prd.md#req-first-industrial-003) | [AC-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#ac-first-industrial-001) / [AC-FIRST-INDUSTRIAL-002](first-session-and-continuation.prd.md#ac-first-industrial-002) |
| [REQ-FIRST-INDUSTRIAL-004](first-session-and-continuation.prd.md#req-first-industrial-004) | [AC-FIRST-INDUSTRIAL-001](first-session-and-continuation.prd.md#ac-first-industrial-001) / [AC-FIRST-INDUSTRIAL-005](first-session-and-continuation.prd.md#ac-first-industrial-005) |
| [REQ-FIRST-INDUSTRIAL-005](first-session-and-continuation.prd.md#req-first-industrial-005) | [AC-FIRST-INDUSTRIAL-003](first-session-and-continuation.prd.md#ac-first-industrial-003) |
| [REQ-FIRST-INDUSTRIAL-006](first-session-and-continuation.prd.md#req-first-industrial-006) | [AC-FIRST-INDUSTRIAL-002](first-session-and-continuation.prd.md#ac-first-industrial-002) |
| [REQ-FIRST-INDUSTRIAL-007](first-session-and-continuation.prd.md#req-first-industrial-007) | [AC-FIRST-INDUSTRIAL-004](first-session-and-continuation.prd.md#ac-first-industrial-004) |
| [REQ-FIRST-INDUSTRIAL-008](first-session-and-continuation.prd.md#req-first-industrial-008) | [AC-FIRST-INDUSTRIAL-004](first-session-and-continuation.prd.md#ac-first-industrial-004) |
| [REQ-FIRST-BRANCH-001](first-session-and-continuation.prd.md#req-first-branch-001) | [AC-FIRST-BRANCH-001](first-session-and-continuation.prd.md#ac-first-branch-001) |
| [REQ-FIRST-BRANCH-002](first-session-and-continuation.prd.md#req-first-branch-002) | [AC-FIRST-BRANCH-002](first-session-and-continuation.prd.md#ac-first-branch-002) |
