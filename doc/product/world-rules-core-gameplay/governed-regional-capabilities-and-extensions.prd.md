# 受治理的区域能力与扩展

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`区域设施合同`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md)、[`玩家发布实体合同`](../../world-runtime/module/player-published-entities.prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文承载受治理的区域设施与工业能力扩展的长期产品承诺：玩家以有限、可读和可审计的方式改变局部世界，授权创作者也只能经治理把新的可用能力接入同一权威世界。它不把每项设施或每个制成品写成独立产品入口，也不冻结实现合同、数值或当前可用性。

## 1. 产品承诺

在完成基础能力后，玩家可以针对可读的区域压力评估并投入有限资源，获得有明确作用域、收益、维护或耗尽边界以及可追溯结果的区域能力。设施不是默认必点税、第一轮教学动作、自由建造或全局治理权。

世界可以在授权创作者和治理流程下扩展新的工业能力；扩展从提案到可用结果始终受同一权威世界、审计和恢复边界约束，不能覆盖既有事实、绕过治理或把技术提交伪装成默认玩家能力。

## 2. 玩家边界与组合关系

- 区域设施经历 `发现压力 -> 报价与取舍 -> 明确提交 -> 有限服务 -> 维护、耗尽、恢复或退役` 的可读生命周期；每一阶段说明成本、约束、预期价值、失败和下一步。
- 设施应在玩家已经理解基础控制权、资源压力和区域 blocker 后提供有限区域 leverage；低价值或不合适的选择必须仍可解释，而不被呈现为必选 buff。
- 受治理扩展让新制成品、配方、工厂或等价工业能力在批准后进入可用世界能力；玩家和创作者不能任意上传、直接写入权威状态、覆盖既有世界身份或取得默认全局治理权。
- 设施和扩展都必须保留世界唯一性：玩家报价、确认、权威执行、持久结果、恢复和重连后的可见结果属于同一条可审计链路。
- 报价只描述当前条件，不预留资源、设施容量、价格、资格或排队顺位；提交时必须按最新权威状态重新校验。报价后条件变化时，系统只能接受一次并产生一个权威结果，或原子拒绝并返回更新后的 blocker、取舍和下一步。
- 重复、过期、重连或跨入口重试不能产生第二次扣减、设施服务、发布或恢复结果。尚未获得权威确认的交接、发布或恢复保持待决；玩家和创作者不能把请求送达、技术构建成功或本地缓存当作已生效能力。
- **扩展授权不预留发布权：** 扩展提案获批只说明它在当时取得了受限的准入资格，并不使制成品、配方、工厂或等价能力自动进入世界。每次待决发布或激活都必须按当时仍有效的治理授权、已批准的能力范围和世界前置重新校验；若授权撤销、到期、收缩、被替代或能力范围已不再匹配，请求只能明确拒绝、过期、取消或在当前有效轨道重新提交，不能继承旧批准、静默迁移到新授权或以部分可用状态绕过审查。已经由权威 receipt 确认的结果保留其历史因果，但不授权补充发布、再次激活或追溯改变既有世界事实。
- 可扩展不等于已公开、无限容量或所有创作均已支持；当前公开状态与实际证据继续由根 README 和专业域拥有。

## 3. 权威边界

| 层级 | 本产品分册拥有 | 下层专业域拥有 |
| --- | --- | --- |
| 区域价值 | 设施解决局部压力的范围、取舍和有限 leverage | `game` 拥有设施玩法、经济与专业验收 |
| 扩展治理 | 新工业能力必须经治理进入同一权威世界 | `world-runtime` 与 WASM 专业域拥有发布、校验、执行和审计合同 |
| 持久世界 | 结果可审计、可恢复且不形成第二世界 | `world-runtime` 与 `p2p` 拥有状态、回放、复制和恢复证明 |
| 验证 | 组合证据证明玩家可读价值和权威结果一致 | `testing` 与 QA 拥有测试矩阵、样本和当前 verdict |

专业入口分别是 [`gameplay-regional-infrastructure-micro-depot-contract.prd.md`](../../game/gameplay/gameplay-regional-infrastructure-micro-depot-contract.prd.md) 与 [`player-published-entities.prd.md`](../../world-runtime/module/player-published-entities.prd.md)。本分册不复制资源数值、设施库存、WASM/ABI、module hash、profile schema、审批角色、签名、SLA、状态机、receipt 字段或任务证据。

## 4. 组合验收

- GR-1：代表性区域设施流程贯通压力发现、报价与取舍、玩家确认、权威执行、有限服务或失败、可读 receipt，以及维护、耗尽、恢复或退役的下一步。
- GR-2：设施样例证明有限区域价值与可选择的专业化，而非首局强制动作、无条件 buff、自由建造或全局治理权。
- GR-3：代表性受治理扩展流程贯通创作者提案、授权审查、权威生效、世界可用能力、审计与异常恢复；任一阶段不能用技术旁路替代治理。
- GR-4：设施和扩展的同一身份在玩家表达、权威执行、持久化/replay、适用的复制或恢复以及重连可见结果中保持一致。
- GR-5：产品、game、runtime、WASM、P2P 与 testing 证据绑定同一候选；单独的传输 green、模块提交、文档迁移或局部 UI 不得代签端到端闭环。
- GR-6：报价后资源、容量、资格、治理状态或世界前置发生变化时，提交只能按最新权威状态接受一次或原子拒绝；扩展批准撤销、到期、收缩或被替代时，待决发布/激活不会继承旧批准、部分生效或静默迁移到新授权。重复、过期、重连与跨入口重试不会产生第二次扣减、服务、发布或恢复结果，待决状态不会被表达为已生效。

### 4.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| GR-1 / GR-2 | gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 | 设施报价到有限生命周期和玩家结果的组合证据 | test_tier_required |
| GR-3 | runtime_engineer / wasm_platform_engineer / gameplay_designer | PRD-WORLD_RUNTIME-010 / PRD-WORLD_RUNTIME-011 / PRD-WORLD_RUNTIME-012 | 受治理扩展从提案到可用能力、拒绝和恢复的证据 | test_tier_required |
| GR-4 | runtime_engineer / blockchain_ops_engineer / viewer_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-P2P-002 | 同一能力跨执行、回放、恢复和重连的组合证据 | test_tier_full |
| GR-5 | producer_system_designer / qa_engineer | PRD-TESTING-003 | 同候选跨域组合审计 | test_tier_full |
| GR-6 | gameplay_designer / runtime_engineer / wasm_platform_engineer / viewer_engineer / qa_engineer | PRD-GAME-016 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-010 / PRD-TESTING-003 | 报价后状态变化、扩展授权失效后的待决发布/激活、重复/过期/重连/跨入口提交与待决表达的负例证据 | test_tier_required |

## 5. Non-Goals

- 不承诺任意 WASM 上传、游戏内 IDE、自由建造、无限设施、自动补货或无成本持续能力。
- 不冻结设施资源成本、库存、吞吐、ROI、服务半径或回收规则。
- 不定义 WASM ABI、module hash、profile schema、审批角色、签名阈值、发布 SLA、状态机或 replay 实现。
- 不把技术提案、内部测试或文档迁移表述为已经公开可用或广泛发行。
