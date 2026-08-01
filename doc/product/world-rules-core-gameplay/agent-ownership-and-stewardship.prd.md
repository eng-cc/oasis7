# Agent 所有权与持续经营

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`Agent claim 经济合同`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文承载玩家取得、维持和结束 Agent 控制权的长期产品承诺。它补充首局与持续游玩、间接控制和成熟世界成长分册，但不冻结经济数值、状态字段、身份实现、界面结构、运营操作、测试步骤或当前放行结论。

## 1. 产品承诺

玩家以明确、非零且可理解的承诺取得自己的 Agent 控制权，并持续看见这份控制权的成本、义务、风险和下一次选择。首个支持路径可以降低进入门槛，但不会把 Agent 所有权变成免费、可套现或可无限复制的能力。

玩家只能把已绑定或已拥有的 Agent 视为自己的可操作对象；共享世界中的其他 Agent 可以构成世界背景与机会，但不会被误导为当前玩家的控制权。

## 2. 玩家边界与组合关系

- 在确认前，玩家能读懂取得控制权的主要投入、持续义务、候选差异或风险，以及继续、比较、等待或补足条件等下一步。
- 取得后，玩家能理解控制权是否有效、何时需要维持、可能因何种可读原因失去或主动结束，以及相应的恢复或重新选择路径。
- 首个支持路径只帮助受支持玩家完成首个明确承诺；它不产生自由可转移财富，不绕过后续容量、维护、反滥用或世界权威边界。
- Agent 所有权保持排他、可归因和可审计；玩家不能通过旁路、误认世界实体或静默自动提交获得控制权。
- 更高的 Agent 容量属于长期成长的受限选择，不是默认囤积权，也不替代区域贡献、专业化或世界治理的其他前置条件。

### 2.1 首个 Agent 认领承诺包

首个 Agent 认领是一次持续的世界参与承诺，而不是只回答“能否支付”的解锁。玩家确认前必须能用一个连贯的承诺包回答：

- 当前候选为何适合或不适合当前目标：其可理解的用途、与其他候选的主要差异，以及这项差异对第一个目标的帮助或风险。
- 此次确认的非零 upfront 成本，以及确认后能够支撑多久的 upkeep runway；持续义务不得被包装为一次性取得。
- 哪些可读条件会导致主动结束、进入风险状态或被回收，从而保留什么、失去什么；玩家必须能看见补足、恢复、重新选择或安全结束的下一步。
- 当前最好的替代决策：比较其他候选、等待一个可理解的条件，或先解决资金/世界 blocker；当没有安全替代时也必须明确说明，不能把立即确认伪装为唯一无代价选择。

受限的首个认领资助只帮助符合条件的玩家承担这次 `slot-1` 的非零 claim/upkeep 承诺，且不能转成自由财富、设施/材料库存、liquid starter OC 或持续补贴。它与完成 Agent 已存在后、为首次对话授予 liquid starter OC 的独立首聊解锁不同：后者不支付或延长 claim/upkeep，也不将认领变成免费或设施补贴。具体 bucket、资格、余额、回收计算和对话 gate 由专业域维护；跨模块来源与 sink 边界见[资源模型与跨模块 provenance](prd.md#resource-model-and-cross-module-provenance)。

## 3. 权威边界

| 层级 | 本产品分册拥有 | 下层专业域拥有 |
| --- | --- | --- |
| 玩家价值 | Agent 控制权是可理解、可承诺、可维持和可结束的世界参与能力 | `game` 拥有经济规则、玩法保证和专业验收 |
| 权威状态 | 所有权、成本和失去控制权的结果属于同一权威世界因果链 | `world-runtime` 拥有校验、记账、状态、审计和恢复合同 |
| 玩家表达 | 玩家能够识别自己的 Agent、报价、风险与下一步 | `world-simulator` 拥有身份、Agent、Viewer 与 API 表达实现 |
| 验证 | 组合证据证明玩家没有免费、误认或不可解释的控制权路径 | `testing` 与 QA 拥有测试矩阵、样本和当前 verdict |

专业合同以 [`gameplay-agent-claim-economy-contract.prd.md`](../../game/gameplay/gameplay-agent-claim-economy-contract.prd.md) 为主入口。本分册不复制费用、倍率、余额来源、状态机、治理门禁、运营入口、字段或任务证据。

## 4. 组合验收

- AS-1：代表性首个控制权路径贯通可读报价、玩家明确确认、权威取得、玩家可见的有效控制权与下一步；首个路径不被包装成免费取得。
- AS-2：玩家能区分自己的可操作 Agent、未绑定 Agent 与其他玩家的 Agent；世界中存在其他实体不能替代自己的绑定或所有权证据。
- AS-3：代表性维持、主动结束或失去控制权路径能说明主要原因、保留或失去的结果，以及可用的恢复、重新选择或安全停止路径。
- AS-4：支持首个控制权的受限帮助不产生可转移财富、无限补贴、所有权旁路或对后续成长边界的豁免。
- AS-5：产品、game、runtime、Agent/Viewer 与 testing 指向同一候选和权威事实；局部 UI、运营记录或文档迁移不能单独证明产品承诺通过。
- AS-6：首个认领样例在确认前证明候选用途/差异、upfront 成本、确认后 upkeep runway、回收或失去触发、恢复/重新选择以及等待或替代动作；受限认领资助与 liquid starter OC 首聊解锁在同一因果链中仍可明确区分，且不产生免费认领或持续补贴含义。

### 4.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| AS-1 / AS-3 | gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-011 / PRD-WORLD_RUNTIME-001 | 报价、确认、持有、结束或回收的组合证据 | test_tier_required |
| AS-2 | agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-011 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | 当前玩家绑定/所有权与共享世界 Agent 可读边界 | test_tier_required |
| AS-4 | producer_system_designer / gameplay_designer / runtime_engineer / qa_engineer | PRD-GAME-011 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | 非免费、不可旁路和反滥用的权威与玩家可读证据 | test_tier_required |
| AS-5 | producer_system_designer / qa_engineer | PRD-GAME-011 / PRD-TESTING-003 | 同候选跨域组合审计 | test_tier_full |
| AS-6 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-011 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | 首个认领承诺包、持续成本、失去/恢复、替代决策与两类启动支持边界的组合证据 | test_tier_required |

## 5. Non-Goals

- 不冻结费用、倍率、宽限期、容量或回收参数。
- 不定义账本 bucket、资金 provenance、管理员权限、签名阈值、CLI 或运营 runbook。
- 不承诺免费 Agent、可转移启动财富、无限补贴、默认多 Agent 囤积或自动认领。
- 不以本文或历史任务状态声明当前 preview、可玩性或公开发行已经通过。
