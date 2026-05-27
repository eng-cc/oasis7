# oasis7 Runtime：分布式存储去单机完整依赖改造（项目管理）（项目管理文档）

- 对应设计文档: `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.design.md`
- 对应需求文档: `doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.prd.md`

审计轮次: 5
## 任务拆解（含 PRD-ID 映射）
### T0 建档
- [x] 设计文档 (PRD-P2P-MIG-065)：`doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.prd.md`
- [x] 项目文档 (PRD-P2P-MIG-065)：`doc/p2p/distfs/distfs-no-single-full-node-assumption-2026-02-23.project.md`

### T1 严格 DHT 拉取
- [x] `fetch_blob_from_dht` 去掉无 provider 回退路径 (PRD-P2P-MIG-065)
- [x] 增补 `oasis7_net` 单测 (PRD-P2P-MIG-065)：无 provider 失败 / provider 重试成功

### T2 覆盖审计与回放接线
- [x] 新增 provider 覆盖审计策略模块 (PRD-P2P-MIG-065)
- [x] 在 DHT 批量拉取入口接入覆盖审计（`DistributedClient (PRD-P2P-MIG-065)::fetch_blobs_from_dht_with_distribution`）
- [x] 补齐单测 (PRD-P2P-MIG-065)：副本不足拒绝、单节点全覆盖拒绝、分布覆盖放行

### T3 收口
- [x] 回归 (PRD-P2P-MIG-065)：`oasis7_net`、`oasis7_distfs`、`oasis7_consensus`、`oasis7_node`
- [x] 更新设计/项目文档状态 (PRD-P2P-MIG-065)
- [x] 追加 `doc/devlog/README.md` 任务日志 (PRD-P2P-MIG-065)

## 依赖
- `crates/oasis7_net/src/client.rs`
- `crates/oasis7_net/src/observer_replay.rs`
- `crates/oasis7_net/src/replay_flow.rs`
- `crates/oasis7_net/src/tests.rs`

## 状态
- 当前状态：`已完成`
- 完成日期：2026-02-23（历史完成，ROUND-005 回填）
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
