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

## 1. 产品承诺

玩家进入的是场景、Agent 意图、世界事件和交互反馈相互对应的持久模拟，而不是一次性聊天或本地假世界。

Agent 的意图在进入世界后必须经过统一权威裁决；并发冲突、重复提交或跨入口重试不得产生重复世界效果，并必须留下玩家可理解、可回放的接受、拒绝、改道或替代原因。

## 2. 范围与玩家边界

覆盖可重现场景、Agent/LLM provider 权威、世界事件流、主 Web 界面、Launcher 转移和交互反馈。玩家可以观察、发起被允许的意图并看到权威结果；不能把 mock、本地演示或未授权 provider 输出当作世界状态。

Agent 可以基于世界观测、工业物流、市场、风险与治理约束形成计划，但这些输入可能延迟、过期或不完整；当其影响当前决策时，产品必须区分当前事实与陈旧/不确定情报，并允许刷新、纠正或重排，不能把缓存信息伪装成实时真值。

模型、provider 或配置只影响 Agent 如何提出决策，不改变世界规则与执行权威；缺失、无效或不可用的推理配置必须形成可诊断阻塞或失败，保持世界状态不被伪造成功或被静默替代策略改写。

### 2.1 跨域闭环

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

## 6. Non-Goals

- 不在产品层决定 Viewer 组件、Launcher 协议、provider 或 runtime 实现。
- 不将 mock、software-safe、本地回退或单次截图包装为真实产品闭环。
- 不复制 `world-simulator` 专题 PRD、project 任务或测试步骤。
