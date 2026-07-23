# DistFS 分布式韧性设计

- 对应需求文档: `doc/p2p/distfs/distfs-distributed-resilience.prd.md`
- 对应项目管理文档: `doc/p2p/distfs/distfs-distributed-resilience.project.md`

## 1. 设计定位

把异构 provider 的兼容读取、分布覆盖校验与有界自愈收敛为一个 P2P/Runtime 合同：先以 DHT provider 索引约束读取，再以受配额控制的 repair/rebalance 补足副本，最后由 NodeRuntime 最佳努力地周期触发。

## 2. 分层与数据流

1. **Provider 数据与选择层**：`ProviderRecord` 保持旧字段可缺失；`ProviderSelectionPolicy` 对候选做确定性排序并限制候选数。
2. **严格读取与覆盖层**：`fetch_blob_from_dht` 只向已索引 provider 定向读取；批量读取先通过 `ProviderDistributionPolicy` 审计副本数及全覆盖集中度。
3. **维护控制层**：维护计划把缺副本和负载倾斜转换为有界 `ReplicaTransferTask`；执行报告隔离每项失败，成功后才发布 provider。
4. **轮询与 runtime 层**：`NodeReplicaMaintenanceConfig` 受校验地提供采样、配额、阈值和 interval；`node_runtime_core` 通过可选 DHT 注入与 `network_bridge` 接入轮询。轮询根据 last-polled 时间及 interval 决定是否运行；NodeRuntime 在依赖齐全时接入，缺依赖/无输入跳过，错误写入 `last_error` 而不影响主 tick。

## 3. 一致性与错误原则

- DHT 没有 provider、所有定向请求失败、策略非法或分布审计不通过，均显式失败；绝不以未知的全网读取隐藏违反分布假设的问题。
- 排序、计划输入、任务迭代和报告必须具有稳定可检查的行为；维护失败不更新 provider 索引。执行器拒绝非本地 target，要求指定 source 的 found payload 通过 BLAKE3 hash 校验后才写入 CAS；重复的本地 CAS 写入可幂等，但不等同于全局调度去重。
- `last_polled_at_ms` 是运行时节拍状态，不是持久化 checkpoint；本设计不声明 replay/recovery 的新合同。

## 4. 部署边界

NodeRuntime 的本地目标执行器不定义远端任务委派、跨节点协调或拓扑调度。配置、DHT、复制 runtime、复制网络和采样内容任一缺失时，维护工作跳过；启用后单轮工作量仍由策略配额控制。

## 5. 验证设计

针对兼容画像、稳定排序、严格失败、覆盖拒绝、计划/执行发布闭环、轮询到期性和 runtime skip/non-blocking 行为建立单元及跨 crate 回归。验证这些行为不构成生产拓扑、容量、SLA 或完整自愈 readiness 证明。
