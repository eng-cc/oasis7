# oasis7 Runtime：分布式存储自愈控制面（项目管理）（项目管理文档）

- 对应设计文档: `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.design.md`
- 对应需求文档: `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md`

审计轮次: 5
## ROUND-002 主从口径
- 本文档为 distfs-self-healing 项目管理主入口（master）。
- `distfs-self-healing-polling-loop-2026-02-23.project.md` 与 `distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md` 仅维护各自增量任务与状态。

## 任务拆解（含 PRD-ID 映射）
### T0 建档
- [x] 设计文档 (PRD-P2P-MIG-077)：`doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md`
- [x] 项目文档 (PRD-P2P-MIG-077)：`doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md`

### T1 维护计划生成器
- [x] 新增 `replica_maintenance` 计划模型与策略 (PRD-P2P-MIG-077)
- [x] 实现 Repair/Rebalance 任务规划 (PRD-P2P-MIG-077)
- [x] 补齐单测 (PRD-P2P-MIG-077)：副本不足与负载倾斜场景

### T2 维护执行器
- [x] 增加计划执行接口与执行报告 (PRD-P2P-MIG-077)
- [x] 执行成功后发布 target provider 到 DHT (PRD-P2P-MIG-077)
- [x] 补齐单测 (PRD-P2P-MIG-077)：成功发布索引 / 失败不污染索引

### T3 收口
- [x] 回归 (PRD-P2P-MIG-077)：`oasis7_net`、`oasis7_distfs`、`oasis7_consensus`、`oasis7_node`
- [x] 更新设计/项目文档状态 (PRD-P2P-MIG-077)
- [x] 追加 `doc/devlog/README.md` 任务日志 (PRD-P2P-MIG-077)

## 依赖
- `crates/oasis7_net/src/lib.rs`
- `crates/oasis7_net/src/provider_selection.rs`
- `crates/oasis7_net/src/dht.rs`
- `crates/oasis7_net/src/tests.rs`

## 状态
- 当前状态：`已完成`
- 完成日期：2026-02-23（历史完成，ROUND-005 回填）
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：T0、T1、T2、T3
- 进行中：无
- 未开始：无
- 阻塞项：无
