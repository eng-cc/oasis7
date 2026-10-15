# README P1 缺口收口：分布式网络主路径生产化设计

- 对应需求文档: `doc/p2p/network/readme-p1-network-production-hardening.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project


## 目标、上游约束与适用范围

本设计的专业输入是配对 [PRD request 路由](readme-p1-network-production-hardening.prd.md#1-libp2p-request-路由)、[共识 topic](readme-p1-network-production-hardening.prd.md#2-共识-topiclibp2p) 与[目标平台约束](readme-p1-network-production-hardening.prd.md#4-oasis7net-与目标平台边界)。P2P/runtime owner 维护 transport/request 与 node bridge；共识 authority 决定消息是否有效，Viewer 只消费观测结果，QA 核对 required 回归和拓扑长跑。

本轮仅归整已有路由、默认拒绝和兼容边界，未修改运行行为；当前实现/验收以实际候选测试为准。PRD 已记载 wasm32 package-level check 的依赖阻断，本设计不把 API/stub 目标改写为已通过。

## 1. 设计定位
定义分布式网络 P1 主路径生产化方案：把请求层升级为 libp2p 多 peer 轮换重试，并把共识消息切到 libp2p pubsub 主链路。

## 2. 设计结构
- 请求路由层：`libp2p request/response` 支持多 peer 轮换重试；远端 `ErrorResponse` 计为当前 peer 请求失败并继续轮换，耗尽后返回 `NetworkRequestFailed`；无 peer 时按开关决定是否回退本地 handler。
- 共识主链路层：proposal / attestation / commit 使用 libp2p pubsub 主题作为生产默认主路径。
- 兼容兜底层：UDP gossip 继续保留为兼容链路，与 libp2p ingest 并存。
- 目标平台层：native `oasis7_node` 单向依赖 `oasis7_net`；wasm32 通过 target cfg 使用保持 API 形状的 unavailable stub，不承诺完整 node/libp2p runtime。
- 稳定性回归层：围绕 node 网络桥接与共识广播路径维持 required-tier 回归稳定。

## 3. 关键接口 / 入口
- `Libp2pReplicationNetworkConfig::allow_local_handler_fallback_when_no_peers`
- `aw.<world_id>.consensus.proposal`
- `aw.<world_id>.consensus.attestation`
- `aw.<world_id>.consensus.commit`
- `crates/oasis7_node/src/libp2p_replication_network.rs`
- `crates/oasis7_node/src/network_bridge.rs`
- `WorldError::NetworkProtocolUnavailable`

## 4. 约束与边界
- 生产默认链路优先走 libp2p，UDP 仅作兼容兜底。
- 无 peer 时默认拒绝 request，只有显式开关开启才允许本地 handler 回退。
- 双路径并存阶段必须依赖既有幂等守卫避免重复消息副作用。
- `oasis7_net` 不得反向依赖 node；wasm stub 的所有网络动作必须结构化失败，不能静默转成本地成功路径。

## 5. 设计演进计划
- 先冻结设计与执行计划。
- 再完成 request 路由升级与共识 topic 接线。
- 最后执行 node 定向回归并收口文档与日志。

## 需求承接与验证范围

承接表只列上述既有设计实际承担的子义务，不改变产品功能状态，也不宣称覆盖产品要求的全部验收。验证表是方法与证据目标，不是本次运行结果。

### 2.1 需求承接与分配表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 具体 obligation 与适用条件 | 本设计条款（path#anchor） | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005)；Trace refs: O7-REQ-ee563883-fe1d-4192-b58b-53251d32920b | 无 peer 默认结构化拒绝；仅显式配置可本地回退，网络不可用不能静默包装为远端成功 | [设计条款](#4-约束与边界) | P2P request layer；runtime/消费者仍负责产品可用性反馈 | 只承接 transport 子边界，不覆盖完整四态反馈、pending 或 receipt |
| [REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005)；Trace refs: O7-REQ-ee563883-fe1d-4192-b58b-53251d32920b | 远端 ErrorResponse 触发 peer 轮换，耗尽结构化失败；重试不把本地 handler 当默认成功 | [设计条款](#2-设计结构) | P2P request routing；消费者解释错误 | 不证明应用层幂等或已提交请求结果确认 |
| [REQ-DCS-001](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-001)；Trace refs: O7-REQ-0bb14512-622e-48fc-879f-bd9f00127b39 | 共识 topic 传递 proposal/attestation/commit，消息传播本身不获得最终性权威；生产默认 libp2p 与 UDP 兼容 ingest 保持既有幂等守卫 | [设计条款](#3-关键接口-入口) | node bridge 与共识 authority；QA 拓扑测试 | 本条只支持传播与一致性回归，不覆盖完整提交证明或持久性 |

### 11.1 验证映射表

| 上游 requirement / product AC / professional acceptance（path#fragment） | 本设计条款（path#anchor） | 独立 obligation 与适用条件 | 准确验证方法、test/manual source 或 ID、scenario/layer、candidate/environment 要求或选择规则 | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | [设计条款](#4-约束与边界) | 无 peer 默认结构化拒绝；仅显式配置可本地回退，网络不可用不能静默包装为远端成功 | [定向测试选择](../../../testing-manual.md#testing-default-selection-tree)；源码 `crates/oasis7_node/src/libp2p_replication_network/tests.rs`：同候选 native/libp2p 下执行 `libp2p_replication_network_request_rejects_without_connected_peers_by_default` 和 `libp2p_replication_network_request_falls_back_to_local_handler_when_enabled` | 定向 cargo test 输出及 feature/target 配置 | 只承接 transport 子边界，不覆盖完整四态反馈、pending 或 receipt；本轮未运行行为测试 |
| [REQ-DCS-005](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-005) | [设计条款](#2-设计结构) | 远端 ErrorResponse 触发 peer 轮换，耗尽结构化失败；重试不把本地 handler 当默认成功 | [定向测试选择](../../../testing-manual.md#testing-default-selection-tree)；源码 `crates/oasis7_node/src/libp2p_replication_network/tests.rs`：同候选 native/libp2p 执行 `libp2p_replication_network_request_round_robins_across_connected_peers`（两个成功 peer 的轮换）、`libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails`（一失败、一成功 peer，最终返回成功），以及 `libp2p_replication_network_preserves_remote_unsupported_error_code`（单 peer 连续两次请求均断言 `NetworkRequestFailed` 的 `ErrUnsupported` code） | 定向测试输出与上述分支断言；不代签世界效果 | 多 peer 全部返回 ErrorResponse 后耗尽及错误选择组合仍未证明；单 peer 测试未断言 retryable/message，不能外推多 peer 耗尽；不证明应用层幂等或已提交请求结果确认；本轮未运行行为测试 |
| [REQ-DCS-001](../../product/world-infrastructure/distributed-consensus-and-state-availability.prd.md#req-dcs-001) | [设计条款](#3-关键接口-入口) | 共识 topic 传递 proposal/attestation/commit，消息传播本身不获得最终性权威；生产默认 libp2p 与 UDP 兼容 ingest 保持既有幂等守卫 | [testing-manual S9](../../../testing-manual.md#testing-p2p-storage-consensus-long-running)；同候选 triad/triad_distributed 使用 S9 对应 profile，核对 consensus_hash_consistent 与 mismatch count；完整最终性仍按 P2P authority 验收 | longrun summary、拓扑配置及 consensus mismatch artifact | 本条只支持传播与一致性回归，不覆盖完整提交证明或持久性；本轮未运行行为测试 |

## 决策、风险与未覆盖范围

保留 UDP 兼容路径降低迁移风险，但双 ingest 必须依赖既有高度/哈希幂等守卫；libp2p 默认路径不改变共识权威。多 peer 重试仍有 pending 迁移和时序风险，runtime/P2P owner 在请求失败分类、topic 或目标平台依赖变更时复核回归。wasm stub 只承诺 API 形状与结构化 unavailable；完整 wasm32 编译和跨平台行为按 PRD 记录的阻断继续单独核验。这里没有新增部署、回滚命令或性能阈值，S9 成功也不单独证明生产 readiness。
