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

## 3. 当前与目标的分离

当前 runtime/consensus 集成不得因本产品目标自动被表述为 BFT-ready、分区恢复完成或可公开发行。完整目标需要 runtime version activation、certificate-gated execution/replication、replay compatibility、恢复后的 root verification，以及与 P2P 相同候选版本的对抗性证据。

## 4. 组合验收

- DE-1：相同 execution version、已排序输入和 world state 在全部活动验证者上产生相同结果；缺证、冲突、越权或版本不匹配的输入不产生部分副作用。
- DE-2：游戏/Agent/入口通过稳定协议仅见 committed state，能验证或获得适用证明，并在 finality 缺失时获得不把 pending 包装为结果的失败语义。
- DE-3：执行升级、snapshot/replay、node recovery 与版本混合的样例证明同一 `world_id` 历史和 state root 连续；未证明则 fail closed。

## 5. Non-Goals

- 不定义配方、设施、市场、区域/组织治理、Agent 决策、玩家动作、UX 或数值平衡。
- 不定义 BFT 消息、签名格式、存储实现、节点部署或具体运行手册。
