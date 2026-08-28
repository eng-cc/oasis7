# 首局与持续游玩

## 文档身份

- 所属产品模块：世界规则与核心玩法
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/game/prd.md`](../../game/prd.md)、[`gameplay-top-level-design.prd.md`](../../game/gameplay/gameplay-top-level-design.prd.md)

本文是长期产品分册，承载首局微循环、后引导承接与首次持续能力的玩家承诺。它不冻结 UI 字段、tick、数值阈值、任务状态或实现方案。

## 1. 产品目标

玩家从第一次发出有效意图开始，就能持续回答五个问题：我正在追求什么、系统是否接受、世界发生了什么、为什么被阻塞、下一步怎样继续。首局结束不是体验终点，而是进入可恢复、有阶段成果且能展开中循环选择的持续游玩链路。阶段成果不是世界的通关条件，而是具有完成边界、可归因后果和下一阶段方向的有限进展。

首局信任与首次持续能力是两个相邻但独立的产品结果。前者证明玩家能够可靠地控制、理解并愿意继续当前体验；后者证明玩家已经获得能持续运转、经受阻塞并展开新选择的世界能力。世界仍在推进、进度数值变化或单次动作成功，均不能单独证明玩家被吸引而愿意继续，也不能替代其中任一结果的组合证据。

## 2. 首局微循环

每个受支持的玩家意图都必须形成可读闭环：

`选择目标或行动 -> 接受或拒绝 -> 推进或阻塞 -> 可读后果 -> 下一决策或恢复`

- 接受、执行、阻塞、改道、无进展完成和有效进展必须可区分。
- 显式推进请求的排队或接受不等于观察到完成；本次完成窗口内无进展必须保持可读、可关联且可继续决策，不能被包装为成功或永久失败。
- 被接受的首局控制不保证立即产生 Agent 行动或世界事件；等待、自动开始、背景推进、重新连接或界面计数都不能替代一次可归因的权威后果。没有观察到进展时，玩家仍应获得当前可信状态和可执行下一步。
- 世界变化必须回到当前主意图的成本、进度与后果，不能要求玩家从原始日志猜测。
- 当前动作不可执行时，系统必须给出原因以及等待、修复、改道或重新定目标中的可行下一步。
- 当前可执行动作与主要 blocker 必须进入玩家入口；内部 snapshot 存在字段不能替代玩家可用证据。
- Viewer 与 pure API 消费同一权威事实，但需要分别证明该闭环在各自入口可用。
- 玩家在提交控制意图前必须能发现当前支持的动作及必要输入；提交后必须能区分接受或拒绝、系统实际理解的动作、失败原因和建议下一步，不能靠盲试协议或原始日志确认系统是否听懂。

### 2.1 状态感知的“现在做什么”动作集

每个正式玩家入口都必须从同一权威状态导出一个规范的、状态感知的“现在做什么”动作集；这里的入口是 Viewer 与 pure API，二者不能因入口不同而让玩家猜测不同的首步。该动作集是玩家决策承诺，不规定字段、接口形状、UI 布局或命名。

- 每个入口在当前状态至少给出一个可立即执行的有效动作，或明确说明当前没有推进动作并给出能恢复决策权的有效动作；“查看状态”或继续等待本身不算满足这一要求，除非它就是当前唯一有意义的恢复决策。
- 对玩家此刻可能合理尝试、但尚不可执行的动作，入口必须同时给出不可用原因，以及对应的解锁条件或恢复路径；不得把不可用动作静默隐藏后要求玩家自行发现前置条件。
- 原因应关联当前目标和世界约束，并把解锁/恢复表述为可选的下一步：例如完成前置能力、补足可恢复的约束、等待可预期的世界恢复、改道至替代目标，或在无安全路径时重新定目标。它不承诺这些路径必然成功或冻结其成本、时机与实现。
- cold start 必须优先给出能建立第一条有效意图链的动作；进行中的目标必须优先给出推进、确认或有价值的改道动作；重连/续玩必须恢复当前目标、blocker 和相应的有效动作；空快照或被阻塞快照必须把恢复、替代或重新定目标作为有效动作，而不能只呈现空动作列表或技术错误。
- 一个动作被解释为暂不可用，不得伪装成已接受、已完成或永久失败；一旦状态变化，入口必须更新有效动作、不可用原因和解锁/恢复路径，使玩家可以重新作出选择。

### 2.1.1 状态置信度与多重 blocker 的动作优先级

动作集必须先通过状态置信度闸门，再进行 blocker 仲裁。这里的置信度是产品语义，不规定快照字段、时钟或实现方式：

- 只有在入口能够取得一份相互一致、仍适用于当前目标的权威状态时，任何会改变资源、权限、路线承诺、设施状态或他人权利的动作才可标记为有效。状态缺失、明显陈旧、来源冲突或无法判断适用范围时，入口必须收窄为取得当前权威状态、重新进入、等待明确复核，或安全停止/重新定目标中的适用恢复动作；不得根据缓存、推断或 Agent 计划猜测一个可执行的世界动作。
- 状态置信度恢复前，入口仍须保留玩家的决策权：至少给出一个真实的恢复或安全停止动作，并说明当前未知/冲突为何阻止原目标。单纯展示“查看状态”、无限等待或空动作列表不构成恢复，除非它就是当前唯一有意义且有边界的复核选择。
- 在状态一致时，多个 blocker 按以下稳定优先级仲裁当前主要 blocker：**安全/权利/授权保护**高于**不可逆损失、资源扣减或锁定**，高于**当前目标的可恢复前置条件**，高于**可选优化或补充信息**。高优先级 blocker 必须先决定主动作；较低优先级 blocker 只有在会改变当前选择、损失、锁定或恢复路径时才同时呈现，不能用次要信息遮蔽高后果约束。
- 同一优先级的多个安全路径不得被静默合并或随机择一：入口可以给出一个有理由的主推荐，但必须保留其他仍适用的可比较路径；如果没有可解释的安全推荐，则明确返回重新定目标或安全停止。推荐排序不能制造新的资格、优先权或世界效果。
- 动作集生成后到提交前若状态、权限或 blocker 发生变化，提交必须按当前权威状态重新判断；旧动作只能被明确拒绝、重新评估或转为恢复/改道提示，不得沿用旧成本、旧资格、旧风险，也不得静默切换到另一个动作。Viewer 与 pure API 可以有不同表现，但这套状态置信度、主 blocker 和重新判断语义必须一致。

专业 gameplay 合同负责把上述决策语义映射到真实动作能力、状态边界与验证样例；runtime 与 viewer/API 专业域分别负责实现和入口证据。本分册不新增 runtime 字段或规定表现层结构。

### 2.2 目标清晰度与首屏优先级

- 首局主目标必须同时说明玩家要采取的动作、怎样算完成，以及玩家可理解的时间或阶段预期；不能只展示描述性主题、内部状态名或没有完成边界的方向。
- 玩家入口优先呈现一个当前主目标。次要目标可以折叠或延后，但必须能被找回，且不能与主目标争夺首屏注意力或给出冲突指令。
- 当前目标的剩余条件、主要 blocker 与恢复动作必须随权威进度更新；世界仍在运行不能替代“玩家知道自己是否推进”的证据。
- 当系统推荐首个采集、探索或工业目标时，玩家需要在行动前读懂推荐对象的预期价值、可达性或进入成本，以及它与首次持续能力的关系；不能只以“最近”或隐藏排序作为理由。
- 推荐首个资源目标时，上述理由可以包含粗粒度材质倾向，但必须明确它只是决策提示，不承诺精确产出、稀有掉落、固定收益或必然完成首次持续能力。
- 首局完成时，体验应回顾已经形成的能力或世界后果，并把主 CTA 交给后引导阶段；一次性庆祝、静态总结或继续观察不能代替下一阶段承接。
- 系统可在首局至首次持续能力期间提供预设目标作为必要引导脊柱；推荐必须来自当前世界状态，并在玩家形成持续能力后降为可选模板，而不是固定职业、强制阵营或无限任务清单。

### 2.2.1 首个工业闭环 walkthrough

首局至首次持续能力至少要能把一条代表性工业目标讲成同一条可追溯因果链：

`工厂就绪 -> 比较配方 -> 获取/精炼原料 -> 物流抵达 -> 多输入齐套 -> 排程 -> 生产 receipt -> 交付 receipt`

这是一条玩家可执行的引导脊柱，不是把内部步骤逐项变成任务清单。每一步都必须让玩家知道当前要做的事、什么会让它停下、哪些投入已被保留或消耗，以及完成后下一步是什么；同一事实可由 Viewer 或 pure API 以各自入口表达，但不能改变动作、阻塞或完成边界。

| 引导节点 | 玩家可做的决定与完成边界 | 必须区分的主阻塞、反馈与恢复 |
| --- | --- | --- |
| 工厂就绪 | 检查一座当前可用的工厂，并选择将其作为本次工业目标；“看到了工厂”不算就绪。就绪只表示该工厂可接受本目标的后续比较，不表示已经占用产能或开始生产。 | 工厂能力、权限或必要条件未确认时，明确说明缺失的类别，提供补足前置、改选可用工厂或重新定目标；不得把已接受的目标写成已开工。 |
| 比较配方 | 比较当前可用配方的投入、产出用途、适用工厂、原料来源与主要运输风险，选择一个配方；预览/比较不产生世界效果，也不锁定库存。 | 配方不适用、资格/规格证据不足或当前来源不可用时，保留比较结果并给出换配方、补证据或先处理原料的路径；不能用材料同名或缓存建议推断适用。 |
| 获取/精炼原料 | 为选定配方形成独立的原料准备动作，比较直接获取与先精炼/替代来源的取舍；只有真实原料结果可供后续使用时才算准备完成。 | 原料不足、来源不可达、精炼未完成或精炼结果不满足适用性时，分别指出是供给、可达性还是规格问题；玩家可等待恢复、改源、改配方或放弃目标，不能静默裁剪投入。 |
| 物流抵达 | 选择并提交把已准备的原料送到工厂的路径；“在途”只表示运输义务存在，只有在目标账本/工厂侧实际到达并结算后才算抵达。 | 路线、边吞吐、在途损耗、目的地容量或时限造成的阻塞必须分开可读；玩家只能比较适用专业合同当前支持的等待、改道、减少承诺或取消尚未生效运输等路径，已发生的在途结果不能伪装为未发生或自动退款。 |
| 多输入齐套 | 在同一配方批次中确认所有独立输入均已到达且适用，再让本次生产进入可执行状态；先到的输入可保持在有界等待/保留状态，但不单独算生产进度。 | 缺失、冲突、过期或不适用的 parent input 是齐套阻塞，不得按到达顺序猜测或先消费一部分来冒充齐套。玩家可补齐缺口或等待未决输入；释放未消费保留、重新排配方等处置只有在适用专业合同支持时才可展示，后到输入只重评尚未满足的部分。 |
| 排程 | 在输入齐套后决定何时让本次配方进入工厂执行；排程被接受不等于已获得执行容量，也不等于生产开始。 | 工厂/阶段容量、电力、窗口或竞争导致的排程阻塞必须说明是尚未开始、暂缓还是拒绝，并显示保留与机会成本；玩家只能比较适用专业合同当前支持的等待释放、重排优先级、改配方/工厂或结束本次意图等路径。 |
| 生产 receipt | 只有配方周期实际执行并按该配方的产物策略结算后，才算“生产完成”；这证明产物在工厂或规定的中间缓冲中形成，不证明已经交付、可交易或已满足目的地用途。 | 生产阶段停机、产物分支无法接收或执行窗口失效时，明确保留的输入、已发生的产出与未决义务；只提供适用专业合同支持的恢复、暂停、改道、重排或终止，不能只显示排程成功。 |
| 交付 receipt | 产物通过运输并进入目标终端/目的地的准入或 buffer 仍是待交付中间状态；只有取得独立的交付/终端结算 receipt，才算“交付完成”。该 receipt 必须能回指本次生产结果和实际运输后果，且只有交付完成后才可把目的地用途作为本链路后果展示。 | 终端容量、目的地资格/需求失效、路线迟到或结算未确认时，生产完成保持生产完成，交付保持未完成；玩家只能比较适用专业合同当前支持的等待、改道、本地用途或结束交付意图等路径，不得把准入、buffer、在途、缓存或预览当作交付。 |

本 walkthrough 的反馈至少要把以下四种边界说清楚：**已接受但未开始**、**正在执行/在途**、**已生产但未交付**、**已交付并产生目的地后果**。其中，生产 receipt 与交付 receipt 是两个独立完成窗口；重复提交、重连、重排或回放不能复制任一世界效果，也不能把前一个 receipt 代替后一个 receipt。若链路在任一节点停下，玩家看到的主 blocker 应是当前最早且可行动的未满足条件，后续影响可作为次要后果呈现。

首局验收使用 `test_tier_required`：至少覆盖一条正向 walkthrough、每类节点一个可恢复阻塞、输入到达顺序变化下的多输入齐套、生产完成但交付未完成，以及重连/重复提交不复制 receipt。`test_tier_full` 再覆盖跨阶段窗口、容量争用、在途损耗、终端失败、恢复/换线与 Viewer/pure API 的事实一致性；这些验证由对应专业域执行，本分册只冻结玩家可理解的因果边界。

### 2.2.2 Starter Industrial Feasibility Gate

在系统把预设工业目标作为首局引导脊柱或当前主推荐交给玩家前，必须先运行产品根 PRD 定义的 `Starter Industrial Feasibility Gate`。本分册把它收敛为一张玩家可读的 feasibility card：闸门顶层结果只有 `candidate_available` 或 `no_safe_starter_chain`，不把内部 job、ledger、target contract 或旧缓存当成可行性证明；闸门本身不创建目标、不锁定库存/容量、不扣资源、不排程，也不保证产量、价格、ETA 或长期稳定。

`candidate_available` 只表示同一份 fresh authority snapshot 已证明一条代表性 starter chain 能到达其 profile 声明的完成边界。卡片至少说明稳定的 starter-chain/candidate identity（或等价的不可变上下文摘要）、绑定的 authority snapshot/version、当前工厂/地点、可用 recipe candidate、原料来源或精炼结果、有效物流路径、电力与输出/终端前置、主要风险、completion boundary、首个可验证成果、即时收益、`progression_effect`、`next_action`、`next_recheck` 与完成后打开的下一 beat；identity 必须覆盖改变产出因果的工厂、配方、输入来源、物流与完成边界。如果有多个候选，不得静默选定，推荐必须能说明来自当前事实的理由。只有已被专业 evidence 证明的 current path 才能通过；多输入 join、多阶段 root/window、output bundle、terminal settlement 或 maintenance 等 target-only 能力不能借 walkthrough 文本直接当作当前前置。首个可验证成果只能证明该链真实产生了声明的第一个世界结果，不能提前发放稳定/交付成长；即时收益必须是本次选择保住的能力、用途或下一选择，而不是后台完成或免费资源。

`no_safe_starter_chain` 不是永久失败。卡片必须指出最早且可行动的 blocker、证据分类（`current-evidence-backed`、`target-contract` 或 `unknown/not_tracked`）、已保留/已消费的价值、真实可用的补料、补证、等待、修复、改道、改候选或重新定目标路径，以及下一次复查边界；该结果不产生工业成长奖励、`progression_effect` 或下一 beat，只有恢复后从 fresh authority snapshot 重新通过闸门，才能再次进入目标。没有安全恢复时，系统应安全停止并交回其他当前可达目标；不得无限等待、自动补料、后台改道、发放免费输入，或把 target/unknown 填成“可达”。

闸门只负责接受前的可行性判断。玩家确认后仍须经过既有配方、排程、生产、交付与 receipt 边界；提交前工厂、配方、输入、路径、容量、电力或终端事实发生变化，或 starter-chain/candidate identity 绑定的 authority snapshot/version 失效时，必须重新判断或无副作用拒绝，不能保留旧 identity、静默切换候选或沿旧 identity 发放 `progression_effect`。`production-only` 的 starter 目标只能在匹配 production receipt 与稳定条件成立后完成生产目标，仍标记 `undelivered`；声明 terminal-admission/delivery 的目标必须等匹配 delivery/terminal settlement receipt，不能把生产、buffer 或准入当成交付。

闸门的 current/target 切线是玩家承诺的证据边界，不是新的 runtime 状态：`current-evidence-backed` 可以进入候选但提交仍须 fresh revalidation；`target-contract` 只能作为未来能力或复查方向；`unknown/not_tracked` 必须进入 `no_safe_starter_chain`，并保留未知原因。相同 authority snapshot/version 应得到相同结果；重连、重复请求、Agent retry、snapshot restore 与 replay 只能重读同一 feasibility/receipt 结果，不复制资源效果、目标完成或奖励。在 fresh composite runtime + QA evidence 证明 Gate 与 starter chain 之前，`test_tier_required` 与 `test_tier_full` 只是验收目标，不是当前 pass；任何 surface 不得宣称 Gate/current starter chain 已实现或默认可用，缺证据必须返回 `no_safe_starter_chain`。

本闸门的 `test_tier_required` 至少覆盖一条正向 starter chain、稳定 identity 与 authority snapshot/version 绑定、首个可验证成果/即时收益/`progression_effect`/下一 beat、工厂/配方/原料/物流/电力/输出各类 blocker、target-only/unknown fail-closed、报价后事实漂移、production-only 与 terminal-admission 的不同完成边界，以及重复/重连/replay 无副作用；`test_tier_full` 再覆盖多候选争用、跨窗口/多阶段链、持久化恢复和 Viewer/pure API 对结果、blocker、下一步与复查点的同义表达。该卡片不新增配方、数值、runtime schema、任务树、自动补给/改道或 UI 布局。

### 2.3 早期 quote/preview 的信息仲裁

首局与早期持续游玩中的既有 quote、preview 与推荐，应优先帮助玩家完成一个当前主要决策，并突出一个会改变该决策的主导 blocker 或成本。可以延后与当前选择无关、且可恢复的补充细节，但延后必须保留可回看的路径和时机；它不能把复杂性伪装成没有代价。

当任何细节会改变损失、锁定、权威移交、不可逆行动或恢复路径是否可用时，必须立即提升为当前决策的显式信息，不能被仲裁为次要或延后内容。仲裁只可重排和解释同一权威事实，不能省略权威成本、改变动作语义、把风险降格，或把建议写成已发生的后果。本规则不规定 UI 层级、字段、数值或实现方式。

## 3. 首局后的阶段承接

首次行动闭环完成后，玩家必须进入正式的后引导阶段，而不是只看到一次性总结或回到无目标观察态。

- 系统提供一个可达的主目标，并说明当前进度、主要阻塞与建议下一步。
- 正式入口默认把这个主目标作为唯一当前焦点，并优先给出“继续推荐路线”的低负担路径；其他目标保持可找回但不争夺当前决策。
- 默认承接应优先帮助玩家形成持续能力，例如稳定生产、恢复被阻塞的能力或完成首次有效协作；不得直接抛出与当前世界状态脱节的宏大目标。
- 玩家可以自由探索或暂时收起目标，但必须能重新聚焦；重连或回流后也能恢复当前目标、阻塞和下一步。
- 主目标不可达时，体验切换到保全、恢复或替代路径，不能只要求继续等待。
- 玩家只在阶段成果后从 2 至 3 个实质不同方向中选择，或主动要求换向；系统不得把目标作用域、canonical 转译、资源/权限校验、共同治理或审计的内部步骤逐项暴露为常规决策。它们只有在会改变当前目标的成本、锁定、恢复或共同承诺时，才应以简短原因和可执行替代路径出现。
- 首局信任只要求玩家能够可靠理解并继续当前链路；首次持续能力及其后的路线选择应在后续游玩中得到独立证明，不能把两者混成同一结论。

### 3.1 首次分支承诺

首次持续能力形成后，后引导至少应在当前世界状态允许时提供下列 2 至 3 类规范分支中的可达选项；类别是玩家可理解的承诺，不是固定职业、功能清单或强制顺序：

| 分支类别 | 即时收益 | 后续两个 beat 必须发生的实质变化 | 主要约束、风险或锁定 | 下次会话 hook |
| --- | --- | --- | --- | --- |
| 扩张 | 将已证明的能力转为更大的产出、覆盖或选择空间。 | 先要为扩张投入或排除当前瓶颈；随后要处理新吞吐、供给或维护压力，而非重复原有循环。 | 资源被占用、旧目标延后，或暴露新的稳定性压力。 | 回来可检查扩张是否兑现，并决定继续扩张、先稳住，还是改道。 |
| 稳定/恢复 | 守住已获得的能力，移除当前阻塞或降低已知失败风险。 | 先完成恢复、保全或替代供给；随后验证该能力能持续运转并重新开放被阻塞的选择。 | 放弃一段即时扩张机会，且恢复可能保留部分约束。 | 回来可确认稳定结果，并选择利用被保住的能力还是处理残留风险。 |
| 专业化/服务 | 将当前能力转成对本地目标、需求或协作有明确用途的贡献。 | 先选择能服务当前世界问题的用途；随后交付、验证或调整该用途，而非只累积库存。 | 对特定需求、地点或协作机会形成机会成本；不得默认要求加入 major power。 | 回来可追踪该贡献的后果，并选择深化、改换用途或保持独立。 |

每个实际推荐必须让玩家在选择前读懂本局的即时收益、与其他分支不同的后续两个 beat、主要约束/风险/锁定及下次会话的第一件可做之事。只给“扩张”“修复”或“协作”等标签，或让各路线在两个 beat 内回到同一个循环，不构成分支承诺。

可回退的路线还必须说明回退仍有效的窗口、回退的主要代价，以及回退后保留与失去的价值；不可回退或尚无安全回退时也必须明确这一点。上述承诺不冻结成本、数值、世界状态、runtime 字段或 UI 表现；具体可用路线仍取决于权威世界状态。

### 3.2 目标换向与在途意图边界

主动换向改变玩家接下来要追求的主目标，但不把旧目标的不同处理阶段混成一个结果：

- **预览 / 推荐**只说明候选目标、预计成本、主要 blocker 与下一步，不产生世界效果；换向可以放弃它，不能把预览当作已提交或已占用资源。
- **已接受但尚未生效的请求**仍按其专业合同判断可继续、明确取消、过期或重新评估。换向、重连、刷新或普通重试不得把旧请求自动迁移到新目标、复制其资格/优先级或产生第二次世界效果；若需要新目标下的行动，玩家或 Agent 必须形成独立的新请求。
- **已提交的世界结果**不因换向被追溯取消、伪装成未发生或再次执行；后续纠错、救济或持续义务只能沿适用专业合同处理。换向前已存在的义务、风险和可归因后果仍须可读。

换向确认后，正式入口只将新目标作为当前主焦点，同时保留旧目标的已生效结果、未决义务/风险和可执行的取消、等待、恢复或重新规划下一步；不得把两个目标同时表现为已接受的当前主线，也不得以切换成功代替旧请求的权威结果。具体请求身份、去重、取消、过期、重评与 receipt 由 runtime、Agent 和 Viewer 专业权威定义。

## 4. 首次持续能力与中循环展开

首次持续能力不是完成一次重复动作，而是玩家建立了一项能够继续运转、修复并产生新选择的世界能力。

- 玩家能读懂投入、产出、当前用途、维护或恢复方式以及下一步价值。
- 首个阶段成果必须在合理的早期游玩窗口内可达；具体时长和数值由专业域与当前验证计划维护。
- 能力证明必须覆盖一次可理解的阻塞及其可恢复处理，表明该能力能继续运转或在受限后恢复，而非只证明一次无阻碍的动作完成。
- 达成后至少展开一个中循环方向，例如生产扩张、区域服务、治理影响或协作保障。
- 每个推荐方向都要说明即时收益、后续体验变化、主要风险或约束以及下次会话的继续理由；方向标签本身不能代替选择后果。可以回退的选择还必须让玩家理解何时仍可回退、回退的主要代价，以及哪些收益会保留或失去，不能用笼统的“可回退”掩盖取舍。
- 首局控制可信度、继续游玩的动机和首次持续能力是相邻但不同的判断，不得用其中一项的通过代签其他项。
- 阶段成果必须使玩家读懂已经形成的能力或世界后果、它为什么不是单次动作成功，以及下一次可以继续、选择或换向的理由；不得将“继续点击确认”或逐动作审核包装为成长。

## 5. 失败与恢复

失败必须保留玩家的下一次决策权：

- 阻塞原因使用玩家可理解的资源、能源、物流、治理、危机、协作或权限类别。
- 恢复建议说明会保留什么、损失什么、何时复查，以及何时应该放弃当前路线。
- 当资源稀缺、采空或恢复节奏阻塞当前目标时，玩家必须能比较等待恢复、转移到其他目标或区域、改走替代材料路线，并读懂这些选择对当前目标的帮助以及等待或改道的主要成本；后台恢复事件不能替代这次选择。
- 没有安全恢复动作时，明确结束当前意图并返回重新定目标或升级约束的决策面。
- 不以无限等待、重复同一操作、隐藏自动改道或泛化错误文案伪装持续游玩。

## 6. 组合验收

- FS-1：首局样例可端到端证明目标、行动、接受或拒绝、推进或阻塞、世界后果与下一步处于同一因果链。
- FS-2：无进展样例能给出可理解的 blocker 和至少一个可执行恢复或重排路径；没有安全路径时能明确返回新的决策面。
- FS-3：首次行动闭环后存在可达的后引导主目标，并能在离开和重连后恢复目标、阻塞及下一步。
- FS-4：首次持续能力样例能证明投入、产出、用途、维护或恢复与后续价值，而不是只证明一次动作成功。
- FS-5：阶段成果后的方向选择至少说明收益、体验变化、风险或约束和继续游玩的 hook。
- FS-6：Viewer 与 pure API 分别提供自身入口证据，且二者对权威事实、动作能力和主要因果保持一致。
- FS-7：首局入口样例能证明主目标包含动作、完成条件和时间或阶段预期，次要目标不干扰当前决策；推荐首个资源目标时可解释其粗粒度价值、可达性与首次持续能力关联而不承诺产出，资源阻塞样例能比较等待、转移与替代路线的主要成本，结束后能进入后引导主目标。
- FS-8：组合证据分别证明首局信任、继续游玩的动机与首次持续能力；持续能力样例包含可恢复阻塞和后续路线，路线样例说明收益、体验变化、风险或约束、回访理由及适用时的可逆取舍。确定性或模拟性证据可以验证规则和体验结构，但不能单独替代真实留存或受控 provider readiness 的专业验证。
- FS-9：Viewer 与 pure API 分别在 cold start、进行中、重连/续玩以及空快照或被阻塞快照证明同一状态感知的“现在做什么”语义：有效动作，合理但不可用动作的原因，以及解锁或恢复路径；空/阻塞快照必须仍保留可作出的恢复、替代或重新定目标决策。
- FS-10：首次持续能力后的样例提供 2 至 3 个当前可达的分支类别，并为每项证明即时收益、实质不同的后续两个 beat、约束/风险/锁定与下次会话第一动作；可回退项还证明窗口、代价、保留价值与失去价值。
- FS-11：代表性早期 quote/preview 样例围绕一个当前主要决策突出一个主导 blocker 或成本，并保留可恢复细节的回看路径；任何损失、锁定、权威移交、不可逆行动或恢复可用性变化均被提升，不会因信息仲裁而遗漏或改变语义。
- FS-12：首局至首次持续能力的样例以预设引导脊柱建立一个当前主目标和可执行“继续”路径；达成阶段成果后只在 2 至 3 个实质不同方向或玩家主动换向时请求选择，后台作用域/转译/校验/治理/审计只在实质影响当前选择时提供原因和替代路径。
- FS-13：代表性主动换向样例区分预览、已接受但尚未生效的请求与已提交的世界结果；换向、重连、并发或重试不会追溯取消已提交结果、自动迁移旧请求或产生第二次 receipt。新目标独立形成，玩家能读到旧目标的已生效结果、未决义务/风险及取消、等待、恢复或重新规划下一步，且正式入口不会把旧、新目标同时表达为当前主线。
- FS-14：空、陈旧或冲突状态，以及同时存在多个 blocker 的代表性样例，证明 Viewer 与 pure API 采用相同的状态置信度闸门和主要 blocker 优先级：状态未确认时不提供会改变世界的猜测动作，至少保留真实的复核、恢复、安全停止或重新定目标路径；状态一致时先呈现安全/权利/授权与不可逆后果，再处理可恢复前置和可选信息；同级安全路径可比较且不会被静默合并。状态在展示后变化时，旧动作必须按当前状态重新判断，不得沿用旧资格/成本、静默改道或产生第二次世界效果。
- FS-15：代表性首局工业 walkthrough 在展示为当前主推荐前，必须先通过 `Starter Industrial Feasibility Gate`，并沿 `工厂就绪 -> 配方比较 -> 原料获取/精炼 -> 物流抵达 -> 多输入齐套 -> 排程 -> 生产 receipt -> 交付 receipt` 逐节点证明玩家动作、完成边界、主 blocker、反馈与恢复；闸门只返回 `candidate_available` 或 `no_safe_starter_chain`，后者必须保留 current/target/unknown 证据分类、可行动 blocker、下一动作和复查边界，不得发放免费输入或静默改道。walkthrough 至少区分 accepted-unstarted、active/in-transit、produced-but-not-delivered 与 delivered/terminal-settled，证明生产完成不解锁交付用途、交付完成才产生目的地后果，且重连/重复提交/回放不复制任一 receipt。正向、可恢复阻塞、arrival reorder、生产成功而交付失败/未确认均需有 `test_tier_required` 证据，跨窗口/争用/损耗/终端故障与两入口一致性进入 `test_tier_full`。

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
| FS-8 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-012 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 信任、继续动机与持续能力的分离证据；能力恢复、路线取舍与专业留存/提供方 readiness 验证的组合审计 | test_tier_full |
| FS-9 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-004 / PRD-GAME-008 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 两个正式入口在 cold start、进行中、重连及空/阻塞快照的有效动作、不可用原因和解锁/恢复路径 parity 证据 | test_tier_required |
| FS-10 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-012 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 当前可达分支的即时收益、两个不同后续 beat、约束/锁定、回访第一动作及适用时的回退取舍证据 | test_tier_required |
| FS-11 | gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-012 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 首局 quote/preview 的主要决策、主导成本/阻塞、延后信息回看与高后果信息提升证据 | test_tier_required |
| FS-12 | producer_system_designer / gameplay_designer / agent_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-004 / PRD-GAME-007 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 单一主目标、预设引导脊柱、继续/分支/换向及仅在实质相关时出现的后台护栏 S6 组合证据 | test_tier_required |
| FS-13 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-GAME-004 / PRD-GAME-014 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 换向、重连、并发与重试下的预览/已接受或待决/已提交分类、旧请求不自动迁移、已提交结果不追溯取消、单次 receipt、新目标独立形成，以及旧义务/风险与下一步的玩家可读性组合证据 | test_tier_full |
| FS-14 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-004 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_SIMULATOR-001 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 空/陈旧/冲突状态与多 blocker 的状态置信度闸门、稳定优先级、同级路径比较、展示后重新判断，以及无猜测动作/无静默改道/无第二次效果的 Viewer + pure API 组合证据 | test_tier_full |
| FS-15 | producer_system_designer / gameplay_designer / runtime_engineer / viewer_engineer / qa_engineer | PRD-GAME-007 / PRD-GAME-012 / PRD-WORLD_RUNTIME-019 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/game/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | 首局工业 walkthrough 的逐节点动作/边界/阻塞/恢复、齐套 arrival-order、accepted 与生产/交付分离、生产成功但交付未完成、重复/重连/回放单次 receipt，以及 Viewer/pure API 事实一致性证据 | test_tier_required + test_tier_full |

具体字段矩阵、测试命令与历史 verdict 不复制到本分册。

## 7. Non-Goals

- 不定义固定 UI 布局、提示文案、事件字段或计时阈值。
- 不要求完整动态任务树或由 LLM 自由生成任务。
- 不把玩家锁进线性教程，也不把自由探索解释为无目标漂浮。
- 不要求玩家逐动作确认 canonical 转译或审核后台校验；预览、报价和治理细节只在它们实质改变当前选择时服务于该选择。
- 不以历史任务完成态或旧版本样本声明当前体验已经通过。
- 本 walkthrough 不新增配方、产能、物流损耗、运输时限、队列、库存或价格数值，不冻结 runtime 字段、事件/receipt schema、UI 布局或具体任务文案；这些仍由 `doc/game`、M4/runtime、Viewer 与 QA 专业权威承接。
- 本 walkthrough 不承诺每座工厂、每个配方或每条路线都可在首局完成；它要求当前被选中的代表性链路能解释可达性、阻塞与恢复，并在不可达时返回安全的替代或重新定目标路径。
