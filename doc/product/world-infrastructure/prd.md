# 大世界基础设施 PRD

## 文档身份

- 产品模块：大世界基础设施
- 产品模块 slug：`world-infrastructure`
- 产品层唯一 PRD：`doc/product/world-infrastructure/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-002`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-08-13`
- 后继文档：`无`
- 下层专业域：[`doc/game/prd.md`](../../game/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文只承载产品承诺、范围 taxonomy、跨域 authority 与组合验收。专业域 PRD 继续拥有各自规则、实现契约、PRD-ID 和测试证据；任何下层专题都不得再自称“大世界基础设施”的并列产品总入口。

## 1. 产品承诺

大世界基础设施是 oasis7 的区块链/分布式系统与确定性世界运行时基础。它提供一个可验证的权威世界历史：共识最终性决定何时提交，网络和存储复制并提供 hash-bound 状态材料，确定性执行把已排序输入转成世界状态，恢复重建同一历史而非第二个世界。

它是下层 provider，而不是设施、市场、区域、frontier、组织治理或玩家循环的产品入口。世界规则与核心玩法、智能体与世界模拟、玩家入口与发行在此基础上组合各自产品语义；它们都不能借由本模块的技术能力直接扩张为权威写入权。

## 2. 范围

本模块只覆盖区块链/分布式底层和其上的确定性世界执行。它不拥有上层的游戏规则、Agent 行为或玩家入口，但这些消费者必须遵循其最终性、版本与 committed-state 边界。

### 活跃产品专题

- [`分布式共识与状态可用性`](distributed-consensus-and-state-availability.prd.md)：共识最终性、单一 canonical history、复制/状态同步、存储角色、网络暴露、bootstrap 与灾难恢复。
- [`确定性世界执行`](deterministic-world-execution.prd.md)：共识之上的版本化确定性执行、验证者重执行、消费者协议、升级 lane 与 finality 缺失时的 fail-closed 语义。

### 非权威迁移索引

- [`世界连续性与恢复（历史引用）`](world-continuity-governance-and-recovery.prd.md)：已退休的旧路径，保留仅为存量专业链接；不构成 active authority 或验收。
- [`区域 charter/tenure（待迁移）`](regional-charter-tenure-and-public-funding.prd.md)、[`工业/市场（待迁移）`](governed-industry-market-and-emergency-supply.prd.md) 与 [`普通治理（部分待迁移）`](global-governance-organization-continuity-and-constitutional-guardrails.prd.md) 都是 `superseded` 迁移债务：各页迁移头记录接收 owner、已吸收切片与删除条件，内容仍可供接收模块 owner 迁移，但这些路径不再构成本模块产品 authority、路线图、active topic 或验收。原 frontier 与区域能力/扩展迁移债务已由世界规则与核心玩法吸收，不再保留在本模块索引。

此前的区域设施、市场/工业、charter、frontier 与普通治理分册已从本模块退休：它们是上层 gameplay/world-rule 产品语义，不能再作为基础设施的 taxonomy 或验收门槛。对应专业域与其他产品模块继续拥有其规则、实现合同和现状证据；本次退休不宣称这些能力已迁移、实现或公开可用。

## 3. 权威与冲突处理

| 层次 | 本模块产品承诺 | 专业域权威 |
| --- | --- | --- |
| 分布式底层 | 验证者集合、最终性、唯一顺序、复制、可用性、存储与恢复共同守住一个权威 `world_id` 历史 | `doc/p2p/prd.md` 拥有共识、网络、节点、存储与运维技术合同 |
| 确定性执行 | 已最终化输入经版本化规则产生唯一世界结果；执行与共识具有独立语义边界 | `doc/world-runtime/prd.md` 拥有状态机、执行、升级、receipt 与 replay 技术合同 |
| 消费者 | gameplay、Agent 与入口以稳定协议提交 intent、观察 committed state 或验证证明 | `doc/game/prd.md`、`doc/world-simulator/prd.md`、入口/Viewer authority 拥有各自产品行为与体验 |
| 证据 | 目标能力只在同一候选的跨域专业证据成立时才可表达当前状态 | `doc/testing/prd.md` 与根 `README.md` 分别拥有验证和公开 claim envelope |

产品层不定义共识实现、签名算法、消息/存储 schema、runtime ABI、游戏规则、Agent 行为、UI、经济参数或操作 runbook。冲突时以更窄的安全和权威边界为准，并由 P2P 或 runtime 专业 owner 显式裁决。

## 4. 路线图

1. 用持久、可复验的 commit certificate 取代当前 stake-threshold prototype，并补全 round、锁定、验证者转换和分区恢复。
2. 建立 hash-bound state availability、按角色的存储/serving、bootstrap/recovery 信任链和 restore drills。
3. 把 deterministic execution 与共识隔离为可版本化语义合同，完成 certificate-gated execution、升级与 replay 证明。
4. 以同一协议支持 game/Agent/入口的并发消费，逐步扩展 proof-serving 与 light companion，同时保持 finality 缺失 fail closed。

### 基础不变量

- 单一世界：global canonical order 与 `world_id` 不因区域、环境、节点或缓存而分叉；local development 世界使用独立身份且永不并入。
- 最终性先于效果：未获得可验证 finality certificate 的 intent 不产生权威世界结果；基础设施不可用时 progression fail closed。
- 可重建性：恢复仅接受 manifest、certificate、hash-bound snapshot、canonical replay 和 verified state root 组成的信任链。
- 可替换实现：oasis7 保有协议语义；可选择性采用成熟库，但依赖被版本化合同隔离。
- 消费者边界：game、Agent 和入口与基础设施并发运行，却不能直接写 canonical state 或把 pending 伪装成 committed。

### Finality 缺失时的意图连续性

当已签名并送出、尚待 finality 的 intent 等待最终确认，而 finality 变为不可用、陈旧或无法验证时，玩家与 Agent 可以继续观察最后一个已验证的世界状态，并保留该 intent 作为**尚无世界效果的待决请求**；这不授予资源、控制权、资格、声誉、阶段完成或任何依赖该 intent 的后续结果。提交被接收、本地排队或界面仍显示该请求，都不能被表达为执行成功或对完成时间的承诺。

恢复后，待决请求必须先按当时仍有效的权限与前置条件重新进入 canonical 顺序；它可能被执行、拒绝、过期或需要替换，且只有可验证的 committed receipt 才能改变玩家/Agent 的世界结论。产品体验必须让消费者区分“仍待决且未生效”与“已无效、被拒绝或须重新规划”，并提供查看恢复状态、等待下一次可验证结果或在对应专业域允许时明确撤回/替换/重提的路径。不得静默重放失效控制、重复执行同一请求，或在恢复前后把本地推测包装成权威结果。

当玩家或 Agent 将一个仍待决的 intent 明确作为替代对象时，替代动作本身也只是待决请求：在 canonical committed receipt 确认之前，它不得单方面取消、覆盖、隐藏或宣称优先于原请求，也不得让两个互斥选择同时被表述为已获授权、已保留资源或即将生效。产品必须保留原请求与其撤回/替代请求之间可读的关联，并分别表达仍待决、已提交替代、撤回/替代被拒绝、原请求已执行或替代已生效等真实状态；若专业域不支持安全的撤回或替代，入口只能提供等待、查看状态或重新规划，不能把普通重提伪装成取消。该边界既防止 outage 后的重复/抢先效果，也不阻止专业域明确允许的独立并发 intent。

对于被专业域明确标为同一 intent lineage 中互斥成员的请求，首个产生有效世界效果的 committed receipt 是唯一胜者，并原子地将其余待决成员终止为无效果、不可执行且可追溯的状态。拒绝或过期只终止其自身，不获胜也不取消其他成员；未被标为互斥成员的独立 intent 仍可并发。产品层不定义 lineage 标记、排序、原子化或 receipt schema。

本条只约束跨域产品承诺；pending 的持久化、去重、重试、过期、receipt 字段和具体 UI 由 runtime、P2P、Agent 与入口/Viewer 专业域分别拥有。

### 恢复目标身份与世界隔离边界

恢复成功只能恢复原有的 `world_id` 及其可验证历史，不能把“某个节点重新可运行”“可读取部分数据”或“建立了新的本地/测试世界”表达为玩家原世界已经回来。恢复候选必须同时能关联既有 `world_id`、受验证 checkpoint 或 snapshot、canonical replay 与 state root；任一关联缺失、相互冲突或指向其他 `world_id` 时，产品结论只能是原世界**不可用且尚未恢复**。此时不得继续接受新的权威 intent、为新世界发放原世界的资格或资源结论，或让入口把替代 endpoint、缓存状态、旧截图或本地开发世界伪装成连续世界。

玩家在该状态下可以查看最后一个已验证 receipt 与明确的世界不可用/待恢复说明；这不是新行动、资产转移、控制权、Agent 授权、组织资格或已完成进度的恢复承诺。已经 committed 的历史仍以其原 receipt 为准；未 final 的 intent 继续遵守上文的待决规则，不因恢复目标不明而被静默确认、取消、重放或迁移。恢复完成后，只有同一 `world_id` 的验证链重新成立，消费者才可按 committed state 恢复结论。

若原 `world_id` 无法被证明恢复，产品不得自行把玩家、资产、Agent、组织或历史复制到替代世界，也不得把替代世界称作自动升级、灾备恢复或无损迁移。是否建立新世界、是否提供受治理的迁移方案及其同意、结算和申诉规则，是需要独立跨模块决策的产品变更；在该决策及其专业合同与证据完成前，唯一安全路径是维持隔离并如实报告不可用。具体 identity 检验、checkpoint 格式、停止服务/投票机制、endpoint 行为和界面/API 字段仍由 P2P、runtime、Viewer 与测试专业域拥有。

### 恢复后的重新服务闸门

恢复证明与重新接受权威写入是两个不同的产品结果。即使同一 `world_id` 的历史已经可以读回，消费者也不能仅凭节点可运行、状态可读或 replay 已结束，就把入口重新表述为可以提交新 intent。恢复后的对外状态至少分为以下三类产品语义（不冻结字段或枚举名称）：

- **恢复只读**：同一世界的 identity、checkpoint/snapshot、canonical replay 与 state root 已重新关联，玩家/Agent 可以查看最后一个已验证 receipt、历史和仍无世界效果的 pending；但当前追加、最终性或执行服务尚未重新满足其专业启用条件。新 intent 必须原子拒绝或保持明确的无效果待决，不能取得资源、资格、排队优先或隐性承诺。
- **恢复可服务**：除同一世界验证链外，当前版本化执行规则、追加/最终性权威和 head 的单调连续性也已由专业证据重新成立，消费者可以把新的结果绑定到新产生的 committed receipt。只有进入这一语义状态后，入口才可接受新的权威 intent；“进程已启动”“能读取旧状态”或“某个 endpoint 可达”均不足以满足该闸门。
- **恢复受阻/隔离**：identity、版本、head、root、追加权威或最终性证据缺失、冲突、回退或再次失效。消费者回到不可用/待恢复语义，不接受新的权威 intent，也不把只读历史、缓存或替代世界表达为连续服务。

从恢复只读转为恢复可服务时，既有 pending 必须以当前仍有效的权限、资源、版本和前置条件重新裁决，并按 canonical 顺序处理；停机期间的排队、客户端重试或历史 receipt 不得自动延长期限、继承优先级或制造第二次效果。替代/撤回请求继续与原请求保持可读关联，只有各自的 committed receipt 才能确认其世界结果。若重新服务闸门在提交或恢复过程中失效，系统必须在产生世界效果前退回原子拒绝或无效果待决语义；已经确认的 receipt 不因闸门回退而被撤销、重放或改写。

这项闸门保持 **world-first**：是否重新开放写入由当前可验证世界与执行/最终性条件决定，而不是由本地进程或界面状态决定；保持 **persistent / auditable**：只读恢复、可服务恢复、再次隔离以及 pending 的重新裁决均可与同一世界历史关联；保持 **extensible**：未来可增加服务角色或证明步骤，但不能把只读恢复降格为可写，也不能跳过当前条件。

### 工业流水线的跨域执行边界

工厂、配方、原材料和物流不是四条可以各自结算、最后再拼成“生产完成”的本地流程。它们只有在同一条可验证世界历史中形成一条因果流水线时，才可以产生玩家或 Agent 能据此行动的工业结果。本节只定义基础设施与工业消费者之间的组合边界，不定义配方比例、产能、价格、物流算法、排队或玩家体验；工业生命周期与材料适用性仍由 [`世界规则与核心玩法`](../world-rules-core-gameplay/prd.md) 及其链接的 M4 / runtime 专业合同拥有。

- **单一流水线身份：** 在工业动作跨过“已接受”边界时，权威世界必须为该次操作建立一个不可变的 root identity，并将它绑定到同一 `world_id`、当前版本化执行规则和 parent committed state。工厂能力/位置、配方版本、原料来源与目标账本、物流 path/edge 以及其 revision/segment 只作为该 root 的可追溯上下文；具体字段、事件和 receipt schema 由 [`world-runtime industrial execution status and authority matrix`](../../world-runtime/prd.md#industrial-execution-status-and-authority-matrix) 定义，产品层不复制 schema。
- **跨阶段守恒：** 工厂启动、配方输入 join、原料 handoff、运输/到达、buffer 或产物交付等 child effect 必须保留 root、所属 revision/segment 与直接 parent/child 关系。首个不可逆 input sink、运输/目的账本 credit 或产出进度前，root、版本、来源/目的地或 parent 关系缺失、冲突或无法验证时，必须原子 fail closed；预览、推荐、pending 或本地排队不得产生上述世界效果。
- **容量承诺与有界背压：** “已接受”只建立该次工业意图的 root，不等同于已经取得阶段槽位、边吞吐、目的地 buffer/terminal 容量或原材料预留。任何 hold/reservation 必须数量有界、不可被多个下游同时承诺，并绑定 root、revision/segment、阶段/边、批次、消费主体和生命周期；预览、报价、计划、accepted-pending 或本地排队不得生成 hold。每条中间边与目的地 buffer 必须有确定性的容量上限和满载处置：保留未消费的上游投入、仅在仍有容量时接收已结算产出，或原子拒绝/延期新的上游承诺。不得静默丢弃、瞬移、无限堆积、自动改道或伪造下游完成；同一过期、撤销、重规划、容量释放或到达事件对同一 hold 的释放，以及该事件触发的 fresh authoritative snapshot 重评，各最多产生一次效果。后续不同的权威事件可以再次重评仍有效意图的未满足部分，但不得复制 hold/库存/进度、回滚已消费量或靠 replay 重复同一效果。
- **因果变更与恢复：** 在同一因果计划内恢复只能继续原 root；若改变工厂、配方、输入/输出、物流边或终端等会改变产出因果的条件，必须建立与旧 root 可读关联的新 revision/child root，重新校验输入、容量、权限和世界版本，不能静默迁移旧任务。重试、重连、snapshot restore 与 replay 只重读同一 root 的 disposition；同一阶段/边/批次不得复制 sink、credit、进度、receipt 或奖励。
- **阻塞与可观察性：** 跨域消费者必须能从 committed state 区分预览、已接受、进行中、被阻塞、已完成与已终止，并定位最早可归因的 stage/edge、仍占用与已消费的投入、下一次权威复查边界和真实可用的恢复/退出路径。Viewer、Agent 和纯 API 可以采用各自读面，但不得把工厂可用、配方已接受、物流已发运或本地重连表述为交付、结算或生产完成。

#### Finality / service outage during industrial execution

现有的通用 finality 规则只能说明“待决 intent 尚无世界效果”，而工业流水线还必须区分**尚未形成 root 的待决请求**与**已经形成 root、但暂时无法推进下一个 child effect 的已提交操作**。否则客户端或 Agent 可能把本地 accepted ack 当成原料预留，也可能在恢复时把已消费投入重放成第二次生产。

- **提交前不可用：** 签名/送达但尚未取得 committed finality 的工业 intent 不创建 root、hold、reservation、input sink、WIP、物流 credit、产出或稳定进度；玩家只能看到仍待决、被拒绝/过期或需要重新规划的真实结果。重连、重复提交和本地排队不能把它升级为已接受工业操作。
- **提交后服务中断：** 已有 committed root 的流水线若在 stage finish、transit、buffer admission 或 terminal settlement 前遇到 finality/service outage，已提交的 hold、sink、WIP、在途或已到达状态保持原 lineage 与实际数量；服务中断本身不产生新 child effect、额外扣减、delivery credit、W 或奖励。surface 必须把“已提交但服务受阻”与“尚无世界效果的待决请求”分开，不能把本地恢复、缓存或重连当成继续生产/交付。
- **恢复闸门：** 服务恢复后，下一 child effect 必须从同一 `world_id` 的 fresh authoritative snapshot 重新校验版本、factory/recipe fit、批次适用性、owner/权限、路径与容量、power/terminal 前置和仍有效的 window/lease；结果只能是沿同一 root 继续、进入 profile 支持的有界 hold/defer/reject/expire disposition，或因产出因果变化建立已有规则要求的 linked revision/child root。恢复不得按 outage 时长隐式续租、重排优先级、补发失效 ETA 或把旧 quote 当作新授权；world-time/order 与 expiry disposition 以 canonical journal/replay 为准，不以客户端墙钟或重试次数重算。
- **替代与审计：** 只有专业合同支持的撤回、替代或补偿才能终止/转换已提交操作，并须保留原 root、已消费/仍占用投入、在途损耗和前后 disposition 的可读关联；普通重提不能取消旧操作或绕过其终端结算。每个恢复重评、释放、终止或补偿至多产生一次效果，既有 receipt 不被撤销、重写或重复消费。

该补充保持 **world-first**：工业效果由 committed finality 和恢复后的权威重评决定，不由 endpoint 可达或本地 ack 决定；保持 **emergence-first**：玩家仍可在等待、修复、改道、延期或受支持处置之间作有边界的选择，不获得 outage 后免费重启；保持 **persistent / auditable**：root、child、hold、window、投入、损耗、恢复原因和最终 disposition 跨停机、恢复与 replay 延续；保持 **extensible**：未来可加入不同的 service/finality profile，但不能把未 final 请求变成已生效或跳过 fresh revalidation。

玩家可以区分“尚无世界效果的待决请求”“已提交但服务受阻的工业操作”和“恢复后重新校验的继续/终止结果”，查看已消费与仍占用的价值、真实复查边界，并选择专业合同允许的等待、修复、改道、延期、撤回或补偿；玩家不能把 accepted ack、缓存、旧 quote、重连或普通重提当作 root、预留、交付、自动续租或第二次生产授权。runtime/consensus 事件、window/lease 算法、具体 UI 和 Agent retry 策略仍由专业 authority 冻结。

该边界保持 **world-first**：工业结果由同一 `world_id`、版本化执行与 committed receipt 决定，而不是由某个设施进程或客户端缓存决定；保持 **emergence-first**：玩家/Agent 可在约束内选择来源、工厂、路线和恢复，不获得预写的免费转换；保持 **persistent / auditable**：root、child lineage、批次、receipt、损耗与 terminal disposition 跨重连、恢复和 replay 延续；保持 **extensible**：未来增加新的工厂、配方、材料或物流 profile 时复用同一组合边界而不改变既有历史。

## 5. Done：成功标准与验收

- SC-1：一个受治理验证者集合在唯一 `world_id` 上形成可验证的 deterministic BFT commit certificate；错误签名、阈值、验证者集合或 round 状态均不能推进历史。
- SC-2：验证者、full/state-sync、archive、light companion 与公开服务在各自角色内复制、提供和验证状态；任何非权威服务不取得最终性或写入权。
- SC-3：bootstrap、snapshot、replay、state sync、pruning 和灾难恢复证明同一历史/状态根可重建；任一证明缺失或不匹配时停止 serving/voting。
- SC-4：全部活动验证者在 attestation 前重执行相同版本化执行；执行升级、混合版本和 replay 不会为同一输入产生两个权威结果。
- SC-5：游戏、Agent 与玩家入口经同一版本化协议只处理 committed state；finality 不可用时，待决 intent 不产生下游资格或世界效果，消费者能区分仍待决与须重新规划的结果，并只在恢复后取得的 committed receipt 上更新结论。明确互斥的同 lineage 成员中，首个产生有效世界效果的 receipt 唯一获胜并原子终止其余成员；拒绝/过期只终止自身，独立 intent 保持并发；原请求与撤回/替代请求的关联及各自真实状态可读。
- SC-6：恢复路径只在同一 `world_id` 的 checkpoint/snapshot、canonical replay 与 state root 验证链成立时恢复世界结论；缺失、冲突或其他 `world_id` 的候选保持原世界不可用和隔离，不接受新权威 intent，也不把本地/测试/替代世界、缓存或部分数据表述为连续恢复。已 committed receipt 保留其原历史，未 final intent 不被静默结算、取消、重放或迁移；任何替代世界或受治理迁移必须作为独立产品决策，而非恢复副作用。
- SC-7：同一候选的恢复演练至少各覆盖一例“恢复只读”“恢复可服务”和“恢复受阻/隔离”，并覆盖一次闸门在提交或恢复中回退；同一 `world_id` 的历史验证链是必要但不足以重新接受写入，追加/最终性/版本化执行或 head 连续性未重新成立时，所有新 intent 尝试均原子拒绝或保持无效果待决（每个受测新 intent 的 committed receipt 数为 `0`）。进入可服务后，既有 pending 按当前条件和 canonical 顺序重新裁决；停机排队与历史 receipt 不延长期限、不继承优先级；每个被接受的新 intent 最多产生一个 committed receipt，且不产生第二次效果。闸门再次失效时回到只读/隔离且不撤销、重放或改写已确认 receipt。玩家/Agent 能读到当前服务语义、主要 blocker 与下一步。测试层级：`test_tier_full`。
- SC-8：同一条代表性 `factory → recipe/input join → raw-material handoff → logistics/transit → output or terminal` 流水线在同一 `world_id`、版本化执行规则和 immutable root 下完成可追溯的 accepted/blocked/terminal 结果；任何 child effect 在缺失或冲突 root、revision、parent 或目的状态时于首个不可逆 sink 前 fail closed。已接受意图不得被表述为已获得容量或原料预留；每个 hold/reservation 必须有界、独占并绑定 root/阶段/边/批次，且每条中间边和目的地 buffer 满载时只能保留未消费投入、接收仍有容量的已结算产出，或原子拒绝/延期新的承诺，不得丢弃、瞬移、无限堆积、自动改道或伪造完成。同一权威释放/到达事件对同一 hold 的释放与其 fresh snapshot 重评各最多产生一次效果；后续不同事件只能从新的权威快照重评仍有效意图的未满足部分。改变产出因果的工厂、配方、输入/输出、物流边或终端条件必须产生 parent-linked revision/child root；retry、reconnect、restore 与 replay 只能重读同一 disposition，不产生第二次 sink、credit、进度、receipt 或奖励。玩家/Agent 可读最早 blocker、仍占用/已消费投入、未满足/剩余量与下一次复查边界，但不能把局部完成、在途或本地缓存当作交付/生产完成。测试层级：`test_tier_full`。
- SC-9：同一代表性流水线分别覆盖 committed finality 前的待决工业 intent、root 已提交但 child effect 因 finality/service outage 暂停、以及恢复后的 fresh revalidation。前者不得产生 root、hold、sink、W 或资格；后者保留既有 root/投入/损耗/hold/window，不产生第二次 child effect、delivery credit、隐式续租或优先级刷新；恢复只能沿同一 root 产生一次继续、受支持的 hold/defer/reject/expire/compensation disposition，或按产出因果变化建立 linked revision/child root。canonical journal/replay、window/lease expiry、重连/重复提交和 Viewer/Agent/pure API 必须保持上述状态与 lineage 一致，并把 accepted ack、服务受阻和已结算结果区分开。测试层级：`test_tier_full`。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| SC-1 | blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-P2P-001 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | certificate、验证者转换、分区/Byzantine 与权限负例 | test_tier_full |
| SC-2 | blockchain_ops_engineer / qa_engineer | PRD-P2P-001 / PRD-P2P-002 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/testing/prd.md` | 复制、存储角色、proof-serving 与非权威权限负例 | test_tier_full |
| SC-3 | blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-P2P-002 / PRD-WORLD_RUNTIME-003 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | bootstrap、checkpoint、snapshot、replay、root verification 与 restore drill | test_tier_full |
| SC-4 | runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | deterministic re-execution、upgrade activation、replay 与 mixed-version 拒绝 | test_tier_full |
| SC-5 | producer_system_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 本文 `Finality 缺失时的意图连续性` 对应的 committed/pending protocol boundary、待决/无效或重规划结果的可区分性、恢复后重新校验与 receipt-gated 结果、proof verification；包含同 lineage 互斥成员的竞态/同序唯一胜者、拒绝/过期仅终止自身、replay/重复重试无第二效果及独立 intent 并发负例，以及正式 surface 的关联/状态可读性核对 | test_tier_required |
| SC-6 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-P2P-001 / PRD-P2P-002 / PRD-WORLD_RUNTIME-003 / `viewer-control-plane-split-live-playback.prd.md` / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`; `doc/testing/prd.md` | 同一 `world_id` 的 checkpoint/snapshot、replay 与 root 验证恢复样例；缺失、冲突或其他 `world_id` 候选的 fail-closed 负例，断言不接收新权威 intent、不把替代 endpoint/缓存/本地世界表述为恢复，并核对 committed receipt 与未 final intent 的连续性；若触达可见 Viewer 恢复/重连/回放表述，按该 Viewer authority 和 `testing-manual.md` S6 核对 canonical snapshot/feedback、可见状态与 browser console 不把本地回放或重连表述为恢复 | test_tier_full |
| SC-7 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-P2P-001 / PRD-P2P-002 / PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-003 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/prd.md`; `doc/testing/prd.md` | 同一候选至少覆盖恢复只读、恢复可服务、恢复受阻/隔离各一例及一次闸门回退；未满足闸门时新 intent 的原子拒绝/无效果待决（committed receipt 为 0）；满足闸门后的 pending 当前条件重裁决与 canonical 顺序；停机排队不继承期限/优先级；每个新 intent 至多一个 committed receipt、无第二次效果；回退后的 fail-closed 与已确认 receipt 连续性；正式 surface 可读服务语义、blocker 与下一步 | test_tier_full |
| SC-8 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/product/world-rules-core-gameplay/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | 代表性工厂→配方/input join→原料 handoff→物流/transit→产物/终端路径的 root/revision/parent lineage、缺失/冲突 identity fail-closed、已接受与容量/预留的区分、有界 hold 与 buffer 满载背压、释放后单次重评、因果 cutover、blocked/terminal 读面、单次 sink/credit 与 retry/restore/replay 证据；包含 Agent/Viewer/pure API 对 blocker、投入、未满足/剩余量、复查边界与 terminal disposition 的读取一致，且不把局部完成、在途或缓存伪装为交付/生产完成 | test_tier_full |
| SC-9 | producer_system_designer / gameplay_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-WORLD_RUNTIME-043 / PRD-WORLD_SIMULATOR-047 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/product/world-rules-core-gameplay/prd.md`; `doc/world-simulator/m4/industrial-resource-flow-contract.prd.md`; `doc/testing/prd.md` | finality 前待决 intent 与 root 已提交但 service 受阻的工业路径分离；恢复后的同 root fresh revalidation、window/lease expiry 与一次性 continuation/disposition；不会复制 child effect、sink、delivery credit、W、优先级或奖励；canonical journal/replay/restore 以及 Agent/Viewer/pure API 对 pending、blocked、resumed/terminated 与投入/损耗/复查边界的语义一致 | test_tier_full |

## 6. Non-Goals

这是目标架构，不是当前可用性或发行声明。当前代码仍须由 P2P/runtime/QA 同一候选证据评估；根 `README.md` 独占公开 claim envelope。

- 不把当前 stake-threshold prototype 误报为完整 BFT certificate、分区恢复、permissionless 服务可用性或主网。
- 不以本模块定义区域设施、工业/市场的玩法规则、配方/价格/产能/物流数值、charter、frontier、普通治理、资源平衡、Agent 行为或玩家体验；本模块仅定义本节的跨域执行边界。
- 不把运维拓扑、软件部署、SLA、费用/奖励数值或第三方库选择写成产品已交付事实。
- 不批量改写历史文档，不制造兼容 redirect 壳，不改变当前公开状态或发布声明。
