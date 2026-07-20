# 成熟世界成长与区域参与

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`gameplay-top-level-design.prd.md`](../../game/gameplay/gameplay-top-level-design.prd.md)

本文是长期产品分册，承载玩家完成首个持续能力后，在已有组织、治理和历史的成熟世界中继续形成独立价值的产品承诺。它不冻结状态字段、数值、Agent 决策顺序、界面结构、任务状态或当前放行结论。

## 1. 产品问题

世界持续发展以后，新玩家、小规模玩家和回流玩家不能只在“立即依附强组织”与“退化为旁观者”之间选择。产品必须让资源和影响力有限的玩家仍能靠自己的行动形成可读、可恢复且对区域有用的成长路线。

世界本身很活跃不代表玩家正在成长。有效路线必须让玩家回答：我采取了什么行动、世界因此发生了什么、我获得了哪种新选择或韧性，以及下次回来为什么仍值得继续。

## 2. 成熟世界成长主线

默认产品路径为：

`本地立足 -> 区域专业化贡献 -> 有限范围的区域影响`

### 2.1 本地立足

- 玩家先稳定一项小规模工业或服务能力，并完成一次对世界有可见后果的成果。
- 首个成果的“受保护”指失败影响范围有限、存在恢复路径且玩家贡献可见，不表示必然成功、永久免战、经济旁路或政治豁免。
- 该阶段不能要求玩家以加入 major power、接受强制赞助或进入全局治理作为唯一继续条件。

### 2.2 区域专业化贡献

- 玩家从重复维持转向满足具体区域需求的专业化贡献，例如恢复、转换、供应、维护或物流服务。
- 选择专业化前，玩家需要读懂第一项贡献服务什么需求、需要哪些主要投入、预计形成什么结果、带来哪类新能力，以及完成后下一次回来可继续什么。
- 专业化不能只是职业标签或同一产线的吞吐增长；它必须带来新的用途、恢复弹性、议价位置或区域选择。

### 2.3 有限范围的区域影响

- 持续的区域贡献可以形成局部优先级、机会、信任、可见度或协调能力。
- 这类影响必须低于全局治理权、联盟领导权或跨区域军政控制，不能把小玩家成长静默升级为 major-power 路线。
- 玩家可以自愿进入更大组织或更深治理，但产品不能把这种升级包装成成熟世界中唯一有效的成长答案。

## 3. 失败、恢复与独立性

局部停机、资源短缺、据点受压或路线失效后，体验应提供可比较的恢复方向：

- `repair`：保留当前路线并修复关键缺口。
- `rebuild`：更换位置或重新建立同类小规模能力。
- `pivot`：转向另一种区域专业化，让已有投入形成新的用途。

恢复选择必须说明主要时间或阶段成本、资源成本、可保留收益、风险和推荐理由。只有在独立恢复确实不可行时，才可以把外部赞助或强组织依赖标记为有原因的受迫路径；常态路线应保留不依赖 major power 的继续空间。

等待只有在存在明确触发条件、复查时机和预期变化时才是有效恢复。无期限等待、反复同一操作或隐藏自动改道不能伪装成持续游玩。

## 4. Anti-grind 与玩家影响判据

每个阶段成果至少需要证明以下一项发生了真实变化：

- 解锁新的可执行选择；
- 改善失败后的恢复弹性；
- 改善玩家的局部议价或协调位置；
- 增加对区域可识别的用途。

如果结果只有产量、库存或重复次数上升，而没有上述变化，应判定为 grind 风险，不能作为成熟世界成长成立的证据。

玩家影响证据必须形成同一条因果链：

`玩家行动 -> 可归因的世界变化 -> 新增能力或区域价值 -> 下一决策或回访理由`

环境事件、其他组织活动或 Agent 自主推进不能替代玩家自身的影响证据。

## 5. 生命周期与相邻分册边界

- [`首局与持续游玩`](first-session-and-continuation.prd.md)负责从首局到首次持续能力及最初的中循环选择。
- 本分册从首次持续能力之后开始，负责成熟世界中的独立成长、专业化、区域影响与失败恢复。
- Agent、设施、治理和世界基础设施可以支持该路线，但其字段、权限、执行与数值由对应专业域维护。
- 本路线不改变当前 early-retention 优先级，也不证明 preview、stage 或公开 claim envelope 已升级。

## 6. 组合验收

- MW-1：代表性成熟世界样例证明玩家不必立即依附 major power，也能完成一次可归因、可读且有后续价值的区域贡献。
- MW-2：首个区域专业化选择能说明本地需求、主要投入、预期结果、新增 leverage 和回访理由，而不是只展示角色标签。
- MW-3：阶段成果证明新增选择、恢复弹性、议价位置或区域用途；只增加吞吐或库存的样例不能通过。
- MW-4：失败样例允许比较 repair、rebuild 与 pivot；若只剩强组织依赖，必须说明独立路线为何不可行并保留后续决策权。
- MW-5：区域影响保持有限，不被误报为全局治理权、联盟领导权或跨区控制。
- MW-6：Viewer 与 pure API 分别提供玩家可读证据，runtime、Agent 和 gameplay 专业域对行动、后果、恢复与依赖边界保持一致。
- MW-7：`test_tier_required` 证明合同和可读性；`test_tier_full` 的 fresh mature-world 样例才能给出当前路线 verdict。历史完成态或文档迁移本身不能代替 fresh evidence。

### 6.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| MW-1 / MW-3 / MW-5 | gameplay_designer / qa_engineer | PRD-GAME-015 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/testing/prd.md` | mature-world player leverage 与 anti-grind 样例 | test_tier_full |
| MW-2 | gameplay_designer / agent_engineer / viewer_engineer | PRD-GAME-015 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-simulator/prd.md` | 专业化第一项贡献的玩家可读预览 | test_tier_required |
| MW-4 | gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer | PRD-GAME-015 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | failure 到 repair / rebuild / pivot 的组合证据 | test_tier_required |
| MW-6 / MW-7 | qa_engineer / runtime_engineer / viewer_engineer | PRD-TESTING-003 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/testing/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | Viewer、pure API 与权威状态对账及 fresh sample verdict | test_tier_full |

具体字段、状态转换、Agent 决策顺序、界面呈现和 pass/watch/block 证据由专业域文档与 GitHub task issue evidence 维护，不复制到本产品分册。

## 7. Non-Goals

- 不新增免费 claim、无限补贴、经济旁路或永久保护区。
- 不承诺完整职业树、固定专业化数值或全局影响力成长曲线。
- 不把区域专业化扩展为默认战争、联盟或全局治理主线。
- 不用历史任务完成态、旧样本或本次文档整理声称当前 mature-world 体验已经通过。
