# README P1 缺口收口：分布式网络主路径生产化设计

- 对应需求文档: `doc/p2p/network/readme-p1-network-production-hardening.prd.md`
- 对应GitHub Issue/Project task truth: GitHub Issue / GitHub Project

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
