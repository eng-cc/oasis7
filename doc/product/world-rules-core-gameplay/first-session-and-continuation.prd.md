# 首局与持续游玩

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`gameplay-top-level-design.prd.md`](../../game/gameplay/gameplay-top-level-design.prd.md)

本文是长期产品分册，承载首局微循环、后引导承接与首次持续能力的玩家承诺。它不冻结 UI 字段、tick、数值阈值、任务状态或实现方案。

## 1. 产品目标

玩家从第一次发出有效意图开始，就能持续回答五个问题：我正在追求什么、系统是否接受、世界发生了什么、为什么被阻塞、下一步怎样继续。首局结束不是体验终点，而是进入可恢复、有阶段成果且能展开中循环选择的持续游玩链路。

## 2. 首局微循环

每个受支持的玩家意图都必须形成可读闭环：

`选择目标或行动 -> 接受或拒绝 -> 推进或阻塞 -> 可读后果 -> 下一决策或恢复`

- 接受、执行、阻塞、改道、无进展完成和有效进展必须可区分。
- 世界变化必须回到当前主意图的成本、进度与后果，不能要求玩家从原始日志猜测。
- 当前动作不可执行时，系统必须给出原因以及等待、修复、改道或重新定目标中的可行下一步。
- 当前可执行动作与主要 blocker 必须进入玩家入口；内部 snapshot 存在字段不能替代玩家可用证据。
- Viewer 与 pure API 消费同一权威事实，但需要分别证明该闭环在各自入口可用。

### 2.1 目标清晰度与首屏优先级

- 首局主目标必须同时说明玩家要采取的动作、怎样算完成，以及玩家可理解的时间或阶段预期；不能只展示描述性主题、内部状态名或没有完成边界的方向。
- 玩家入口优先呈现一个当前主目标。次要目标可以折叠或延后，但必须能被找回，且不能与主目标争夺首屏注意力或给出冲突指令。
- 当前目标的剩余条件、主要 blocker 与恢复动作必须随权威进度更新；世界仍在运行不能替代“玩家知道自己是否推进”的证据。
- 当系统推荐首个采集、探索或工业目标时，玩家需要在行动前读懂推荐对象的预期价值、可达性或进入成本，以及它与首次持续能力的关系；不能只以“最近”或隐藏排序作为理由。
- 首局完成时，体验应回顾已经形成的能力或世界后果，并把主 CTA 交给后引导阶段；一次性庆祝、静态总结或继续观察不能代替下一阶段承接。

## 3. 首局后的阶段承接

首次行动闭环完成后，玩家必须进入正式的后引导阶段，而不是只看到一次性总结或回到无目标观察态。

- 系统提供一个可达的主目标，并说明当前进度、主要阻塞与建议下一步。
- 默认承接应优先帮助玩家形成持续能力，例如稳定生产、恢复被阻塞的能力或完成首次有效协作；不得直接抛出与当前世界状态脱节的宏大目标。
- 玩家可以自由探索或暂时收起目标，但必须能重新聚焦；重连或回流后也能恢复当前目标、阻塞和下一步。
- 主目标不可达时，体验切换到保全、恢复或替代路径，不能只要求继续等待。

## 4. 首次持续能力与中循环展开

首次持续能力不是完成一次重复动作，而是玩家建立了一项能够继续运转、修复并产生新选择的世界能力。

- 玩家能读懂投入、产出、当前用途、维护或恢复方式以及下一步价值。
- 首个阶段成果必须在合理的早期游玩窗口内可达；具体时长和数值由专业域与当前验证计划维护。
- 达成后至少展开一个中循环方向，例如生产扩张、区域服务、治理影响或协作保障。
- 每个推荐方向都要说明即时收益、后续体验变化、主要风险或约束以及下次会话的继续理由；方向标签本身不能代替选择后果。
- 首局控制可信度、继续游玩的动机和首次持续能力是相邻但不同的判断，不得用其中一项的通过代签其他项。

## 5. 失败与恢复

失败必须保留玩家的下一次决策权：

- 阻塞原因使用玩家可理解的资源、能源、物流、治理、危机、协作或权限类别。
- 恢复建议说明会保留什么、损失什么、何时复查，以及何时应该放弃当前路线。
- 没有安全恢复动作时，明确结束当前意图并返回重新定目标或升级约束的决策面。
- 不以无限等待、重复同一操作、隐藏自动改道或泛化错误文案伪装持续游玩。

## 6. 组合验收

- FS-1：首局样例可端到端证明目标、行动、接受或拒绝、推进或阻塞、世界后果与下一步处于同一因果链。
- FS-2：无进展样例能给出可理解的 blocker 和至少一个可执行恢复或重排路径；没有安全路径时能明确返回新的决策面。
- FS-3：首次行动闭环后存在可达的后引导主目标，并能在离开和重连后恢复目标、阻塞及下一步。
- FS-4：首次持续能力样例能证明投入、产出、用途、维护或恢复与后续价值，而不是只证明一次动作成功。
- FS-5：阶段成果后的方向选择至少说明收益、体验变化、风险或约束和继续游玩的 hook。
- FS-6：Viewer 与 pure API 分别提供自身入口证据，且二者对权威事实、动作能力和主要因果保持一致。
- FS-7：首局入口样例能证明主目标包含动作、完成条件和时间或阶段预期，次要目标不干扰当前决策；推荐首个目标时可解释其价值、可达性与首次持续能力关联，结束后能进入后引导主目标。

### 6.1 验收追踪

| 成功标准 | 专业 owner | 专业域 PRD-ID | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- | --- |
| FS-1 | gameplay_designer / runtime_engineer / viewer_engineer | PRD-GAME-004 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | 首局目标、行动、接受/拒绝、推进/阻塞、权威后果和下一步的 S6 组合证据 | test_tier_required |
| FS-2 | gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer | PRD-GAME-004 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md` | blocker、恢复、重排及无安全路径时返回决策面的 S6 证据 | test_tier_required |
| FS-3 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | PostOnboarding 转换、目标恢复、重连续玩的 S6 与 playability 证据 | test_tier_required |
| FS-4 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-012 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 首次持续能力的投入、产出、用途、维护/恢复和后续价值组合证据 | test_tier_required |
| FS-5 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-012 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 分支收益、体验变化、风险/约束和继续游玩 hook 的 S6 证据 | test_tier_required |
| FS-6 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-008 / PRD-WORLD_SIMULATOR-039 / PRD-WORLD_SIMULATOR-041 / PRD-WORLD_SIMULATOR-046 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | Viewer 与 pure API 各自入口证据及权威事实、动作能力、主要因果 parity 对账 | test_tier_full |
| FS-7 | gameplay_designer / viewer_engineer / qa_engineer | PRD-GAME-004 / PRD-GAME-012 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 首局主目标结构、首屏优先级、推荐理由、阻塞恢复与 PostOnboarding 交接的 S6 入口证据 | test_tier_required |

具体字段矩阵、测试命令与历史 verdict 不复制到本分册。

## 7. Non-Goals

- 不定义固定 UI 布局、提示文案、事件字段或计时阈值。
- 不要求完整动态任务树或由 LLM 自由生成任务。
- 不把玩家锁进线性教程，也不把自由探索解释为无目标漂浮。
- 不以历史任务完成态或旧版本样本声明当前体验已经通过。
