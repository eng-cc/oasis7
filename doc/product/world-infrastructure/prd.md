# 大世界基础设施 PRD

## 文档身份

- 产品模块：大世界基础设施
- 产品模块 slug：`world-infrastructure`
- 产品层唯一 PRD：`doc/product/world-infrastructure/prd.md`
- 产品模块总入口：`doc/product/README.md`
- Product PRD-ID：`PRD-PRODUCT-002`
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-08-07`
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
- [`区域能力与扩展（待迁移）`](governed-regional-capabilities-and-extensions.prd.md)、[`区域 charter/tenure（待迁移）`](regional-charter-tenure-and-public-funding.prd.md)、[`工业/市场（待迁移）`](governed-industry-market-and-emergency-supply.prd.md) 与 [`普通治理（待迁移）`](global-governance-organization-continuity-and-constitutional-guardrails.prd.md) 都是 `superseded` 迁移债务：各页迁移头记录接收 owner 与删除条件，内容仍可供接收模块 owner 迁移，但这些路径不再构成本模块产品 authority、路线图、active topic 或验收。原 frontier 迁移债务已由世界规则与核心玩法吸收，不再保留在本模块索引。

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

## 5. Done：成功标准与验收

- SC-1：一个受治理验证者集合在唯一 `world_id` 上形成可验证的 deterministic BFT commit certificate；错误签名、阈值、验证者集合或 round 状态均不能推进历史。
- SC-2：验证者、full/state-sync、archive、light companion 与公开服务在各自角色内复制、提供和验证状态；任何非权威服务不取得最终性或写入权。
- SC-3：bootstrap、snapshot、replay、state sync、pruning 和灾难恢复证明同一历史/状态根可重建；任一证明缺失或不匹配时停止 serving/voting。
- SC-4：全部活动验证者在 attestation 前重执行相同版本化执行；执行升级、混合版本和 replay 不会为同一输入产生两个权威结果。
- SC-5：游戏、Agent 与玩家入口经同一版本化协议只处理 committed state；finality 不可用时，待决 intent 不产生下游资格或世界效果，消费者能区分仍待决与须重新规划的结果，并只在恢复后取得的 committed receipt 上更新结论。明确互斥的同 lineage 成员中，首个产生有效世界效果的 receipt 唯一获胜并原子终止其余成员；拒绝/过期只终止自身，独立 intent 保持并发；原请求与撤回/替代请求的关联及各自真实状态可读。
- SC-6：恢复路径只在同一 `world_id` 的 checkpoint/snapshot、canonical replay 与 state root 验证链成立时恢复世界结论；缺失、冲突或其他 `world_id` 的候选保持原世界不可用和隔离，不接受新权威 intent，也不把本地/测试/替代世界、缓存或部分数据表述为连续恢复。已 committed receipt 保留其原历史，未 final intent 不被静默结算、取消、重放或迁移；任何替代世界或受治理迁移必须作为独立产品决策，而非恢复副作用。

### 5.1 验收追踪

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| SC-1 | blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-P2P-001 / PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | certificate、验证者转换、分区/Byzantine 与权限负例 | test_tier_full |
| SC-2 | blockchain_ops_engineer / qa_engineer | PRD-P2P-001 / PRD-P2P-002 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/testing/prd.md` | 复制、存储角色、proof-serving 与非权威权限负例 | test_tier_full |
| SC-3 | blockchain_ops_engineer / runtime_engineer / qa_engineer | PRD-P2P-002 / PRD-WORLD_RUNTIME-003 / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/testing/prd.md` | bootstrap、checkpoint、snapshot、replay、root verification 与 restore drill | test_tier_full |
| SC-4 | runtime_engineer / blockchain_ops_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-P2P-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/p2p/prd.md`; `doc/testing/prd.md` | deterministic re-execution、upgrade activation、replay 与 mixed-version 拒绝 | test_tier_full |
| SC-5 | producer_system_designer / runtime_engineer / agent_engineer / viewer_engineer / qa_engineer | PRD-WORLD_RUNTIME-001 / PRD-TESTING-003 | `doc/world-runtime/prd.md`; `doc/testing/prd.md` | 本文 `Finality 缺失时的意图连续性` 对应的 committed/pending protocol boundary、待决/无效或重规划结果的可区分性、恢复后重新校验与 receipt-gated 结果、proof verification；包含同 lineage 互斥成员的竞态/同序唯一胜者、拒绝/过期仅终止自身、replay/重复重试无第二效果及独立 intent 并发负例，以及正式 surface 的关联/状态可读性核对 | test_tier_required |
| SC-6 | producer_system_designer / blockchain_ops_engineer / runtime_engineer / viewer_engineer / qa_engineer | PRD-P2P-001 / PRD-P2P-002 / PRD-WORLD_RUNTIME-003 / `viewer-control-plane-split-live-playback.prd.md` / PRD-TESTING-003 | `doc/p2p/prd.md`; `doc/world-runtime/prd.md`; `doc/world-simulator/viewer/viewer-control-plane-split-live-playback.prd.md`; `doc/testing/prd.md` | 同一 `world_id` 的 checkpoint/snapshot、replay 与 root 验证恢复样例；缺失、冲突或其他 `world_id` 候选的 fail-closed 负例，断言不接收新权威 intent、不把替代 endpoint/缓存/本地世界表述为恢复，并核对 committed receipt 与未 final intent 的连续性；若触达可见 Viewer 恢复/重连/回放表述，按该 Viewer authority 和 `testing-manual.md` S6 核对 canonical snapshot/feedback、可见状态与 browser console 不把本地回放或重连表述为恢复 | test_tier_full |

## 6. Non-Goals

这是目标架构，不是当前可用性或发行声明。当前代码仍须由 P2P/runtime/QA 同一候选证据评估；根 `README.md` 独占公开 claim envelope。

- 不把当前 stake-threshold prototype 误报为完整 BFT certificate、分区恢复、permissionless 服务可用性或主网。
- 不以本模块定义区域设施、工业/市场、charter、frontier、普通治理、资源平衡、Agent 行为或玩家体验。
- 不把运维拓扑、软件部署、SLA、费用/奖励数值或第三方库选择写成产品已交付事实。
- 不批量改写历史文档，不制造兼容 redirect 壳，不改变当前公开状态或发布声明。
