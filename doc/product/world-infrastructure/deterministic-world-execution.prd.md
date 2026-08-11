# 确定性世界执行

## 文档身份

- 所属产品模块：大世界基础设施
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文定义分布式共识底层之上的确定性世界执行层。它是基础设施内部的上层：接收已签名意图、在版本化规则边界内确定性重执行、提交已最终化结果，并向游戏、Agent 与玩家入口提供稳定协议；它不定义那些消费者的规则、行为或界面。

## 1. 执行与共识边界

- 每个活动验证者在 attestation 前完整重执行同一输入。权威世界结果只在 verified commit certificate 后生效；未最终化的 signed intent 不改变世界。
- 共识和执行是独立协议而非必须独立进程：它们通过版本化语义合同通信，in-process 与未来 IPC/network transport 必须通过同一 conformance suite。首个 triad 可单进程共运行。
- 执行版本、manifest 与激活必须可审计、治理激活且 replay-safe。软件 rolling upgrade、治理激活 runtime manifest、独立客户端升级、基础协议 fork 是不同 release lane；若混合版本能对同一输入计算不同权威结果，则必须走协调协议升级/fork。

## 2. 消费者和部署边界

- gameplay、Agent 和 Viewer/玩家入口是基础设施的并发消费者，不是基础设施内的产品循环。游戏提交 intent 并投影 committed state；它不直接写入或以未最终化状态推进权威世界。
- 普通玩家运行游戏加 light companion；operator 运行 full infrastructure node；development/local test 运行游戏加 embedded 或 full local node。所有 profile 使用同一版本化 game/infrastructure 协议，只改变共识、存储、执行和验证职责。
- 最终性不可用时权威 progression fail closed；pending signed intent 在最终化前无世界效果。local development world 必须使用不同 `world_id`，且永不合入全局历史。

### 2.1 Finality 中断时的待决 intent 连续性

- 已签名并送出的 intent 在最终性不可用、陈旧或无法验证时，只能保留为**尚无世界效果的待决请求**。本地排队、提交被接收或界面仍显示请求，都不授予资源、控制权、资格、声誉、阶段完成或依赖它的后续结果，也不得被表达为执行成功或对完成时间的承诺。
- 恢复后，待决请求必须按当时仍有效的权限和前置条件重新进入 canonical 顺序；它可能被执行、拒绝、过期或需要替换。只有可验证的 committed receipt 才能更新玩家、Agent 或入口的世界结论。消费者必须区分仍待决与已无效、被拒绝或须重新规划，并提供查看状态、等待结果，或在专业域允许时明确撤回、替换或重提的路径。
- 替代或撤回请求本身也是待决请求，在各自 committed receipt 出现前，不得单方面取消、覆盖、隐藏或宣称优先于原请求；原请求与其撤回/替代请求之间的关联和真实状态必须可读。若专业域不支持安全的撤回或替代，消费者只能等待、查看状态或重新规划，不能把普通重提伪装成取消。
- 对被专业域明确标记为同一 intent lineage 中互斥的成员，首个产生有效世界效果的 committed receipt 是唯一胜者，并原子地终止其余成员为无效果、不可执行且可追溯的状态。拒绝或过期只终止自身；未标记为互斥的独立 intent 仍可并发。产品层不定义 lineage 标记、排序、原子化、pending 持久化、去重、重试、过期或 receipt schema。

### 2.2 治理激活与在途 intent 的版本边界

- 运行时 manifest 的激活边界是 canonical 世界事实；玩家、Agent、客户端提交时间、本地队列或节点软件版本都不能自行绑定世界语义。一个 intent 只有在首次进入 canonical execution block 时才确定执行版本：该 block 位于激活边界之前则使用旧 manifest，位于激活边界或之后则使用新 manifest。等待中的请求不会因为本地已经提交或旧节点仍在运行而自动锁定旧规则。
- 激活边界前提交、但尚未取得 committed receipt 的请求，若在新 manifest 下首次进入 canonical block，必须按新 manifest 的当前权限、资源和前置条件重新裁决；不兼容时只能原子拒绝/过期并给出可读原因，或由玩家/Agent 通过明确的新请求重新规划。不得静默翻译 payload、沿用旧报价/资格、部分应用旧规则后再套新规则，或让重连把它伪装成已结算。
- 玩家可以查看待决请求、等待 canonical 结果、改道或在专业域允许时撤回/替换；不能把客户端版本、旧缓存、提交回执或本地倒计时当成执行版本、成本、权限、优先级或完成保证。跨版本重新提交是新的 intent，必须经过同一权威顺序并至多产生一次新的世界效果，不能借旧请求或历史 receipt 获得第二次效果。
- 已 committed 的历史结果永远按其所在 block 的 manifest 重放；恢复、审计和 replay 不得用当前 manifest 重新解释旧 receipt，也不得因激活而追溯改写成本、权限、资源或责任。若激活证明、版本工件或兼容性检查缺失/冲突，执行与恢复必须 fail closed，不产生部分世界效果，保留已确认历史并把未确认请求留在真实的待决/拒绝/重新规划路径。

本条只规定版本选择、玩家可见边界与单次效果语义；manifest 字段、兼容矩阵、激活高度、pending 持久化、重试/过期与 receipt schema 仍由 runtime、P2P、Agent 和入口/Viewer 专业权威定义。

## 3. 当前与目标的分离

当前 runtime/consensus 集成不得因本产品目标自动被表述为 BFT-ready、分区恢复完成或可公开发行。完整目标需要 runtime version activation、certificate-gated execution/replication、replay compatibility、恢复后的 root verification，以及与 P2P 相同候选版本的对抗性证据。

## 4. 组合验收

- DE-1：相同 execution version、已排序输入和 world state 在全部活动验证者上产生相同结果；缺证、冲突、越权或版本不匹配的输入不产生部分副作用。
- DE-2：游戏/Agent/入口通过稳定协议仅见 committed state，能验证或获得适用证明，并在 finality 缺失时将 pending 表达为无世界效果的待决请求，而非结果。恢复样例必须证明待决请求按当时条件重审、只有 committed receipt 更新结论，并能区分待决、无效/拒绝、须重新规划及已生效；对明确互斥的同 lineage 成员，竞态、替代/撤回、重复重试和 receipt 重放至多产生一个有效世界效果，拒绝/过期不取消独立 intent。
- DE-3：执行升级、snapshot/replay、node recovery 与版本混合的样例证明同一 `world_id` 历史和 state root 连续；未证明则 fail closed。
- DE-4：版本激活窗口样例证明 intent 的执行版本由首次进入的 canonical block 与激活边界确定，而不是由提交端版本或本地时间决定；激活前待决请求在激活后按新 manifest 重新校验，不能兼容时原子拒绝/过期或经明确新请求重提，不能静默翻译、沿用旧报价或产生部分副作用。跨版本重试至多结算一次，历史 receipt 按原 block 的 manifest replay，激活证据缺失/冲突时 fail closed。玩家可区分待决、拒绝/过期、重规划与已结算结果。测试层级：`test_tier_full`。

## 5. Non-Goals

- 不定义配方、设施、市场、区域/组织治理、Agent 决策、玩家动作、UX 或数值平衡。
- 不定义 BFT 消息、签名格式、存储实现、节点部署或具体运行手册。
