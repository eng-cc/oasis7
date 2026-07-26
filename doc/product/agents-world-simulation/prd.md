# 智能体与世界模拟 PRD

## 文档身份

- 产品模块：智能体与世界模拟
- 产品模块 slug：`agents-world-simulation`
- 产品层唯一 PRD：`doc/product/agents-world-simulation/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-003`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-07-19`
- 后继文档：`无`
- 下层专业域：[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)

本文只承载场景、Agent/provider 决策、权威世界状态与可交互表达的产品组合。Viewer、Launcher、provider 与模拟合同由 `world-simulator` 专业域拥有。

### 活跃产品专题

- [Agent/provider 体验连续性](provider-agent-experience-continuity.prd.md)：只收敛玩家体验承诺；provider 组合的场景范围、评估方法与结论仍由专业域权威文档维护。
- [玩家可读的世界舞台](player-readable-world-stage.prd.md)：约束正式世界表面的首读层级、空间关系来源、可归因玩家因果与按需诊断边界。
- [玩家可读表面的连续性](player-readable-surface-continuity.prd.md)：约束 viewport、信息密度、语言与连接状态变化时，主要决策面仍可读、可操作且可恢复。
- [Agent 对话与 Prompt 控制](agent-conversation-and-prompt-control.prd.md)：区分一次对话、预设/草稿与持续 Prompt/目标调整，并配对承载跨 surface 产品交互设计和迁移追踪。

## 1. 产品承诺

玩家进入的是场景、Agent 意图、世界事件和交互反馈相互对应的持久模拟，而不是一次性聊天或本地假世界。

Agent 的意图在进入世界后必须经过统一权威裁决；并发冲突、重复提交或跨入口重试不得产生重复世界效果，并必须留下玩家可理解、可回放的接受、拒绝、改道或替代原因。

## 2. 范围与玩家边界

覆盖可重现场景、Agent/LLM provider 权威、世界事件流、主 Web 界面、Launcher 转移和交互反馈。玩家可以观察、发起被允许的意图并看到权威结果；不能把 mock、本地演示或未授权 provider 输出当作世界状态。

Agent 可以基于世界观测、工业物流、市场、风险与治理约束形成计划，但这些输入可能延迟、过期或不完整；当其影响当前决策时，产品必须区分当前事实与陈旧/不确定情报，并允许刷新、纠正或重排，不能把缓存信息伪装成实时真值。

模型、provider 或配置只影响 Agent 如何提出决策，不改变世界规则与执行权威；缺失、无效或不可用的推理配置必须形成可诊断阻塞或失败，保持世界状态不被伪造成功或被静默替代策略改写。

### 2.1 玩家与可控制 Agent 的承诺边界

玩家对 Agent 的直接策略控制来自当前账号已经绑定或按权威规则认领的 Agent；共享世界中可见的其他 Agent、未绑定 Agent 或默认选中对象不因此成为“我的 Agent”。首个可控制 Agent 也不是免费席位：确认前，玩家必须能理解一次性承诺、持续维护、可维持时间和主要失去控制风险，并在认领、比较候选、等待或先补足资源之间作出可读选择。面向首个认领的受限启动帮助只能降低进入摩擦，不得被表达为免费控制、通用补贴或可转让资产。若持续承诺无法维持或所有权不再成立，体验必须如实说明状态、恢复/释放或重新规划的下一步，而不是静默保留、静默清退或把别人的 Agent 当作玩家可操作对象。详细的认领报价、维护、回收与反滥用专业合同见 [`PRD-GAME-011`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md)，产品层不复制其字段、数值或状态机。

### 2.2 跨域闭环

`世界观测与工业/市场/物流/风险约束 -> Agent 形成意图与理由 -> runtime 统一校验、冲突裁决和提交 -> 世界事件/资源/治理状态更新 -> Viewer 或 pure API 回写 accepted intent、执行状态、主因果、成本/进展、阻塞与下一步 -> 玩家纠正、打断或重排`

Agent 不得绕过世界规则与基础设施约束；当工业、市场或治理数据实质影响当前 Agent 意图或玩家可见结果时，必须进入可追踪的因果反馈。

## 3. 权威与冲突处理

| 产品层拥有 | 专业域权威 |
| --- | --- |
| 场景、Agent/provider、世界状态到交互模拟的端到端承诺 | `doc/world-simulator/prd.md` 拥有 Viewer、Launcher、Agent/provider、场景和验证合同；`doc/game/prd.md` 拥有间接控制与玩家能动性；`doc/world-runtime/prd.md` 拥有权威执行和冲突语义 |

产品 PRD 不改写具体模式、API、UI 或 provider 选择；冲突时由对应 `agent_engineer`、`viewer_engineer` 或 runtime owner 与产品 owner 共同裁决。

相邻产品组合依赖：[`doc/product/world-infrastructure/prd.md`](../world-infrastructure/prd.md) 定义工业经济底座的产品承诺；本模块消费该承诺形成 Agent 端到端模拟，但不能以相邻产品 PRD 代替 game/runtime/world-simulator 的专业规则与验证证据。

## 4. 路线图

1. 可重现场景：初始状态、资源变化、关键事件与验收证据一致。
2. 权威 Agent 闭环：provider 和 Agent 意图不绕过世界校验。
3. 多入口等价：主 Web、Launcher 与 pure API 在权威状态与核心动作上一致。

## 5. Done：成功标准与验收

- SC-1：关键场景可从固定初始状态重现到相同权威事件和交互结果。
- SC-2：Agent/provider 的输入、决策、世界接受或拒绝及玩家反馈可追踪。
- SC-3：主 Web 入口呈现真实世界状态、核心动作和结构化失败。
- SC-4：Launcher、Viewer 与 pure API 不会将不同模式的证据或 claim 混为同一事实。
- SC-5：固定场景中以不同但等价的提交顺序重放同一冲突裁决窗口的并发意图，并加入重复投递或跨入口重试时，accepted/rejected 集合、单次权威世界效果及接受、拒绝或替代原因保持确定且在 replay、Viewer 与 pure API 上语义一致。
- SC-6：Agent 基于过期或不完整观测做出决定时，正式体验能标记情报新鲜度或不确定性，并提供刷新、纠正或重排路径；不得让 stale observation 静默驱动长期行动。
- SC-7：至少一条 Agent 驱动的工业闭环可端到端验证：Agent 消费资源、物流、市场、威胁或治理约束形成意图，runtime 接受或拒绝并更新权威世界，玩家随后能读到主因果、成本/进展、阻塞和下一步。
- SC-8：正式玩家 surface 能区分当前账号绑定/权威认领的可控制 Agent 与共享世界中的其他或未绑定 Agent；首次认领前可读一次性承诺、持续维护、可维持时间和主要风险，并在无法维持时给出真实的恢复、释放或重新规划下一步；受限启动帮助不会被表达为免费控制、通用补贴或可转让资产。
- SC-9：正式玩家 surface 能区分 Agent 对话、预设/草稿填充与持续 Prompt/目标调整；目标 Agent、内容来源、提交结果和缺失能力边界可读，本地填充或 request acceptance 不会被呈现为已应用。
- SC-10：正式玩家 surface 在受支持的 viewport、信息密度、语言和连接状态变化中仍能保留当前目标、主要 blocker、可信行动反馈与下一决策；断连恢复、语言或布局变化不会改写权威结果或代签玩家进展。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| SC-1 | runtime_engineer | PRD-WORLD_SIMULATOR-001/002/003 | `doc/world-simulator/prd.md` | 场景初始化、事件与回放证据 | test_tier_required |
| SC-2 | agent_engineer | PRD-WORLD_SIMULATOR-016-019 | `doc/world-simulator/prd.md` | live provider 和权威行为闭环 | test_tier_required |
| SC-3 | viewer_engineer | PRD-WORLD_SIMULATOR-039/041/046 | `doc/world-simulator/prd.md` | 主 Web Viewer 真实世界交互证据 | test_tier_required |
| SC-4 | qa_engineer | PRD-WORLD_SIMULATOR-020-031 | `doc/world-simulator/prd.md` | 模式、转移与多入口 claim 回归 | test_tier_required |
| SC-5 | runtime_engineer / agent_engineer / gameplay_designer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-001 / PRD-WORLD_RUNTIME-001 / PRD-GAME-008 / PRD-TESTING-003 | `doc/world-simulator/prd.md`; `doc/world-runtime/prd.md`; `doc/game/prd.md`; `doc/testing/prd.md` | 等价重排序、冲突、重复投递、重试与 replay 的 accepted/rejected 集合、单次效果，以及 Viewer/pure API 解释一致性证据，包含 S6 玩家 surface 核对 | test_tier_full |
| SC-6 | agent_engineer / viewer_engineer | PRD-WORLD_SIMULATOR-016 | `doc/world-simulator/prd.md` | 观测新鲜度、不确定性与刷新/纠正/重排路径证据，包含正式玩家 surface 的 S6 交互闭环 | test_tier_required |
| SC-7 | agent_engineer / gameplay_designer / runtime_engineer / viewer_engineer | PRD-WORLD_SIMULATOR-001 / PRD-WORLD_SIMULATOR-016 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 | `doc/world-simulator/prd.md`; `doc/game/prd.md`; `doc/world-runtime/prd.md` | Agent 工业约束到权威后果与玩家反馈的端到端证据；相邻产品组合承诺由 `doc/product/world-infrastructure/prd.md` 提供 | test_tier_required |
| SC-8 | producer_system_designer / gameplay_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-011 / PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-039 / PRD-TESTING-003 | `doc/game/gameplay/gameplay-agent-claim-economy-contract.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 账号绑定/认领可读性、首次承诺与持续维护选择、受限启动帮助边界，以及无法维持时的恢复/释放/重规划玩家 surface 证据；不复制数值、字段或状态机 | test_tier_required |
| SC-9 | producer_system_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-016 / PRD-WORLD_SIMULATOR-039 / PRD-TESTING-003 | `doc/product/agents-world-simulation/agent-conversation-and-prompt-control.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 对话、草稿、默认/override、accepted/applied/rejected/blocked 与窄屏可达性对账 | test_tier_required |
| SC-10 | producer_system_designer / viewer_engineer / qa_engineer | PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/product/agents-world-simulation/player-readable-surface-continuity.prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | desktop、窄屏/低高度、双语 fallback 与断连恢复中的决策锚点、权威结果和下一步对账 | test_tier_required |

## 6. Non-Goals

- 不在产品层决定 Viewer 组件、Launcher 协议、provider 或 runtime 实现。
- 不将 mock、software-safe、本地回退或单次截图包装为真实产品闭环。
- 不复制 `world-simulator` 专题 PRD、project 任务或测试步骤。
