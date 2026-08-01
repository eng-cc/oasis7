# 分布式共识与状态可用性

## 文档身份

- 所属产品模块：大世界基础设施
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文定义 oasis7 区块链/分布式系统底层向上层确定性世界执行提供的产品级保证。它不定义共识消息、密码学、网络协议、节点配置、存储格式或运行步骤；这些由 P2P、共识和运维专业权威拥有。

## 1. 基础承诺

基础层为一个 `world_id` 提供唯一的可验证提交序列：只有经验证的共识最终性才使输入成为世界历史。它提供节点间复制、状态同步、可验证的数据可用性、故障恢复与证明服务；它不定义游戏规则、Agent 行为、玩家交互或发行体验。

确定性 BFT 是目标模型：oasis7 自己拥有协议语义，首个实现采用 Tendermint/CometBFT 风格的 `Propose -> Prevote -> Precommit -> Commit` 轮次。有效 commit certificate 必须证明活动、治理注册的验证者集合中超过三分之二质押权重的签名预提交。slot 只负责提议节奏；round 在超时后推进。三等权验证者是首个可验证的运行基线，不构成 `f=1` Byzantine 容错声明；四等权验证者才是该声明的最小常规拓扑。

## 2. 边界与不变量

- 一个全局 canonical order 和一个权威 `world_id` 历史是默认拓扑；区域只是逻辑作用域，不能独立最终化。未来分片必须先证明跨域收敛。
- chain 保存顺序、commitment、验证者转换和最终性证明；DistFS/CAS 保存 hash-bound 的 snapshot、blob 和历史可用性材料，永不自行取得最终写入权。
- 验证者为治理注册、可轮换的受保护节点；公开 sentry/relay、full/state-sync/archive、RPC/proof gateway 可无许可运行。服务节点被攻破不能产生共识权威。
- 验证者保留权威恢复/投票窗口；full/state-sync 节点提供较长热历史和快照；archive 保存完整审计历史；light companion 保留 finalized header、验证者转换和所需证明。任何 pruning 仅在可重建、hash/root 验证和冗余 archive 可用被证明后进行。
- bootstrap/recovery 必须依序绑定 immutable tier/genesis manifest、finalized checkpoint certificate、hash-bound snapshot、canonical committed-log replay 和 verified state root。任一身份、签名、连续性、hash、replay 或 root 不匹配均 fail closed。

## 3. 运行与经济边界

- 终端安全模型中，验证者使用 stake、奖励和客观可证明的共识故障 slashing；非权威服务使用可验证 usage/availability receipt 与市场费用或治理 grant。基础层拥有计量、证明、锁定与确定性结算 hook，不拥有奖励率、费用、补贴预算或资格数值。
- 基础层可选择性采用成熟库并借鉴公开链架构，但所有共识、状态、治理、receipt、replay 与恢复语义仍由 oasis7 自己拥有；第三方依赖必须保持可替换。

## 4. 当前与目标的分离

当前实现是 stake-weighted proposer/attestation threshold prototype，不是已经具备完整 BFT 最终性的公开承诺。目标仍缺持久且可复验的 quorum certificate、prevote/precommit 锁定、round timeout/view-change、验证者转换证明、复制端证书复验与对抗性恢复证据。本文不因目标描述而宣称 mainnet、去中心化规模、SLA 或发行 readiness。

## 5. 组合验收

- DC-1：任何服务、full node 或 light companion 都只能从已验证的最终性证明和 hash-bound 材料得出世界状态；非权威 peer/缓存/快照不能代签。
- DC-2：分区、重启、落后追赶、恢复和 pruning 样例证明相同 `world_id` 的唯一顺序、可重建性与 state-root 一致；不满足证据时停止服务或投票。
- DC-3：验证者注册/轮换、网络暴露和服务角色不扩大非权威节点的共识权限。
- DC-4：BFT 实现样例证明超过三分之二活动质押预提交形成可验证 commit certificate，且 equivocation、缺证、错误验证者集合和 round 故障均不得推进权威历史。

## 6. Non-Goals

- 不定义区域设施、市场、工业、charter、frontier、普通治理或玩家资源经济；这些是世界规则与核心玩法模块的产品语义。
- 不定义 deterministic world runtime 的规则解释；该上层基础子层由本模块的执行专题和 `doc/world-runtime/` 专业权威承载。
- 不定义共识/存储/网络实现或当前运维、发布与公开状态。
