# 间接控制下的玩家能动性与续接

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`PRD-GAME-014`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，承载玩家通过 Agent 间接推动世界时的能动性、因果可读性、干预与续接承诺。它补充 [`首局与持续游玩`](first-session-and-continuation.prd.md)，但不重复后者拥有的首局引导、首次持续能力与成长承接，也不冻结字段、状态枚举、UI/API、runtime/Agent 实现、测试步骤、任务状态或历史 verdict。

## 1. 产品承诺

玩家通过目标与 Agent 间接推动持续世界时，始终能理解自己正在追求什么、系统是否接受并如何推进、当前后果或阻塞为何发生，以及现在可以如何继续、改道或恢复；离开后返回仍可从这一决策链继续，而不是旁观不可解释的自动行为。

间接控制不是第一人称逐帧操控，也不是让 Agent 隐藏过程后替玩家做完一切。Agent 可以在世界规则、资源、权限和治理边界内自主执行，但玩家必须保有理解主要因果、修正方向和作出下一次决定的能力。

## 2. 玩家体验边界

- 每个受支持的玩家意图形成可读闭环：`选择目标或行动 -> 接受或拒绝 -> 推进或阻塞 -> 可读后果 -> 下一决策或恢复`。
- 玩家能够把当前世界变化归因到自己的主意图，读懂主要付出、进展或无进展、阻塞与下一步；世界仍在运行或存在原始日志不能代替这一结果。
- 当前路线不可行、被改道或不再值得等待时，玩家保有下一次决策权，能够理解原因并选择等待、修复、改道、重新聚焦，或安全结束当前意图后重新定目标。
- 玩家可以暂时探索或离开，但能够重新聚焦；重连或回流后能够恢复当前目标、主要阻塞、最近后果与可执行下一步，而不是回到无目标观察。
- 当 Agent 使用长期记忆影响当前决定时，玩家能够理解相关记忆的来源和作用，并能提交纠正或重排当前意图；纠正是否接受、影响范围及生效结果由 Agent 与 Viewer 专业合同拥有。
- Viewer 与 pure API 等正式玩家入口以同一权威世界事实支撑上述体验；入口布局、字段和实现机制可以不同，但不能制造不同的意图、因果或下一步真值。

## 3. 权威与组合关系

| 层级 | 本产品分册拥有 | 下层专业域拥有 |
| --- | --- | --- |
| 玩家价值 | 间接控制仍然保有可理解、可干预、可恢复的玩家能动性 | `game` 定义玩法规则、保证项、失败签名与专业验收 |
| 权威执行 | 玩家意图、世界后果和阻塞处于同一可解释因果链 | `world-runtime` 定义权威状态、校验、执行、回放与恢复合同 |
| 玩家入口与 Agent | 正式入口能够表达同一意图、因果、干预和续接结果 | `world-simulator`、Agent 与 Viewer 专业文档定义 API/UI、记忆和交互实现 |
| 验证 | 组合证据必须证明玩家能理解并继续决策 | `testing` 与 QA 权威拥有测试矩阵、命令、样本和当前 verdict |

`PRD-GAME-014` 的长期产品承诺由本文承载；其专业合同以 [`gameplay-indirect-control-agency-contract.prd.md`](../../game/gameplay/gameplay-indirect-control-agency-contract.prd.md) 为主入口，继续拥有玩法保证、失败判据与专业验证。IA-1 至 IA-6 只裁定组合体验，不能替代该专业合同；本分册不复制其中的字段矩阵、状态 taxonomy、bounded-response 规则、API parity、失败签名或任务证据。

## 4. 组合验收

- IA-1：代表性流程能够证明 `玩家意图 -> Agent 解释与执行 -> 权威规则校验 -> 世界后果 -> 可读因果 -> 下一决策或恢复` 形成同一条端到端链路。
- IA-2：正式玩家入口都能回答四个问题：玩家要求了什么、系统是否接受、为什么当前这样推进或阻塞、现在最有效的下一步是什么。
- IA-3：路线被阻塞、替换或改道时，玩家能够理解原因并使用至少一种有效的干预、重排、fallback 或恢复路径；没有安全路径时能够返回新的决策面。
- IA-4：离开和返回后，玩家能够从最近有效意图、主要阻塞、最近后果和下一步继续，而不依赖原始日志重建上下文。
- IA-5：记忆驱动的行动能够说明相关记忆为何影响当前决定，并提供可理解的纠正结果；未使用长期记忆的流程不因此被要求引入记忆系统。
- IA-6：产品层、game、runtime、Agent/Viewer 与 testing 的证据指向同一候选和权威事实；任一局部 green、文档建档或世界持续 tick 都不能单独证明本产品承诺通过。

### 4.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| IA-1 | gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 同一意图贯通 Agent、权威校验、世界后果、玩家因果与下一步的组合证据 | test_tier_full |
| IA-2 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | Viewer 与 pure API 各自入口的意图、接受、主因果和下一步对账 | test_tier_full |
| IA-3 | gameplay_designer / runtime_engineer / agent_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 阻塞、替换、改道、fallback、重排及无安全路径时返回决策面的证据 | test_tier_required |
| IA-4 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 重连或回流后的意图、阻塞、后果与下一步恢复证据 | test_tier_required |
| IA-5 | gameplay_designer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 记忆来源、当前用途、纠正与结果可读性证据 | test_tier_required |
| IA-6 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同候选跨域组合审计；产品文档、局部测试或世界 tick 不得代签 | test_tier_full |

## 5. Non-Goals

- 不把 oasis7 改成第一人称逐帧操控、逐块建造或以增加动作数量代替能动性。
- 不冻结 UI 布局、API 字段、状态枚举、tick、数值、runtime/Agent 实现、测试矩阵或任务状态。
- 不把本分册或 `PRD-GAME-014` 建档包装成留存、active-LLM readiness、QA gate 或公开发行已经通过。
- 不承诺复杂预测、完整分支模拟或完整记忆编辑系统。
