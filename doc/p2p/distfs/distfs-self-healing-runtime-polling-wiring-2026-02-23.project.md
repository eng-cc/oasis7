# oasis7 Runtime：分布式存储自愈轮询 Runtime 接线（项目管理）（项目管理文档）

- 对应设计文档: `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.design.md`
- 对应需求文档: `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md`

审计轮次: 5
## 审计备注（ROUND-002 主从口径）
- 本文档为增量子项目文档（slave），主入口为 `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md`。
- 仅维护 Runtime 接线增量任务，不覆盖主项目文档的总任务分解与状态口径。

## 任务拆解（含 PRD-ID 映射）
### T0 建档
- [x] 设计文档 (PRD-P2P-MIG-079)：`doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md`
- [x] 项目文档 (PRD-P2P-MIG-079)：`doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md`

### T1 Runtime 接线实现
- [x] 新增 Node 级副本维护配置模型与校验 (PRD-P2P-MIG-079)
- [x] 增加 Runtime 轮询状态字段与 tick 调用链接线 (PRD-P2P-MIG-079)
- [x] 增加 Node 侧本地目标执行器（网络拉取 + 本地 CAS 写入） (PRD-P2P-MIG-079)
- [x] 补齐单测（启用执行/缺失依赖跳过/非法配置） (PRD-P2P-MIG-079)

### T2 收口
- [x] 回归 (PRD-P2P-MIG-079)：`oasis7_node`、`oasis7_net`、`oasis7_distfs`、`oasis7_consensus`
- [x] 更新设计/项目文档状态 (PRD-P2P-MIG-079)
- [x] 追加 `doc/devlog/README.md` 任务日志 (PRD-P2P-MIG-079)

## 依赖
- `crates/oasis7_node/src/lib.rs`
- `crates/oasis7_node/src/node_runtime_core.rs`
- `crates/oasis7_node/src/types.rs`
- `crates/oasis7_node/src/network_bridge.rs`
- `crates/oasis7_node/src/tests_split_part1.rs`

## 状态
- 当前状态：`已完成`
- 完成日期：2026-02-23（历史完成，ROUND-005 回填）
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：T0、T1、T2
- 进行中：无
- 未开始：无
- 阻塞项：无
