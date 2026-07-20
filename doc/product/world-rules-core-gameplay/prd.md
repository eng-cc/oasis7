# 世界规则与核心玩法 PRD

## 文档身份

- 产品模块：世界规则与核心玩法
- 产品模块 slug：`world-rules-core-gameplay`
- 产品层唯一 PRD：`doc/product/world-rules-core-gameplay/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-001`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-07-19`
- 后继文档：`无`
- 下层专业域：[`doc/game/prd.md`](../../game/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)

本文只定义玩家目标、间接能动性、核心循环、成长与资源压力的产品承诺。玩法规则、数值平衡、专题 PRD-ID 与测试证据由 `game` 专业域拥有。

### 活跃产品专题

- [`首局与持续游玩`](first-session-and-continuation.prd.md)：首局微循环、后引导承接、首次持续能力、中循环展开与失败恢复。

## 1. 产品承诺

玩家通过可读、有代价、有反馈的行动持续影响同一个持久世界，并在权威规则内与其他玩家、Agent 和区域系统产生可审计的涌现结果。

## 2. 范围与玩家边界

覆盖首局目标、micro-loop、后引导承接、间接控制、资源压力与长期参与。玩家可以观察、决策、行动并处理反馈；不能越过资源、时间、权限、治理或反滥用边界直接改写世界。

### 物理尺度、间接控制与未来候选

玩家影响的是一个有物理尺度且持续存在的世界，但当前默认体验是通过目标、Agent、地点、设施、配方和治理等间接动作推进，而不是第一人称逐块编辑。表现层可以为可读性抽象或夸张，但不得把它呈现为世界物理真值。

当玩家提出当前未开放的过细动作时，产品体验必须给出可执行的 canonical 替代动作；没有安全替代时，必须说明边界和下一次可决策点，而不是伪造动作或只留下无解释的失败。具身或 block-editing 仅是未来候选：只有在强化本模块的间接控制主路线、具备对应专业域合同与验证，并经显式跨域决策后才可进入原型。

本产品层只定义上述玩家承诺和端到端边界；玩法动作粒度由 [`doc/game/prd.md`](../../game/prd.md) 与其核心玩法骨架拥有，物理/执行真值由 `world-runtime`，表现真值由 `world-simulator` 的对应专业域文档拥有。

世界只有一条持续、权威的时间线和一套可测量的物理真值。厘米级距离、顺序、成本与持久化结果可以由工业、物流、治理等粗粒度子系统消费，也可以由 Viewer 做可读性抽象，但任何映射都必须确定、可追溯，不能因子系统分辨率或视觉夸张而改写权威结果。

间接控制不等于旁观：对当前受支持的玩家意图，系统必须呈现意图是否被接受、Agent 如何解释并执行、主要世界后果，以及玩家可用的打断、重排、纠正、fallback 或恢复动作；不能以 Agent 自主性为由隐藏因果或让玩家失去下一次决策权。

### Data 所有权与授权边界

Data 是有归属、有获取成本且受授权边界约束的世界资源。未经授权的使用必须原子失败，不产生未授权收益；产品体验需要说明成本、归属、用途、授权状态和可恢复的授权或替代路径，且可读性层或 Agent 自动化不能静默绕过权限。通用资源与 runtime 领域模型、经治理模块记录之间的产品边界由[大世界基础设施资源模型](../world-infrastructure/prd.md#25-资源模型与模块扩展边界)统一定义。

### 世界宪法级产品不变量

- 玩家通过目标、Agent、地点、设施、配方、关系与治理等受支持动作获得间接战略能动性；资源变化必须来自被授权的 source/sink 因果链，不能凭空生成或绕过成本。
- 每个权威行动都必须经过规则与权限校验并产生可审计后果；玩家能够读懂 target、action、cost、blocker、result、next decision 与 recovery，不靠隐藏状态猜测世界为何变化。
- 社会关系、组织、市场与制度可以在权限、治理和 anti-abuse 边界内产生有限涌现，但不会因此获得绕过权威规则的能力。世界保持持久、开放式演化，不要求强制终局。
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

## 5. Done：成功标准与验收

- SC-1：玩家在首局可识别当前目标、可执行动作、行动代价与下一步。
- SC-2：核心循环完整呈现行动接受、推进、阻塞、反馈和结果。
- SC-3：FirstSessionLoop 之后存在可达的 PostOnboarding 目标、压力与承接。
- SC-4：世界规则、资源消耗与玩家结果可映射到专业 PRD-ID 和验证证据。
- SC-5：玩家的间接战略动作具备授权资源因果、权威校验和可审计后果，界面或接口可读 target/action/cost/blocker/result/next/recovery；涌现关系与组织不绕过权限、治理或 anti-abuse，未支持细粒度请求有 canonical 替代或安全停止。
- SC-6：代表性间接控制流程证明玩家意图进入 Agent/策略决策，经权威规则与资源校验产生世界后果，并返回可解释结果与可执行的打断、纠正、下一步或恢复动作；任一专业域的局部 green 不能替代组合闭环。
- SC-7：同一物理行动在 gameplay、runtime、Agent 与 Viewer 的粗粒度/表现映射中保持距离、顺序、成本和持久化结果一致，不产生第二条时间线或表现层真值；权威时间线在没有直接玩家输入时仍按当前世界规则继续推进，不冻结具体 tick 时长。
- SC-8：Data 的获取和一次授权使用路径可端到端验证；未经授权的使用原子失败且不产生旁路收益，并向玩家提供可理解的原因和恢复或替代路径。具体许可状态机、结算规则与副作用矩阵由专业域拥有。

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

## 6. Non-Goals

- 不在产品层冻结新的玩法细则、数值、掉落或成长曲线。
- 不把分布式执行、任意 WASM 或全局治理包装成当前玩家默认能力。
- 不复制 `game` 专题 PRD、project 任务或测试步骤。
