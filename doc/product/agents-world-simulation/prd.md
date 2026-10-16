# 智能体、世界模拟与交互 PRD

## 文档身份

- 产品模块：智能体、世界模拟与交互
- 产品模块 slug：`agents-world-simulation`
- 产品层唯一 PRD：`doc/product/agents-world-simulation/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-003`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-10-15`
- 后继文档：`无`
- 下层专业域：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)

本文只承载场景、Agent/provider 决策、权威世界状态与可交互表达的产品组合。Viewer、Launcher、provider 与模拟合同由 `world-simulator` 专业域拥有。

### 活跃产品专题

- [外部 Agent Runtime 接入与持续游玩](external-agent-runtime-play.prd.md)：明确用户带 OpenClaw、Codex 等 Runtime 接入的产品目标、职责边界、首期完整游戏流程与分别验收；[配对产品设计](external-agent-runtime-play.design.md)承接接入、委托、运行和恢复体验。
- [玩家与外部 Runtime 的持续协作](player-runtime-collaboration.prd.md)：细化 Web/桌面玩家与 OpenClaw/Codex 的任务绑定、目标版本确认、进度汇报、双向问答、授权申请及离线恢复；[配对产品设计](player-runtime-collaboration.design.md)承接协作交互。
- [Agent/provider 体验连续性](provider-agent-experience-continuity.prd.md)：只收敛玩家体验承诺；provider 组合的场景范围、评估方法与结论仍由专业域权威文档维护。
- [玩家可读的世界舞台](player-readable-world-stage.prd.md)：约束正式世界表面的首读层级、空间关系来源、可归因玩家因果与按需诊断边界。
- [玩家可读表面的连续性](player-readable-surface-continuity.prd.md)：约束 viewport、信息密度、语言与连接状态变化时，主要决策面仍可读、可操作且可恢复。
- [Agent 对话与 Prompt 控制](agent-conversation-and-prompt-control.prd.md)：区分一次对话、预设/草稿与持续 Prompt/目标调整，并配对承载跨 surface 产品交互设计。
- [Agent 自治、委托与责任连续性](agent-authority-ownership-and-accountability.prd.md)：承载自治模式、预授权、转让后的有效委托与身份连续性、异议/override 与因果责任边界；团队资产扩张与经济义务由玩法模块主责。
- [Provider、学习认证与情报连续性](provider-learning-intelligence-and-cadence.prd.md)：可选择的认证 provider、固定权威节奏、可审计训练/重训与有期限情报的长期产品边界。

## 1. 产品承诺

玩家进入的是场景、Agent 意图、世界事件和交互反馈相互对应的持久模拟，而不是一次性聊天或本地假世界。

Agent 的意图在进入世界后必须经过统一权威裁决；并发冲突、重复提交或跨入口重试不得产生重复世界效果，并必须留下玩家可理解、可回放的接受、拒绝、改道或替代原因。

## 2. 范围与玩家边界

覆盖可重现场景、Agent/LLM provider 权威、世界事件流、进入世界后的主 Web 界面、交互连续性和反馈。玩家可以观察、发起被允许的意图并看到权威结果；不能把 mock、本地演示或未授权 provider 输出当作世界状态。下载、启动、模式选择、真实后端核验、升级与重新进入由 [`玩家接入与发行`](../player-entry-distribution/prd.md) 主责，本模块只消费这些入口提供的世界上下文并保证进入后的语义连续。

Agent 可以基于世界观测、工业物流、市场、风险与治理约束形成计划，但这些输入可能延迟、过期或不完整；当其影响当前决策时，产品必须区分当前事实与陈旧/不确定情报，并允许刷新、纠正或重排，不能把缓存信息伪装成实时真值。

模型、provider 或配置只影响 Agent 如何提出决策，不改变世界规则与执行权威；缺失、无效或不可用的推理配置必须形成可诊断阻塞或失败，保持世界状态不被伪造成功或被静默替代策略改写。

### 2.1 玩家与可控制 Agent 的承诺边界

玩家对 Agent 的直接策略控制来自当前账号已经绑定或按权威规则认领的 Agent；共享世界中可见的其他 Agent、未绑定 Agent 或默认选中对象不因此成为“我的 Agent”。首个可控制 Agent 也不是免费席位：确认前，玩家必须能理解一次性承诺、持续维护、可维持时间和主要失去控制风险，并在认领、比较候选、等待或先补足资源之间作出可读选择。面向首个认领的受限启动帮助只能降低进入摩擦，不得被表达为免费控制、通用补贴或可转让资产。若持续承诺无法维持或所有权不再成立，体验必须如实说明状态、恢复/释放或重新规划的下一步，而不是静默保留、静默清退或把别人的 Agent 当作玩家可操作对象。详细的认领报价、维护、回收与反滥用专业合同见 [`PRD-GAME-011`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md)，产品层不复制其字段、数值或状态机。

### 2.2 跨域闭环

`世界观测与工业/市场/物流/风险约束 -> Agent 形成意图与理由 -> runtime 统一校验、冲突裁决和提交 -> 世界事件/资源/治理状态更新 -> Viewer 或 pure API 回写 accepted intent、执行状态、主因果、成本/进展、阻塞与下一步 -> 玩家纠正、打断或重排`

Agent 不得绕过世界规则与基础设施约束；当工业、市场或治理数据实质影响当前 Agent 意图或玩家可见结果时，必须进入可追踪的因果反馈。

<a id="external-runtime-play"></a>
### 2.3 外部 Agent Runtime 的产品目标

玩家可以把已有的 OpenClaw、Codex 等 Agent Runtime 接入 oasis7，在高层目标、有效委托和已说明的预算范围内，驱动合法绑定的游戏 Agent 持续、自主地推进游戏任务。接入需要形成观察、合法能力、行动、权威反馈和下一轮决策的完整经历，允许玩家调整持续目标并在断连/重启后核对原结果继续。

本节与 SC-14 的首期范围、用户流程、**官方 Skill 引导 + 可直接调用的标准 Game API** 及叶子验收由[外部 Runtime 产品分册](external-agent-runtime-play.prd.md#external-runtime-canonical-interface)拥有。外部 Runtime 主动读取观察、发现能力、提交并追读权威结果；原 Provider callback/Bridge 仅是可替换的既有实现，不能代签完整产品能力。OpenClaw 与 Codex 是分别取证的首批目标；产品目标不表示当前已经支持或默认启用。`viewer` / `pure_api` 仍是玩家访问模式，`headless_agent` / `player_parity` 是执行 lane；关闭观战界面后的持续执行以 Runtime、连接、预算和独立委托仍有效为条件。认领/维护、工业成果、自治与授权、公开状态继续消费各自主责条款。

玩家与外部 Runtime 通过持久化的游戏任务、目标、消息和有效委托协作，而非由网页直接控制某个聊天进程。目标保存、执行方读取、采纳声明、实际用于新决策和世界结果分别表达；普通聊天不自动改目标或授权。具体协作功能与验收见[持续协作分册](player-runtime-collaboration.prd.md#player-runtime-collaboration)，其[局部功能清单](player-runtime-collaboration.prd.md#collaboration-feature-catalog)细化外部 Runtime 主清单已有承诺，不重编主功能编号。

## 3. 权威与冲突处理

| 产品层拥有 | 专业域权威 |
| --- | --- |
| 场景、Agent/provider、世界状态到交互模拟的端到端承诺 | `doc/world-simulator/prd.md` 拥有 Viewer、Launcher、Agent/provider、场景和验证合同；`doc/game/prd.md` 拥有间接控制与玩家能动性；`doc/world-runtime/prd.md` 拥有权威执行和冲突语义 |

产品 PRD 不代替具体模式、API、UI 的专业合同、默认 provider 配置或实际支持判定；冲突时由对应 `agent_engineer`、`viewer_engineer` 或 runtime owner 与产品 owner 共同裁决。

相邻产品组合依赖：工业、市场、物流和治理的产品语义由 [`世界规则与玩法系统`](../world-rules-core-gameplay/prd.md) 拥有；[`权威世界基础设施`](../world-infrastructure/prd.md) 提供最终性、确定性执行与状态连续性保证；本模块消费两者形成可解释的 Agent 决策与反馈闭环，但不能以相邻产品 PRD 代替 `game` / `runtime` / `world-simulator` 的专业规则与验证证据。

Agent 团队资产扩张、容量与转让后的经济义务由玩法模块的[Agent 所有权与持续经营](../world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md)主责；本模块只消费其结果，并独立定义有效委托与责任连续性。

## 4. 路线图

1. 可重现场景：初始状态、资源变化、关键事件与验收证据一致。
2. 权威 Agent 闭环：provider 和 Agent 意图不绕过世界校验。
3. 多入口等价：主 Web、Launcher 与 pure API 在权威状态与核心动作上一致。
4. 外部 Runtime 游玩：从契约验证推进到 OpenClaw、Codex 各自完成代表性首局，再按实际证据扩展持续玩法与 Runtime 范围；阶段范围见[产品分册](external-agent-runtime-play.prd.md#22-阶段目标)。

<a name="5-done：成功标准与验收"></a>

## 5. Done：成功标准与验收

SC-1～SC-14 的成功标准原文及独立组合状态见 §5.2；§5.1 保留对应六列验收责任与原测试层级。


### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | runtime_engineer | PRD-WORLD_SIMULATOR-001/002/003 | `doc/world-simulator/prd.md` | 场景初始化、事件与回放证据 | test_tier_required |
| SC-2 | agent_engineer | PRD-WORLD_SIMULATOR-016-019 / PRD-WORLD_SIMULATOR-036 | `doc/world-simulator/prd.md`；`doc/world-simulator/llm/decision-provider-contract.prd.md`；[builtin 输入完整性](../../world-simulator/llm/decision-provider-contract.prd.md#builtin-input-integrity)与[参数说明一致性](../../world-simulator/llm/decision-provider-contract.prd.md#builtin-parameter-consistency) | live provider 和权威行为闭环；builtin 的[输入完整性验收](../../world-simulator/llm/decision-provider-contract.prd.md#ac-builtin-input-integrity)与[参数一致性验收](../../world-simulator/llm/decision-provider-contract.prd.md#ac-builtin-parameter-consistency)分别提供专业证据，不代签完整玩家体验或其它 provider 支持 | test_tier_required |
| SC-3 | viewer_engineer | PRD-WORLD_SIMULATOR-039/041/046 | `doc/world-simulator/prd.md` | 主 Web Viewer 真实世界交互证据 | test_tier_required |
| SC-4 | qa_engineer | PRD-WORLD_SIMULATOR-020-031 | `doc/world-simulator/prd.md` | 模式、转移与多入口 claim 回归 | test_tier_required |
| SC-5 | runtime_engineer / agent_engineer / gameplay_designer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-001 / PRD-WORLD_RUNTIME-001 / PRD-GAME-008 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/game/prd.md`; `doc/testing/prd.md` | 等价重排序、冲突、重复投递、重试与 replay 的 accepted/rejected 集合、单次效果，以及 Viewer/pure API 解释一致性证据，包含 S6 玩家 surface 核对 | test_tier_full |
| SC-6 | agent_engineer / viewer_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-036 | `doc/world-simulator/prd.md`；`doc/world-simulator/llm/decision-provider-contract.prd.md`；[builtin 当前输入与不足处理](../../world-simulator/llm/decision-provider-contract.prd.md#builtin-input-integrity) | 观测新鲜度、不确定性与刷新/纠正/重排路径证据，包含正式玩家 surface 的 S6 交互闭环；builtin 必需输入与配置不足另按[专业验收](../../world-simulator/llm/decision-provider-contract.prd.md#ac-builtin-input-integrity)取证，局部 fixture 不代签 S6 | test_tier_required |
| SC-7 | agent_engineer / gameplay_designer / runtime_engineer / viewer_engineer | PRD-WORLD_SIMULATOR-001 / PRD-WORLD_SIMULATOR-016 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 | `doc/world-simulator/prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/product/world-rules-core-gameplay/prd.md`; `doc/product/world-infrastructure/prd.md` | Agent 工业约束到权威后果与玩家反馈的端到端证据；工业、市场、物流和治理的产品语义由 `doc/product/world-rules-core-gameplay/prd.md` 提供，`doc/product/world-infrastructure/prd.md` 仅提供最终性、确定性执行与状态连续性保证 | test_tier_required |
| SC-8 | producer_system_designer / gameplay_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-011 / PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-039 / PRD-TESTING-003 | `doc/game/gameplay/gameplay-agent-claim-economy-contract.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 账号绑定/认领可读性、首次承诺与持续维护选择、受限启动帮助边界，以及无法维持时的恢复/释放/重规划玩家 surface 证据；不复制数值、字段或状态机 | test_tier_required |
| SC-9 | producer_system_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-039 / PRD-TESTING-003 | `doc/product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 对话、草稿、默认/override、accepted/applied/rejected/blocked 与窄屏可达性对账 | test_tier_required |
| SC-10 | producer_system_designer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/player-readable-surface-continuity.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | desktop、窄屏/低高度、双语 fallback 与断连恢复中的决策锚点、权威结果和下一步对账 | test_tier_required |
| SC-11 | producer_system_designer / agent_engineer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-011 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/agent-authority-ownership-and-accountability.prd.md`; `doc/world-runtime/runtime/indirect-control-agency-execution-and-continuation.design.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 自治/授权、规模、转让、异议/override、责任 receipt 和正式 surface 可读性的组合证据；系统边界追踪见 DES-WR-IA-010..012 | test_tier_required |
| SC-12 | producer_system_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/provider-learning-intelligence-and-cadence.prd.md`; `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | provider 准入/暂停、固定 cadence、训练/认证/重训历史、情报私有期/披露/公共 baseline 与 freshness 的组合证据 | test_tier_full |
| SC-13 | producer_system_designer / game_visual_interaction_designer / viewer_engineer / gameplay_designer / qa_engineer | PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/player-readable-world-stage.prd.md`; `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 世界舞台、Web/native 分别交互、区域/Fragment 下钻、定位选择、合法指导与回执、terrain 非直接动作与当前 claim 分离证据 | test_tier_required |
| SC-14 | producer_system_designer / agent_engineer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-038 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/external-agent-runtime-play.prd.md`; `doc/product/agents-world-simulation/player-runtime-collaboration.prd.md`; `doc/world-simulator/llm/provider-agent-experience-parity.prd.md`; `doc/world-simulator/prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 两真实 Runtime 分别用 Skill/Game API 完成首局、目标、反馈与恢复；Web 和桌面客户端分别完成世界交互、Agent 定位与合法指导；同一任务的目标接收/采纳、问答、授权和离线恢复分别有证据；旧 Bridge 非默认前置，支持范围绑定版本、玩法、入口与证据窗口 | test_tier_full |

### 5.2 根成功标准的组合状态与审读边界

根 SC 保持原汇总语义与 §5.1 六列验收层级；以下摘要不新增产品能力。已由专题拥有的义务复用其唯一 Trace，不为根汇总重复声明。SC-1 的固定场景重现由本根拥有，单独声明一次。叶级设计/实现状态分别见精确要求块；本节的“未核实”针对整个根组合，不覆盖或降级已确认的叶级切片。§5.1 的验证证据列是应取得的方法/证据目标，不是本轮已执行结果。

<a id="sc-agent-001"></a>
#### 功能状态卡：SC-1 组合

- SC-1：关键场景可从固定初始状态重现到相同权威事件和交互结果。

Trace ID: O7-REQ-3cf9f857-8083-4527-9648-95e9425a8448

- 功能点：SC-1，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：部分覆盖
- 系统设计依据：[场景加载约束](../../world-simulator/scenario/scenario-files.design.md#scenario-loading-design)及[生成来源与固定输入重建](../../world-simulator/scenario/scenario-files.design.md#generated-world-provenance-design)承接初始化与来源重建切片；五列承接及六列验证反链本SC。
- 初始化切片补充：[分块生成既有义务与验证](../../world-simulator/scenario/chunked-fragment-generation.design.md#chunk-generation-existing-obligations)接收同一有效 seed/config/chunk trigger order 的初始化确定性；只补反链，不改变本卡组合状态或全部权威事件及交互重现的剩余范围。该专题的[回放规范冲突](../../world-simulator/scenario/chunked-fragment-generation.design.md#chunk-replay-conflict-register)仍待裁决，不由本反链消解。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：固定场景有效seed、配置和触发顺序的确定性初始化；不能由加载成功代签整个权威事件及交互结果重现。

<a id="sc-agent-002"></a>
#### 功能状态卡：SC-2 组合

- SC-2：Agent/provider 的输入、决策、世界接受或拒绝及玩家反馈可追踪。

Trace refs: O7-REQ-aad7b341-83b1-4402-8ebc-8540db36ccfb（[REQ-EXT-003](external-agent-runtime-play.prd.md#req-ext-003)）; O7-REQ-bd792456-4b98-4891-b5e3-e24b1de4f1a1（[REQ-EXT-004](external-agent-runtime-play.prd.md#req-ext-004)）; O7-REQ-a27895ed-2ca4-45cc-901a-466befd02cf7（[REQ-EXT-005](external-agent-runtime-play.prd.md#req-ext-005)）

- 功能点：SC-2，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：外部Runtime观察、决策和权威结果切片以所列叶项为准；builtin及其他provider、正式玩家反馈、完整因果链不能由外部Runtime子集代签。

<a id="sc-agent-002-builtin-input-integrity"></a>
##### SC-2 已接收子范围：builtin 当前输入完整性

Trace ID: O7-REQ-d1ea0352-7f1a-426b-907a-cf41571a1e7f

本条只为 §5.1 SC-2/SC-6 已接收的 [builtin 当前输入完整性目标](../../world-simulator/llm/decision-provider-contract.prd.md#builtin-input-integrity)登记跨层身份：实际请求保留当前必需输入，配置不足时在模型调用前停止并给出既有无副作用失败与恢复路径。完整目标、适用条件与独立验收仍见该专业原文及其 [AC](../../world-simulator/llm/decision-provider-contract.prd.md#ac-builtin-input-integrity)，技术预算、输入投影与字段合同不转移到产品层。对应专业条款为[必要输入与预算流程](../../world-simulator/llm/decision-provider-contract.design.md#des-builtin-input-integrity)及[计划验证组](../../world-simulator/llm/decision-provider-contract.design.md#ver-builtin-input-integrity)；这些定位不改变本根 SC 的未核实状态、原证据基线或剩余组合范围，不证明实现修复、完整玩家体验或其它 provider 支持。

<a id="sc-agent-002-builtin-parameter-consistency"></a>
##### SC-2 已接收子范围：builtin 参数说明一致性

Trace ID: O7-REQ-be4bb896-dc4f-41d1-9f87-c381049ac190

本条只为 §5.1 SC-2 已接收的 [builtin 参数说明一致性目标](../../world-simulator/llm/decision-provider-contract.prd.md#builtin-parameter-consistency)登记跨层身份：同一次请求的文字、schema、parser 与 Runtime 表达同一治理合法参数合同，不因修复说明漂移而扩大合法域。完整目标及独立验收仍见该专业原文与其 [AC](../../world-simulator/llm/decision-provider-contract.prd.md#ac-builtin-parameter-consistency)，正式数值及技术消费关系继续归原专业 authority。对应专业条款为[治理约束消费关系](../../world-simulator/llm/decision-provider-contract.design.md#des-builtin-parameter-consistency)及[计划验证组](../../world-simulator/llm/decision-provider-contract.design.md#ver-builtin-parameter-consistency)；这些定位不改变本根 SC 的未核实状态、原证据基线或剩余组合范围，不证明四方实现已一致或真实模型选择。

<a id="sc-agent-002-factory-metrics"></a>
##### SC-2 已接收子范围：builtin 动作触发、成败与首达可分辨

Trace ID: O7-REQ-ff9e304a-1782-434c-a344-f5377affe3a3

报告可独立判断建厂/排产是否触发、成败次数与首达 tick。 本条只为原 SC-2 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-metrics)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-metrics)拥有；四个报告 map 保留原语义，不新增触发率分母或首次成功 tick。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-metrics)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-metrics)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-002-factory-tool-only"></a>
##### SC-2 已接收子范围：builtin 最终决策工具协议

Trace ID: O7-REQ-67e3afba-3cce-4e00-b84e-7be5960020f0

builtin 最终决策经既定工具提交，缺 function_call 不成为有效最终决策。 本条只为原 SC-2 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-tool-only)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-tool-only)拥有；LFO7 的 required、查询工具与 guardrail/修复边界保持，不扩大工具或动作集合。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-tool-only)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-tool-only)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-002-factory-typed-input"></a>
##### SC-2 已接收子范围：builtin 类型化决策输入单路径

Trace ID: O7-REQ-265317d7-916b-439e-8668-30123bbfdaab

builtin 行为层消费类型化 turn，SDK 解析失败不启用 raw-body JSON 后备入口。 本条只为原 SC-2 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-typed-input)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-typed-input)拥有；LFO9 删除字符串往返不删除 tool 参数/决策语义校验；output 仍供 trace/debug。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-typed-input)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-typed-input)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-002-factory-rewrite-receipt"></a>
##### SC-2 已接收子范围：builtin 改写原因可追踪

Trace ID: O7-REQ-876ef676-3032-4d78-989a-9d5af5971f43

builtin 的决策改写保留 from/to/reason，并在 trace 与下一轮语境解释原请求为何变化。 本条只为原 SC-2 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-rewrite-receipt)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-rewrite-receipt)拥有；LFO11 的已有 decision_rewrite 与两个 future quote diagnostics 分开，回执不证明成本预览。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-rewrite-receipt)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-rewrite-receipt)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-003"></a>
#### 功能状态卡：SC-3 组合

- SC-3：主 Web 入口呈现真实世界状态、核心动作和结构化失败。

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616（[REQ-AGENT-STAGE-001](player-readable-world-stage.prd.md#req-agent-stage-001)）; O7-REQ-41b03765-cccf-4a1f-814c-7b498dc975f6（[REQ-AGENT-SURFACE-003](player-readable-surface-continuity.prd.md#req-agent-surface-003)）; O7-REQ-e3a70494-b3f3-4220-a654-6c9f07cdb8a4（[REQ-AGENT-SURFACE-004](player-readable-surface-continuity.prd.md#req-agent-surface-004)）

- 功能点：SC-3，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：世界数据来源、个人反馈及结构化错误已有叶级关系；主Web全部核心动作、真实世界与正式入口组合尚未形成完整设计/实现证据。

<a id="sc-agent-004"></a>
#### 功能状态卡：SC-4 组合

- SC-4：Launcher、Viewer 与 pure API 不会将不同模式的证据或 claim 混为同一事实。

Trace refs: O7-REQ-f3f198ce-2afd-4487-a99e-9f371bdd8264（[REQ-ENTRY-MODE-001](../player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-mode-001)）；O7-REQ-94ed85c5-c05d-4385-a919-687bfd66b270（[REQ-ENTRY-MODE-002](../player-entry-distribution/access-modes-and-release-readiness.prd.md#req-entry-mode-002)）；O7-REQ-21f1c7b1-8be0-438e-8f92-d4d1b34be0e6（[REQ-AGENT-PARITY-001](provider-agent-experience-continuity.prd.md#req-agent-parity-001)）

- 功能点：SC-4，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：入口MODE-001/002的独立模式与失败表达及PARITY-001的体验连续性共同约束；Launcher/Viewer/pure API的所有声明与证据组合仍须按本SC分别核验。

<a id="sc-agent-005"></a>
#### 功能状态卡：SC-5 组合

- SC-5：固定场景中以不同但等价的提交顺序重放同一冲突裁决窗口的并发意图，并加入重复投递或跨入口重试时，accepted/rejected 集合、单次权威世界效果及接受、拒绝或替代原因保持确定且在 replay、Viewer 与 pure API 上语义一致。

Trace refs: O7-REQ-41b03765-cccf-4a1f-814c-7b498dc975f6（[REQ-AGENT-SURFACE-003](player-readable-surface-continuity.prd.md#req-agent-surface-003)）; O7-REQ-e3a70494-b3f3-4220-a654-6c9f07cdb8a4（[REQ-AGENT-SURFACE-004](player-readable-surface-continuity.prd.md#req-agent-surface-004)）；O7-REQ-cf672e18-ed16-42a3-9338-ab7b965ac607（[REQ-DWE-001](../world-infrastructure/deterministic-world-execution.prd.md#req-dwe-001)）；O7-REQ-4fcbad4f-f576-4bcf-b4fc-b027967b0405（[REQ-DWE-002](../world-infrastructure/deterministic-world-execution.prd.md#req-dwe-002)）；O7-REQ-78948fd8-29dc-4e9b-8a38-321ed37b7ad6（[REQ-DWE-003](../world-infrastructure/deterministic-world-execution.prd.md#req-dwe-003)）。

- 功能点：SC-5，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：基础设施DWE-001/002/003的确定性、待决/提交与lineage义务是依赖；本SC另要求等价重排序、重复投递/跨入口重试后accepted/rejected集合及replay/Viewer/pure API原因一致，不能由单一去重或反馈隔离代签。

<a id="sc-agent-006"></a>
#### 功能状态卡：SC-6 组合

- SC-6：Agent 基于过期或不完整观测做出决定时，正式体验能标记情报新鲜度或不确定性，并提供刷新、纠正或重排路径；不得让 stale observation 静默驱动长期行动。

Trace refs: O7-REQ-aad7b341-83b1-4402-8ebc-8540db36ccfb（[REQ-EXT-003](external-agent-runtime-play.prd.md#req-ext-003)）; O7-REQ-dc313559-e262-478c-8a8d-ee2f9817e5e7（[REQ-AGENT-LEARNING-002](provider-learning-intelligence-and-cadence.prd.md#req-agent-learning-002)）

- 功能点：SC-6，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：外部Runtime上下文有效性和学习情报历史仅覆盖各自叶范围；builtin输入不足、各provider的新鲜度及正式surface刷新/纠正/重排组合仍未完整承接。

Trace refs: O7-REQ-d1ea0352-7f1a-426b-907a-cf41571a1e7f；仅引用 §5.1 已接收的 [builtin 当前输入完整性子范围](#sc-agent-002-builtin-input-integrity)，不另发身份，也不以该子范围代签 SC-6 的正式 surface 与各 provider 组合。

<a id="sc-agent-007"></a>
#### 功能状态卡：SC-7 组合

- SC-7：至少一条 Agent 驱动的工业闭环可端到端验证：Agent 消费资源、物流、市场、威胁或治理约束形成意图，runtime 接受或拒绝并更新权威世界，玩家随后能读到主因果、成本/进展、阻塞和下一步。

Trace refs: O7-REQ-bd792456-4b98-4891-b5e3-e24b1de4f1a1（[REQ-EXT-004](external-agent-runtime-play.prd.md#req-ext-004)）; O7-REQ-a27895ed-2ca4-45cc-901a-466befd02cf7（[REQ-EXT-005](external-agent-runtime-play.prd.md#req-ext-005)）; O7-REQ-7283b901-ea06-49e3-956d-2bad46b6c89d（[REQ-EXT-010](external-agent-runtime-play.prd.md#req-ext-010)）

- 功能点：SC-7，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：多轮决策、权威回读及首局义务仅作组合入口；工业资源/物流/市场/威胁/治理约束到实际世界更新及玩家主因果、成本、blocker和下一步仍需同候选端到端证据。

<a id="sc-agent-007-factory-recovery"></a>
##### SC-7 已接收子范围：builtin 失败恢复与反循环

Trace ID: O7-REQ-fffbfcf7-4ab2-46b5-9771-bc14276f9521

失败原因驱动补救并避免无效重复，使 builtin 工业策略收敛到建厂和排产。 本条只为原 SC-7 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-recovery)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-recovery)拥有；恢复目标为原强制要求；初版四项映射仍是建议，N 与模板下限不在此重定。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-recovery)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-recovery)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-007-factory-nested-wait"></a>
##### SC-7 已接收子范围：builtin 已解析内嵌等待的既定处理

Trace ID: O7-REQ-bdf9e6ce-431b-4bbc-a1d0-4eaecab050e1

旧 builtin lane 的 execute_until 内嵌 wait/wait_ticks 按已选最小动作改写继续推进。 本条只为原 SC-7 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-nested-wait)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-nested-wait)拥有；仅限 LFO11 已解析的内嵌等待；不是 provider 失败 fallback，也不是 native durable Wait。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-nested-wait)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-nested-wait)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-007-factory-refine-preview"></a>
##### SC-7 已接收子范围：builtin 精炼恢复价值可读

Trace ID: O7-REQ-46cda05b-3486-48e6-a30c-75123ff2822f

硬件不足恢复建议让玩家读到精炼净收益、电力机会成本、硬件缺口与首目标关联。 本条只为原 SC-7 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-refine-preview)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-refine-preview)拥有；strategy hint 或未来升级 schema 路线；缺预览保留 refine_quote_missing，不宣称当前公共 observation 已有字段。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-refine-preview)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-refine-preview)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-007-factory-power-preview"></a>
##### SC-7 已接收子范围：builtin 电力恢复生存价值可读

Trace ID: O7-REQ-a66aa3a6-7e7b-49f7-9cef-51c94c229179

电力恢复建议让玩家读到恢复后 runway、下一步可负担性和防停机收益。 本条只为原 SC-7 已接收结果登记精确身份，完整适用条件、技术机制和参数真值仍由[专业义务](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#lfo-power-preview)及其[独立验收](../../world-simulator/llm/llm-factory-strategy-optimization.prd.md#ac-lfo-power-preview)拥有；power_survival_quote_missing 仍是 future routed diagnostic；不新增公共 last_action/schema 字段。 对应[设计承接](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#des-lfo-power-preview)和[计划验证](../../world-simulator/llm/llm-factory-strategy-optimization.design.md#ver-lfo-power-preview)不改变本根组合的未核实状态、原证据基线或剩余范围，不证明实现、玩家体验或其它 provider 已通过。

<a id="sc-agent-008"></a>
#### 功能状态卡：SC-8 组合

- SC-8：正式玩家 surface 能区分当前账号绑定/权威认领的可控制 Agent 与共享世界中的其他或未绑定 Agent；首次认领前可读一次性承诺、持续维护、可维持时间和主要风险，并在无法维持时给出真实的恢复、释放或重新规划下一步；受限启动帮助不会被表达为免费控制、通用补贴或可转让资产。

Trace refs: O7-REQ-612f4d3d-e831-48f9-9dfc-f0844690e6cf（[REQ-WR-AOS-001](../world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-001)）；O7-REQ-8b94a27c-bbae-4349-94c1-f1f40f46e287（[REQ-WR-AOS-002](../world-rules-core-gameplay/agent-ownership-and-stewardship.prd.md#req-wr-aos-002)）。

- 功能点：SC-8，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：消费AOS-001/002的认领承诺及失控恢复；本SC保留当前账号/共享Agent区分、surface可读选择和受限启动帮助不得成为免费控制的独立组合验收。

<a id="sc-agent-009"></a>
#### 功能状态卡：SC-9 组合

- SC-9：正式玩家 surface 能区分 Agent 对话、预设/草稿填充与持续 Prompt/目标调整；目标 Agent、内容来源、提交结果和缺失能力边界可读，本地填充或 request acceptance 不会被呈现为已应用。

Trace refs: O7-REQ-14ddd9ba-2d94-48e8-b2e4-0dbae26ddc36（[REQ-PROMPT-001](agent-conversation-and-prompt-control.prd.md#req-prompt-001)）; O7-REQ-a82f1ee8-9d70-442c-9972-5f49f6665293（[REQ-PROMPT-002](agent-conversation-and-prompt-control.prd.md#req-prompt-002)）; O7-REQ-395be972-59c0-4cd2-babc-f582ae659c5d（[REQ-PROMPT-003](agent-conversation-and-prompt-control.prd.md#req-prompt-003)）; O7-REQ-59fca4ef-cd56-44ee-b982-b5eb9e15a111（[REQ-PROMPT-004](agent-conversation-and-prompt-control.prd.md#req-prompt-004)）; O7-REQ-b6140b6c-ca2c-4734-84ce-7963f9385fff（[REQ-PROMPT-005](agent-conversation-and-prompt-control.prd.md#req-prompt-005)）; O7-REQ-6f348a2d-cb87-4838-8447-21c8eebf6d07（[REQ-PROMPT-006](agent-conversation-and-prompt-control.prd.md#req-prompt-006)）; O7-REQ-458146d9-d3c2-4e14-935d-29a91594e8dd（[REQ-PROMPT-007](agent-conversation-and-prompt-control.prd.md#req-prompt-007)）; O7-REQ-a9797fdb-39f7-48fb-9202-9700d9d86159（[REQ-PROMPT-008](agent-conversation-and-prompt-control.prd.md#req-prompt-008)）; O7-REQ-92124092-6021-49e4-8def-33038e20b637（[REQ-PROMPT-009](agent-conversation-and-prompt-control.prd.md#req-prompt-009)）; O7-REQ-3bbcade1-3f73-4899-b324-f857fd27b245（[REQ-PROMPT-010](agent-conversation-and-prompt-control.prd.md#req-prompt-010)）; O7-REQ-36cc9e9d-eaa7-436f-b16b-6f6af089b04c（[REQ-PROMPT-011](agent-conversation-and-prompt-control.prd.md#req-prompt-011)）

- 功能点：SC-9，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：Prompt叶项逐一保留对话/草稿/配置、目标与资格、默认/override、accepted/applied、能力缺失、可达性及迟到反馈边界；部分叶项覆盖不代表全surface组合完成。

<a id="sc-agent-010"></a>
#### 功能状态卡：SC-10 组合

- SC-10：正式玩家 surface 在受支持的 viewport、信息密度、语言和连接状态变化中仍能保留当前目标、主要 blocker、可信行动反馈与下一决策；断连恢复、语言或布局变化不会改写权威结果或代签玩家进展。

Trace refs: O7-REQ-20b49f66-b663-41ed-9b52-253ce76ed1ef（[REQ-AGENT-SURFACE-001](player-readable-surface-continuity.prd.md#req-agent-surface-001)）; O7-REQ-d7081137-94a9-4524-b08a-fc3fbfcb9f64（[REQ-AGENT-SURFACE-002](player-readable-surface-continuity.prd.md#req-agent-surface-002)）; O7-REQ-41b03765-cccf-4a1f-814c-7b498dc975f6（[REQ-AGENT-SURFACE-003](player-readable-surface-continuity.prd.md#req-agent-surface-003)）; O7-REQ-e3a70494-b3f3-4220-a654-6c9f07cdb8a4（[REQ-AGENT-SURFACE-004](player-readable-surface-continuity.prd.md#req-agent-surface-004)）; O7-REQ-e393156d-2326-4d75-a0cb-ffba6ed74e5d（[REQ-AGENT-SURFACE-005](player-readable-surface-continuity.prd.md#req-agent-surface-005)）

- 功能点：SC-10，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：surface叶项保留viewport/语言、连接/未知结果、反馈归属、typed错误与preflight的各自边界；正式入口组合及真实玩家可读性没有由单项代码或设计证明。

<a id="sc-agent-011"></a>
#### 功能状态卡：SC-11 组合

- SC-11：玩家可在高自治和有界授权模式间作出可读选择；严重后果只在有效的提前授权或后续有效确认下执行。Agent 团队扩张、转让、异议、owner override 与组织责任保持身份连续、授权边界和可审计因果，不把 Agent 当作责任替身。

Trace refs: O7-REQ-bd548350-7fce-4026-aeec-5b26a10b86ad（[REQ-AGENT-AUTH-001](agent-authority-ownership-and-accountability.prd.md#req-agent-auth-001)）; O7-REQ-be254f99-9b66-47b1-ad7f-4a356d296b2a（[REQ-AGENT-AUTH-002](agent-authority-ownership-and-accountability.prd.md#req-agent-auth-002)）

- 功能点：SC-11，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：本专题AC-1～9保留自治选择、高后果授权、累计额度、团队扩张、工业供给、转让、异议/override和责任归因；不能由两项局部授权/历史Trace代签全部组合。

<a id="sc-agent-012"></a>
#### 功能状态卡：SC-12 组合

- SC-12：代表性 Agent 样例证明可选择 provider profile 只能经证据优先的准入、有限试点/分层/范围与暂停/撤销后使用，且不会改变固定权威节奏或 action slots；训练、认证、重训与情报在可审计、可更新和安全披露边界下形成连续历史，不以现有 Local Provider parity 或局部证据代签当前 readiness。

Trace refs: O7-REQ-86ed9522-e22d-4706-9b1e-3e1452a9fb3c（[REQ-AGENT-LEARNING-001](provider-learning-intelligence-and-cadence.prd.md#req-agent-learning-001)）; O7-REQ-dc313559-e262-478c-8a8d-ee2f9817e5e7（[REQ-AGENT-LEARNING-002](provider-learning-intelligence-and-cadence.prd.md#req-agent-learning-002)）

- 功能点：SC-12，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：provider准入/暂停/撤销、固定cadence、训练重训历史、情报私有/披露/更新均沿叶项完整范围；Local Provider parity不是该长期组合已设计或已交付证据。

<a id="sc-agent-013"></a>
#### 功能状态卡：SC-13 组合

- SC-13：长期世界舞台保持世界、Agent/路线、blocker 与下一步为默认 primary decision surface；真实 Web 和受支持桌面客户端分别可从世界总览浏览区域/Fragment、缩放平移、定位选择与查看对象和事件并返回合法指导；terrain/blocks 不暗示直接 edit/harvest/build，不将长期可视化方向误报为当前 2D/native/zoom 已交付。

Trace refs: O7-REQ-24cbdd98-cf69-4f7b-8342-6216b1b91616（[REQ-AGENT-STAGE-001](player-readable-world-stage.prd.md#req-agent-stage-001)）; O7-REQ-8b88187a-985b-4cf0-a9d4-b6137882ffc3（[REQ-AGENT-STAGE-002](player-readable-world-stage.prd.md#req-agent-stage-002)）; O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf（[REQ-AGENT-STAGE-003](player-readable-world-stage.prd.md#req-agent-stage-003)）; O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a（[REQ-AGENT-STAGE-004](player-readable-world-stage.prd.md#req-agent-stage-004)）; O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096（[REQ-AGENT-STAGE-005](player-readable-world-stage.prd.md#req-agent-stage-005)）

- 功能点：SC-13，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：舞台叶项的局部专业设计不代签Web/native分别浏览、下钻、定位、间接指导及renderer降级的组合；当前表现与长期空间目标仍须分开。

<a id="sc-agent-014"></a>
#### 功能状态卡：SC-14 组合

- SC-14：OpenClaw、Codex 各自在声明组合中驱动合法绑定 Agent 完成代表性首局，包括自主多轮决策、合法能力、权威反馈、持续目标应用、无 GUI 运行和恢复；玩家另通过真实 Web 和受支持桌面客户端分别探索同一大世界、定位 Agent、查看权威结果并合法间接指导。玩家与执行方还须能围绕同一任务发布并确认目标、汇报、问答、明确授权及断线后恢复；目标/授权的当前版本不能被旧消息覆盖。API、旧 P0 smoke、模型连通或另一 surface 的证据不代签完整游戏体验。

Trace refs: O7-REQ-b3e367c3-c72d-4959-b599-613ac652dab8（[REQ-EXT-001](external-agent-runtime-play.prd.md#req-ext-001)）; O7-REQ-d30925fe-d798-4056-9641-062255931a61（[REQ-EXT-002](external-agent-runtime-play.prd.md#req-ext-002)）; O7-REQ-aad7b341-83b1-4402-8ebc-8540db36ccfb（[REQ-EXT-003](external-agent-runtime-play.prd.md#req-ext-003)）; O7-REQ-bd792456-4b98-4891-b5e3-e24b1de4f1a1（[REQ-EXT-004](external-agent-runtime-play.prd.md#req-ext-004)）; O7-REQ-a27895ed-2ca4-45cc-901a-466befd02cf7（[REQ-EXT-005](external-agent-runtime-play.prd.md#req-ext-005)）; O7-REQ-091d6c22-fca5-4284-91dd-2edce9ef440b（[REQ-EXT-006](external-agent-runtime-play.prd.md#req-ext-006)）; O7-REQ-c5c90e09-ae74-4d77-9858-a4769562b01e（[REQ-EXT-007](external-agent-runtime-play.prd.md#req-ext-007)）; O7-REQ-4a8be58c-bcda-4efe-b199-1cf9bd9b3330（[REQ-EXT-008](external-agent-runtime-play.prd.md#req-ext-008)）; O7-REQ-cb69c2a7-f13a-42ea-a929-214fa7ff9d43（[REQ-EXT-009](external-agent-runtime-play.prd.md#req-ext-009)）; O7-REQ-7283b901-ea06-49e3-956d-2bad46b6c89d（[REQ-EXT-010](external-agent-runtime-play.prd.md#req-ext-010)）; O7-REQ-a5bbf6e4-3e71-4d48-9eba-68c50867c95c（[REQ-EXT-011](external-agent-runtime-play.prd.md#req-ext-011)）; O7-REQ-8120dffe-17cc-458a-9c70-8121217e6f7b（[REQ-EXT-012](external-agent-runtime-play.prd.md#req-ext-012)）; O7-REQ-7d6cef7f-9419-49ed-b445-663382383458（[REQ-EXT-013](external-agent-runtime-play.prd.md#req-ext-013)）; O7-REQ-68ffc843-8679-4ad9-8cef-10a239a29c73（[REQ-EXT-014](external-agent-runtime-play.prd.md#req-ext-014)）; O7-REQ-ecbc5765-ec25-4bfe-9aaa-4bbdd0a3064a（[REQ-EXT-015](external-agent-runtime-play.prd.md#req-ext-015)）; O7-REQ-2ede7851-f1c0-4851-8d6b-5da4afcdefcf（[REQ-EXT-016](external-agent-runtime-play.prd.md#req-ext-016)）; O7-REQ-9cb09cbd-3c98-4742-a290-ea98a64ab096（[REQ-EXT-017](external-agent-runtime-play.prd.md#req-ext-017)）; O7-REQ-62007da5-b12a-4b8c-98bb-82ab53a35069（[REQ-COLLAB-001](player-runtime-collaboration.prd.md#req-collab-001)）; O7-REQ-edd52061-73c0-433c-9531-30d86b1db459（[REQ-COLLAB-002](player-runtime-collaboration.prd.md#req-collab-002)）; O7-REQ-b100c55e-5c35-46dc-aaf6-e66c02b323d8（[REQ-COLLAB-003](player-runtime-collaboration.prd.md#req-collab-003)）; O7-REQ-2f463269-c3c0-4da1-95f8-90011c35799a（[REQ-COLLAB-004](player-runtime-collaboration.prd.md#req-collab-004)）; O7-REQ-f5341eef-d8f6-42ae-b7b6-751803abd588（[REQ-COLLAB-005](player-runtime-collaboration.prd.md#req-collab-005)）; O7-REQ-6044cb04-5fbb-4c68-81a6-fc57cfdd608a（[REQ-COLLAB-006](player-runtime-collaboration.prd.md#req-collab-006)）; O7-REQ-8dad5caa-d064-4ac8-8c98-40a702b13685（[REQ-COLLAB-007](player-runtime-collaboration.prd.md#req-collab-007)）

- 功能点：SC-14，本卡成功标准原文与 §5.1 对应行共同限定范围。
- 系统设计状态：未核实
- 系统设计依据：已核对 §5.1 专业责任与上列精确叶项/组合关系；叶项已确认的专业条款仅证明其实际子范围，尚不能判定整个根组合的系统设计覆盖。
- 实现状态：未核实
- 实现与验证依据：叶项中固定源码或历史 Task 只证明原候选、入口和局部范围；尚无本根全部适用条件的同候选组合证据，未执行本 SC 行为验收。
- 核对日期与基线：2026-10-15；需求及设计审读基线 `ef28aa8461e45fe277ca2ec28b485a1c9a0ac0ad`；只读文档及已记录证据核对，无新增 Runtime、provider、浏览器、部署或真人运行。
- 剩余范围：外部Runtime、世界舞台复用叶项及协作七项共同承接；OpenClaw/Codex、Web/native各自的完整首局、授权/问答/目标版本与恢复不能相互代签，历史prepared-world原生切片不等于当前组合通过。

## 6. Non-Goals

- 不在产品层决定 Viewer 组件、Launcher 协议、provider 或 runtime 实现。
- 不将 mock、software-safe、本地回退或单次截图包装为真实产品闭环。
- 不复制 `world-simulator` 专题 PRD、project 任务或测试步骤。
