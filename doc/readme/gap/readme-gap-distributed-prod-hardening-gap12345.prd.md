# README 分布式计算与存储生产级收口（Gap 1/2/3/4/5）设计文档

- 对应设计文档: `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.design.md`
- 对应GitHub Issue/Project task truth: `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md`

审计轮次: 4

## ROUND-002 主从口径
- 本文件为 readme/gap 主文档。
- 其余 gap 专题文档为增量子文档。

- 对应标准执行入口: `doc/readme/gap/readme-gap-distributed-prod-hardening-gap12345.prd.md`

## 目标
- 收口 Gap 1：将 `oasis7_node` 的 PoS 主循环关键算法与阈值判定下沉到 `oasis7_consensus`，避免双轨共识语义漂移。
- 收口 Gap 2：将存储挑战共识门控从“单点网络比对”升级为“多样本网络挑战 + 匹配阈值”模型，提升抗网络抖动与抗单点异常能力。
- 收口 Gap 3：将 replication gap sync 从“首错即停”升级为“分高度重试 + 可观测错误”的稳健同步路径。
- 收口 Gap 4：将 DistFS sidecar 恢复失败从“静默回退”升级为“可审计回退”，保留 JSON 兜底的同时输出失败审计证据。
- 收口 Gap 5：将 `triad_distributed` 从“全量手工静态 peer 配置”升级为“最小引导 + 自动发现”拓扑，降低跨机部署运维复杂度。

## 范围
- In scope
  - `crates/oasis7_consensus`
    - 暴露可复用 PoS 判定内核（阈值状态、提议者选择）供 node 主循环调用。
  - `crates/oasis7_node`
    - 复用 consensus PoS 内核，收敛本地重复算法。
    - replication request 选择策略增强（peer 轮换 / provider 优先）。
    - 存储挑战门控升级为多样本验证与最小匹配阈值。
    - gap sync 升级为按高度重试与错误上报。
    - UDP gossip 自动发现对端地址并动态加入广播 peers。
  - `crates/oasis7`
    - `oasis7_viewer_live` 的 `triad_distributed` 参数与启动路径支持“最小引导拓扑”。
  - `crates/oasis7/src/runtime/world/persistence.rs`
    - DistFS 恢复失败审计记录落盘。
  - 测试
    - `oasis7_node` / `oasis7_consensus` / `oasis7_viewer_live` / runtime persistence 的 required-tier 回归。
- Out of scope
  - wasm32 完整分布式节点协议栈。
  - 共识算法从 PoS 切换到全新协议。
  - 完整自动运维编排平台（仅提供最小可生产自动发现能力）。

## 接口 / 数据
### 1) Node 复用 Consensus PoS 内核
- 新增（或扩展）`oasis7_consensus::distributed_pos_consensus` 导出：
  - `decide_pos_status(total_stake, required_stake, approved_stake, rejected_stake)`
- `oasis7_node::PosNodeEngine`：
  - 提议者选择与 epoch 计算统一走 consensus 内核。
  - supermajority 阈值判定统一走 consensus 内核。

### 2) 存储挑战门控（多样本）
- 新增门控策略（node 内部常量配置）：
  - `network_samples_per_check`（默认 3）
  - `min_successful_matches`（默认按可用样本动态计算：`min(2, available_successes)`，至少 1）
- 校验规则：
  - 仅统计哈希匹配且字节匹配本地 blob 的响应为 success。
  - success 数低于阈值时拒绝提交复制。

### 3) Gap Sync 重试与可观测性
- 每个缺口高度同步新增：
  - 最大尝试次数（默认 3）
  - 最后失败原因保留并上抛到 runtime `last_error`。
- 在“未找到目标高度”场景不再视为致命错误，保持增量同步语义。

### 4) DistFS 回退审计
- 新增审计文件：
  - `<world_dir>/distfs.recovery.audit.json`
- 记录字段：
  - `timestamp_ms`
  - `status` (`distfs_restored` / `fallback_json`)
  - `reason`（失败时包含错误摘要）

### 5) Triad Distributed 最小引导拓扑
- `triad_distributed` 参数语义升级：
  - 仅强制要求当前角色 bind 地址。
  - storage/observer 角色至少提供 sequencer 引导地址。
  - sequencer 可零静态 peer 启动。
- UDP gossip 动态发现：
  - 从入站报文源地址自动学习 peer 并加入后续广播列表。

## 里程碑
- M1：T0 文档冻结（设计 + 项管）。
- M2：T1 共识内核复用收口（Gap 1）。
- M3：T2 存储挑战门控增强（Gap 2）。
- M4：T3 Gap Sync 稳健化（Gap 3）。
- M5：T4 DistFS 回退审计化（Gap 4）。
- M6：T5 分布式拓扑自动发现（Gap 5）。
- M7：T6 回归验证与文档/devlog 收口。

## 风险
- 行为兼容风险：共识阈值算法统一后，历史依赖本地实现细节的测试可能需要更新断言。
- 可用性风险：挑战门控阈值过严可能在弱网络下降低提交吞吐，需要动态阈值与错误观测。
- 运维风险：自动发现机制引入动态 peer 集，需避免恶意地址污染（通过世界/版本/消息合法性过滤）。
- 回退风险：DistFS 恢复失败审计写入本身若异常，必须不阻塞 JSON 兜底路径。

## 原文约束点映射（内容保真）
- 约束-1（目标与问题定义）：沿用原“目标”章节约束，不改变问题定义与解决方向。
- 约束-2（范围边界）：沿用原“范围”章节的 In Scope/Out of Scope 语义，不扩散到新增范围。
- 约束-3（接口/里程碑/风险）：沿用原接口字段、阶段节奏与风险口径，并保持可追溯。
