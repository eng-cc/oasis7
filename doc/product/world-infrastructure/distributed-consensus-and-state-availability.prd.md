# 分布式共识与状态可用性

## 文档身份

- 所属产品模块：权威世界基础设施
- 上位产品 PRD：[prd.md](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- Last reviewed：`2026-09-14`
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据）。
- 运维 authority：[`node triad 运维与可观测性`](../../p2p/node/node-triad-operations-observability.prd.md#权威边界)（节点 inventory、topology、health/status 与运维证据边界）、[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#stable-authority-and-evidence-boundary)（deployment truth、恢复/回滚演练与同窗口事实捕获）。
- 状态同步 evidence envelope：[`state-sync closure evidence packet`](../../testing/templates/state-sync-closure-evidence-packet-template.md#claim-boundary)（topology/node truth、peer heads、gap sync、observer catch-up 与 blob closure；`module_full` 证据，不单独证明 readiness）。

本文定义 oasis7 区块链/分布式系统底层向上层确定性世界执行提供的产品级保证。它不定义共识消息、密码学、网络协议、节点配置、存储格式或运行步骤；这些由 P2P、共识和运维专业权威拥有。运维证据的采集与解释由 `blockchain_ops_engineer` 负责，不能单独把模块证据解释为 release、public testnet 或 mainnet readiness。

## 设计适用性与生命周期闭合

- 设计判定：`simple-topic-exemption`（`PRD-only-sufficient`）。
- 设计判定 task issue：#3680。
- 设计适用性理由：本 PRD 只定义 canonical history、可用性、权限隔离和 fail-closed 产品结果；它不声明独立玩家信息架构。
- 当前 GitHub task evidence：本次分类见 [Issue #3680 C4 设计判定](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652452993)，本次闭合要求见 [Issue #3680 accepted repair](https://github.com/eng-cc/oasis7/issues/3680#issuecomment-5652870280)。
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

## 4. 可用性等级、消费者反馈与 fail-closed 边界

共识、复制和存储层的“可达”不等于消费者可以把读到的字节当作当前世界或提交新行动。为避免节点、缓存和入口各自发明健康含义，产品层将可用性收敛为以下四种消费者语义；它们不是新的玩家访问模式，也不冻结实现字段或内部枚举。

- **已验证可服务**：`world_id`、最新可验证 checkpoint/head、hash-bound 状态和适用的最终性/追加条件均成立。消费者可以观察 committed state，并按对应专业合同提交新的 intent；“可提交”仍不等于该 intent 已产生世界效果。
- **已验证只读**：同一 `world_id` 的 checkpoint/snapshot、canonical replay 和 state root 可验证，但当前追加、最终性或服务闸门尚未成立。消费者可以查看最后一个已验证状态、历史 receipt 与仍无世界效果的 pending；新的 intent 必须原子拒绝或保持明确的无效果待决，不得取得资源、资格、排队优先或隐性完成承诺。
- **陈旧/追赶中**：副本能证明自己属于同一世界，但其 head 或可用性材料落后于当前权威，或 freshness 尚未证明。只读内容必须标记陈旧/追赶中及最后可信边界；不能把陈旧状态用于声称当前权限、价格、容量、治理结果或行动成功。依赖新鲜条件的 intent 只能等待重新验证、重新评估或被 fail closed。
- **不可用/隔离**：`world_id`、checkpoint、hash/root、canonical 连续性或证明材料缺失、冲突、回退或指向其他世界。消费者只能看到最后一个已验证 receipt（如有）及“原世界不可用/待恢复”说明；不得把缓存、部分快照、替代 endpoint 或本地开发世界表述为当前世界，也不得接受新的权威 intent。

“已验证可服务”还必须满足当前 governing runtime manifest/version 的 compatibility declaration 已能对应到 committed/finality-verified canonical execution block，且 head continuity 单调可验证；客户端提交的 version 不能自行选择世界规则。manifest、compatibility 或 head continuity 证据缺失、冲突、未知或非单调时，不得保持可服务语义，必须降级为已验证只读或不可用/隔离，并原子拒绝或保留无效果待决；每个受影响新 intent 的 committed receipt 必须为 `0`。版本激活与 governing version 的具体边界见[`确定性世界执行`](deterministic-world-execution.prd.md)第 2.2 节及其 DE-1/DE-4。

每次从一种语义降级或恢复到另一种语义，正式消费者都必须能读到当前状态、影响的操作范围、最后可信边界（若可提供）、主要 blocker 和真实下一步（等待、重新验证、重新进入、重新规划或安全返回）。状态变化不能依靠页面刷新、进程存活、重连成功、历史提交回执或本地缓存静默升级；恢复只读、恢复可服务和恢复受阻的组合闸门仍以本模块根 PRD 的同名语义为准。

以下边界必须保持一致：

- 网络分区、lagging replica、proof gateway 暂时不可达或 archive 追赶时，不能选择“看起来最新”的副本继续写入；只有重新取得适用证明后才能升级状态。
- 状态字节可读但 proof/root 校验失败时，视为不可验证而非“可服务”；不得以 transport green、HTTP 成功或节点存活替代权威证明。
- 两个候选 snapshot/checkpoint 互相冲突时必须隔离并报告冲突，不得在产品层合并、择一或把其中一个当作新世界；同一 `world_id` 的已确认 receipt 仍保持原历史。
- 重连、重复提交和跨入口重试都要重新评估当前等级。缓存的“已发送/处理中”只保留请求关联，不产生第二次世界效果，也不能把无效果 pending 表述为已结算。

本节定义的是跨消费者的产品结果与反馈语义，具体合同按以下入口分工：P2P 负责共识最终性、证明与 freshness；world-runtime 负责版本化执行 manifest/compatibility、head continuity、服务闸门、pending 持久化、去重与 receipt；world-simulator 负责消费者/Viewer/入口的状态与 UI/API 反馈投影；testing 负责组合矩阵、验证命令与证据。产品层不复制这些专业字段、错误码或实现。

Compatibility declaration 只证明客户端能理解当前 manifest，不能选择或锁定 governing version；manifest 缺失、冲突或无法与当前 committed/finality-verified block 对账时，前述可服务语义不成立。

## 4.1 正常路径与消费者选择

正常路径从消费者读取同一 `world_id` 的已验证状态开始：消费者确认当前等级、最后可信边界和适用的版本/最终性条件，再选择等待、重新验证或提交一个 intent。只有在“已验证可服务”成立时才进入专业合同定义的提交路径；提交后仍须等待可验证的 committed receipt，消费者再据此更新自己的世界结论。已验证只读、陈旧/追赶中或不可用/隔离时，路径停留在查看历史、等待恢复、重新规划或安全返回，不把本地排队和界面反馈包装成世界效果。

该选择的代价是明确的：等待或重新规划可能延迟行动，但提交未经证明的状态会造成重复、错误授权或把临时读面误认为权威历史。基础设施产品层只规定这种可观察的选择和风险边界；提交顺序、去重、receipt 字段和具体恢复步骤仍由 P2P、world-runtime、消费者与测试专业权威定义。

## 5. 当前与目标的分离

当前实现是 stake-weighted proposer/attestation threshold prototype，不是已经具备完整 BFT 最终性的公开承诺。目标仍缺持久且可复验的 quorum certificate、prevote/precommit 锁定、round timeout/view-change、验证者转换证明、复制端证书复验与对抗性恢复证据。本文不因目标描述而宣称 mainnet、去中心化规模、SLA 或发行 readiness。

## 6. 叶子需求与可观察验收

这些叶子把本专题的五类可独立失败义务映射到现有 DC 汇总标准；它们保留 P2P、runtime、blockchain ops 和 QA 的专业 authority，不把证书、拓扑或节点运行步骤复制到产品层。

<a id="req-dcs-001"></a>
### REQ-DCS-001：单一 canonical history 与最终性

同一 `world_id` 只能由适用的、可验证的 finality/commit certificate 推进唯一 canonical order；副本、缓存、非权威 peer 或未最终化输入不能成为玩家世界结果。

- 对应验收：[AC-DCS-001](#ac-dcs-001)。
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)。

<a id="ac-dcs-001"></a>
### AC-DCS-001：非权威材料不能代签

同一候选的服务节点、full node、light companion 和消费者只能依据验证过的最终性证明及 hash-bound 材料得出世界状态；冲突、缺证或非权威写入被拒绝且不进入玩家可见历史。证据必须覆盖适用的 P2P/runtime 组合边界，transport 成功不能代签。

- 对应需求：[REQ-DCS-001](#req-dcs-001)。

<a id="req-dcs-002"></a>
### REQ-DCS-002：状态可用性与恢复链连续

恢复只能沿同一 `world_id` 的 genesis/manifest、finalized checkpoint certificate、hash-bound snapshot、canonical replay 和 verified state root 建立连续历史；材料缺失、冲突、回退或指向其他世界时必须保持隔离或只读。

- 对应验收：[AC-DCS-002](#ac-dcs-002)。
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)、[`node triad 运维与可观测性`](../../p2p/node/node-triad-operations-observability.prd.md#inventory-与采样合同)、[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#stable-authority-and-evidence-boundary)、[`state-sync closure evidence packet`](../../testing/templates/state-sync-closure-evidence-packet-template.md#topology-and-node-truth)。
- 证据层级与 owner：`test_tier_full`；`blockchain_ops_engineer` 负责同一证据窗口的 topology/inventory、`/healthz` 与 `/v1/chain/status`、peer-head/replication、state-sync closure 及 restore/rollback 事实，`runtime_engineer` 负责 replay/state-root 连续性，QA 负责 verdict。缺少任一适用证据时保持隔离或只读，不得解释为 readiness。

<a id="ac-dcs-002"></a>
### AC-DCS-002：分区、重启与恢复不产生第二世界

分区、重启、落后追赶、snapshot/state sync 或 pruning 样例能证明同一 `world_id` 的唯一顺序、可重建性和 state-root 连续；任一恢复材料不匹配时停止服务/投票或降为明确只读，不能把替代 endpoint、缓存或本地世界包装成原世界恢复。

恢复验收还覆盖根 [`prd.md`](prd.md) 的 SC-7：恢复只读、恢复可服务、恢复受阻/隔离和闸门回退。相同 `world_id` 的历史验证链完整只允许恢复只读；只有当前追加/最终性、版本化执行和 head 单调连续性的专业证据全部重新成立，才可恢复可服务。其余状态下新 intent 的 committed receipt 必须为 `0`；进入可服务后，既有 pending 按当前有效条件和 canonical 顺序重新裁决，已确认 receipt 保持不变。消费者能读到当前恢复语义、主要 blocker 和真实下一步。测试层级：`test_tier_full`。

- 对应需求：[REQ-DCS-002](#req-dcs-002)。

- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)、[`node triad 运维与可观测性`](../../p2p/node/node-triad-operations-observability.prd.md#权威边界)、[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#4-hard-rules)、[`state-sync closure evidence packet`](../../testing/templates/state-sync-closure-evidence-packet-template.md#peer-heads-and-gap-sync)。
- 证据层级与 owner：`test_tier_full`；`blockchain_ops_engineer` 负责同一窗口的 topology/inventory、health/status、peer-head/replication、state-sync 与 restore/rollback 事实，`runtime_engineer` 负责 replay/state-root 连续性，QA 负责 verdict。缺少任一适用证据时保持隔离或只读，不得解释为 readiness。
- 运维证据边界：[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#4-hard-rules) 的 deployment truth 与恢复约束，以及 [`state-sync closure evidence packet`](../../testing/templates/state-sync-closure-evidence-packet-template.md#peer-heads-and-gap-sync) 的 peer-head、gap sync、observer catch-up 与 blob closure 字段；产品层只消费同一窗口的结果，不复制运行步骤。

<a id="req-dcs-003"></a>
### REQ-DCS-003：共识权限与服务角色隔离

验证者集合及其轮换保有受治理的共识权威；sentry、relay、full/state-sync/archive、RPC/proof gateway 等非权威服务只能提供传播、存储或证明材料，不能通过暴露面、缓存或服务被攻破取得最终写入权。

目标态参与边界：公网 IP 不是受治理 validator 或 permissionless full/storage/observer service role 参与网络的通用前置条件；治理 admission/activation 仅适用于 validators，full/storage/observer 等服务角色按本地 role policy 和技术可达性参与。该产品目标对应 [P2P reachability PRD §2 的首项验收标准](../../p2p/network/mainnet-private-reachability-architecture.prd.md#2-user-experience-functionality)，不表示当前软件支持、已有部署能力，也不证明 mainnet 或 release readiness。

- 对应验收：[AC-DCS-003](#ac-dcs-003)。
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)、[`node triad 运维与可观测性`](../../p2p/node/node-triad-operations-observability.prd.md#权威边界)、[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#stable-authority-and-evidence-boundary)。

<a id="ac-dcs-003"></a>
### AC-DCS-003：非权威节点不能扩大权限

验证者注册/轮换、网络暴露和服务角色的组合样例证明非权威节点不能推进 canonical history、签发最终性或代替受治理验证者完成高影响状态变化；被攻破的服务节点只能导致隔离/缺证等可观察结果，不能产生权威写入。

- 对应需求：[REQ-DCS-003](#req-dcs-003)。

- 证据层级与 owner：`test_tier_full`；`blockchain_ops_engineer` 负责同一 topology/inventory 与 health/status 窗口中的 validator/service-role 隔离、网络暴露和停止 serving/voting 的运维证据，P2P 保有权限/最终性语义，QA 负责 verdict。该映射只证明 fail-closed 结果，不证明部署或发行 readiness。

<a id="req-dcs-004"></a>
### REQ-DCS-004：commit certificate 的安全推进边界

目标 BFT commit 只有在活动、治理注册的验证者集合满足产品要求的最终性证据时才能推进；equivocation、缺证、错误验证者集合或 round 故障不得产生部分或可见权威历史。

- 对应验收：[AC-DCS-004](#ac-dcs-004)。
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)。

<a id="ac-dcs-004"></a>
### AC-DCS-004：错误证明和 round 故障 fail closed

BFT 实现样例证明符合产品最终性条件的 commit certificate 才能推进历史；equivocation、缺证、错误验证者集合和 round 故障全部被拒绝或保持无效果，不以 prototype 状态宣称完整 BFT readiness。具体阈值、签名和 round 状态由 P2P authority 定义。

- 对应需求：[REQ-DCS-004](#req-dcs-004)。

<a id="req-dcs-005"></a>
### REQ-DCS-005：消费者可读的可用性与 fail-closed

只有“已验证可服务”同时满足 `world_id`、checkpoint/head、hash-bound 状态、finality/追加条件、manifest compatibility 和单调 head continuity 时，消费者才可提交新的权威 intent；其他等级必须保持只读、无效果待决或不可用/隔离，并说明 blocker 与下一步。

- 对应验收：[AC-DCS-005](#ac-dcs-005)。
- 专业域权威：[`doc/p2p/prd.md`](../../p2p/prd.md)、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)、[`node triad 运维与可观测性`](../../p2p/node/node-triad-operations-observability.prd.md#inventory-与采样合同)、[`public-testnet governed bootstrap runbook`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md#stable-authority-and-evidence-boundary)、[`state-sync closure evidence packet`](../../testing/templates/state-sync-closure-evidence-packet-template.md#peer-heads-and-gap-sync)。

<a id="ac-dcs-005"></a>
### AC-DCS-005：状态转换不伪造世界效果

同一候选覆盖“已验证可服务 -> 陈旧/追赶中 -> 已验证只读 -> 已验证可服务”和证明冲突进入“不可用/隔离”的转换；非可服务等级的新 intent 的 committed receipt 为 `0`，正式消费者能读到当前等级、受影响操作、最后可信边界、blocker 和真实下一步。重连、重复提交和跨入口重试不产生第二次世界效果。

- 对应需求：[REQ-DCS-005](#req-dcs-005)。

- 证据层级与 owner：`test_tier_full`；`blockchain_ops_engineer` 负责 topology、inventory、`/healthz`、`/v1/chain/status`、peer-head/replication 与 state-sync/恢复窗口的事实捕获，runtime/P2P 负责执行、manifest/head 与 finality 语义，world-simulator 负责消费者投影，QA 负责 verdict。health endpoint、transport green 或一次重连不能单独升级可服务状态。

## 7. 组合验收

- DC-1：任何服务、full node 或 light companion 都只能从已验证的最终性证明和 hash-bound 材料得出世界状态；非权威 peer/缓存/快照不能代签。
- DC-2：分区、重启、落后追赶、恢复和 pruning 样例证明相同 `world_id` 的唯一顺序、可重建性与 state-root 一致；不满足证据时停止服务或投票。
- DC-3：验证者注册/轮换、网络暴露和服务角色不扩大非权威节点的共识权限。
- DC-4：BFT 实现样例证明超过三分之二活动质押预提交形成可验证 commit certificate，且 equivocation、缺证、错误验证者集合和 round 故障均不得推进权威历史。
- DC-5：同一候选至少覆盖“已验证可服务 -> 陈旧/追赶中 -> 已验证只读 -> 已验证可服务”以及证明冲突进入“不可用/隔离”的转换；仅在已验证可服务且当前 governing manifest/compatibility 与单调 head continuity 证据均成立时接受新的权威 intent，其他等级或版本/连续性证据缺失、冲突时新 intent 的 committed receipt 数为 `0`。陈旧内容带有最后可信边界与 blocker，不被表述为当前权限、价格或成功；冲突/替代世界不被当作原世界恢复；重连、重复提交和跨入口重试不产生第二次世界效果。正式消费者能读到当前等级、受影响操作与真实下一步。测试层级：`test_tier_full`。证据追踪必须将根产品 PRD 的 SC-7 恢复闸门与 [`Game World State Sync and Commit Closure` 计划](../../testing/longrun/game-world-state-sync-commit-closure-2026-06-26.prd.md) 作为执行 lane，将 [`state-sync closure evidence packet` 模板](../../testing/templates/state-sync-closure-evidence-packet-template.md) 作为 evidence envelope，并在同一候选的 linked structured attachment/schema 中提供状态转换、manifest/head 负例、每个 intent 的 receipt `0/1` 与消费者 blocker/next-step 字段；缺少该 attachment/schema 或任一字段时，DC-5 不得判定通过。

## 8. Non-Goals

- 不定义区域设施、市场、工业、charter、frontier、普通治理或玩家资源经济；这些是世界规则与玩法系统模块的产品语义。
- 不定义 deterministic world runtime 的规则解释；该上层基础子层由本模块的执行专题和 `doc/world-runtime/` 专业权威承载。
- 不定义共识/存储/网络实现或当前运维、发布与公开状态。

## 全量语义追踪

| REQ / AC | 专业 owner | 专业权威 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| [REQ-DCS-001](#req-dcs-001) / [AC-DCS-001](#ac-dcs-001) | `producer_system_designer` | [`doc/p2p/prd.md`](../../p2p/prd.md#目标)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据） | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-DCS-002](#req-dcs-002) / [AC-DCS-002](#ac-dcs-002) | `producer_system_designer / blockchain_ops_engineer / runtime_engineer / viewer_engineer / qa_engineer` | [`doc/p2p/prd.md`](../../p2p/prd.md#目标)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据） | 根产品 SC-7 的恢复只读/可服务/受阻或隔离与闸门回退矩阵；历史链完整只允许只读，可服务须追加/最终性、版本化执行和 head 连续性证据同时成立；未满足时新 intent 的 committed receipt 为 0，pending 按当前条件重裁决且既有 receipt 不变；同一候选的 topology/state-sync/restore、replay/state-root 与消费者 blocker/下一步证据 | `test_tier_full` |
| [REQ-DCS-003](#req-dcs-003) / [AC-DCS-003](#ac-dcs-003) | `producer_system_designer` | [`doc/p2p/prd.md`](../../p2p/prd.md#目标)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据） | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-DCS-004](#req-dcs-004) / [AC-DCS-004](#ac-dcs-004) | `producer_system_designer` | [`doc/p2p/prd.md`](../../p2p/prd.md#目标)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据） | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
| [REQ-DCS-005](#req-dcs-005) / [AC-DCS-005](#ac-dcs-005) | `producer_system_designer` | [`doc/p2p/prd.md`](../../p2p/prd.md#目标)（共识最终性、证明与 freshness）、[`doc/world-runtime/prd.md`](../../world-runtime/prd.md)（版本化执行、manifest/head、pending、receipt 与去重）、[`doc/world-simulator/prd.md`](../../world-simulator/prd.md)（消费者/Viewer/入口状态反馈）、[`doc/testing/prd.md`](../../testing/prd.md)（组合验证与证据） | 本专题对应要求、验收与专业 authority 的可导航追踪证据 | `test_tier_required` |
