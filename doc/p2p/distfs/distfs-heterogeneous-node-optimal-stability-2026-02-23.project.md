# oasis7 Runtime：异构节点分布式存储最优稳定性改造（项目管理）（项目管理文档）

- 对应设计文档: `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.design.md`
- 对应需求文档: `doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
### T0 建档
- [x] 设计文档 (PRD-P2P-MIG-064)：`doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.prd.md`
- [x] 项目文档 (PRD-P2P-MIG-064)：`doc/p2p/distfs/distfs-heterogeneous-node-optimal-stability-2026-02-23.project.md`

### T1 Provider 能力画像扩展
- [x] 扩展 `ProviderRecord` 可选能力字段并保持 serde 向后兼容 (PRD-P2P-MIG-064)
- [x] 更新 `oasis7_net` / `oasis7_consensus` 相关 DHT provider 构造路径 (PRD-P2P-MIG-064)
- [x] 补齐兼容测试 (PRD-P2P-MIG-064)

### T2 评分排序与重试拉取
- [x] 新增 provider 评分策略模块（权重 + 归一化） (PRD-P2P-MIG-064)
- [x] `DistributedClient (PRD-P2P-MIG-064)::fetch_blob_from_dht` 升级为排序后逐 provider 重试
- [x] 补齐 `oasis7_net` 单测（排序、重试、回退） (PRD-P2P-MIG-064)

### T3 收口
- [x] 运行回归 (PRD-P2P-MIG-064)：`oasis7_net`、`oasis7_distfs`、`oasis7_consensus`、`oasis7_node`
- [x] 更新设计/项目文档状态 (PRD-P2P-MIG-064)
- [x] 追加 `doc/devlog/README.md` 任务日志 (PRD-P2P-MIG-064)

## 依赖
- `crates/oasis7_proto/src/distributed_dht.rs`
- `crates/oasis7_net/src/client.rs`
- `crates/oasis7_net/src/dht.rs`
- `crates/oasis7_net/src/dht_cache.rs`
- `crates/oasis7_net/src/provider_cache.rs`
- `crates/oasis7_net/src/libp2p_net.rs`
- `crates/oasis7_net/src/tests.rs`
- `crates/oasis7_consensus/src/dht.rs`

## 状态
- 当前状态：`已完成`
- 完成日期：2026-02-23（历史完成，ROUND-005 回填）
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
