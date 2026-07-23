# DistFS 分布式韧性（项目与历史追溯）

- 对应需求文档: `doc/p2p/distfs/distfs-distributed-resilience.prd.md`
- 对应设计文档: `doc/p2p/distfs/distfs-distributed-resilience.design.md`

## 任务拆解

| 历史 PRD-ID | 已完成范围 | 保留的验证/证据重点 |
| --- | --- | --- |
| PRD-P2P-MIG-064 | Provider 能力画像兼容、评分排序与定向重试 | 旧记录兼容、排序/重试/候选上限；`oasis7_net`、`oasis7_distfs`、`oasis7_consensus`、`oasis7_node` 回归。 |
| PRD-P2P-MIG-065 | 严格 DHT provider 读取与分布覆盖审计 | 无 provider 失败、重试成功、副本不足/单 provider 全覆盖拒绝和分布覆盖放行。 |
| PRD-P2P-MIG-077 | Repair/Rebalance 计划、执行与 DHT 发布闭环 | 副本不足/负载倾斜计划，成功发布 provider，失败不污染索引。 |
| PRD-P2P-MIG-078 | 维护轮询策略、状态、结果与到期判断 | 首轮执行、未到期跳过、非法策略拒绝。 |
| PRD-P2P-MIG-079 | NodeRuntime 配置、状态、轮询接线与本地目标执行器（M0/M1/M2） | `test_tier_required`：启用执行、缺依赖跳过、非法配置、target/source/payload/hash 校验及轮询错误不阻断主 tick。 |

MIG-079 于 2026-02-23 完成，2026-03-06 经 ROUND-005 字段回填（审计轮次 5）；决策记录为 `DEC-PRD-P2P-MIG-079-001`，选择逐篇语义合并而非直接重命名。

## 依赖

- `crates/oasis7_net/src/{client,provider_selection,provider_distribution,replica_maintenance}.rs`
- `crates/oasis7_node/src/{lib,replica_maintenance_support,node_runtime_core,network_bridge,types}.rs`
- DHT provider 索引、replication runtime/network 与本地 CAS 执行器。

## 状态

上述专题均在 2026-02-23 完成并于 2026-03-06 ROUND-005 回填字段。它们已合并为当前的 `distfs-distributed-resilience` 专业权威；历史 review/audit/archive 中的旧路径保留为 provenance，不恢复为 active 入口。

## 当前验证责任

后续变更应按 PRD 的验证重点运行受影响的 `oasis7_net` 与 `oasis7_node` 测试，并在涉及 DHT 读取、维护计划或 NodeRuntime tick 时补跨 crate 回归。该历史完成记录不替代当前 CI、release、replay/recovery 或 production-readiness 证据。
