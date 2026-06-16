# oasis7 Runtime：分布式存储自愈轮询 Runtime 接线（2026-02-23）

- 对应设计文档: `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.design.md`
- 对应项目管理文档: `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md`

审计轮次: 5
## ROUND-002 主从口径
- 本文档仅描述 self-healing NodeRuntime 接线与执行器增量；主文档为 `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md`，完整主从说明见 `doc/p2p/prd.index.md` 的 ROUND-002 distfs/self-healing 条目。

## 1. Executive Summary
- Problem Statement: 将已实现的 `run_replica_maintenance_poll` 接入 `NodeRuntime` 周期循环，实现自动触发副本维护。
- Proposed Solution: 保持“无单机完整数据假设”：节点仅在目标副本为本节点时执行拉取与落盘，不引入单机全量回退路径。
- Success Criteria:
  - SC-1: 保持现有共识/复制主链路稳定，轮询失败不影响主 tick 的推进，仅记录错误。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：分布式存储自愈轮询 Runtime 接线（2026-02-23） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: `oasis7_node` 新增 Runtime 级副本维护配置模型（启用开关、采样窗口、维护策略、轮询策略）。
  - AC-2: 在 `NodeRuntime` worker tick 中接线维护轮询：
  - AC-3: 按 `poll_interval_ms` 判断是否执行。
  - AC-4: 采集候选 content hash（来自本地复制热数据窗口）。
  - AC-5: 调用 `run_replica_maintenance_poll` 执行计划+任务。
  - AC-6: 新增 Node 侧执行器：
- Non-Goals:
  - 远端目标节点任务委派与跨节点协同执行协议（本轮仅做本地目标可执行闭环）。
  - 全局调度仲裁与冲突解决（并发多节点 planning 冲突）。
  - 维护结果持久化审计索引（本轮仅依赖 runtime 内错误观测）。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.prd.md`
  - `doc/p2p/distfs/distfs-self-healing-runtime-polling-wiring-2026-02-23.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口/数据
### 1) Runtime 配置
- `NodeReplicaMaintenanceConfig`
  - `enabled: bool`
  - `max_content_hash_samples_per_round: usize`
  - `target_replicas_per_blob: usize`
  - `max_repairs_per_round: usize`
  - `max_rebalances_per_round: usize`
  - `rebalance_source_load_min_per_mille: u16`
  - `rebalance_target_load_max_per_mille: u16`
  - `poll_interval_ms: i64`

### 2) Runtime 状态扩展
- `RuntimeState` 增加副本维护轮询状态与最近一轮摘要（最小必要字段）。

### 3) DHT 句柄接线
- `NodeRuntime` 新增可选 DHT 句柄注入接口；未注入时维护轮询自动降级为跳过。

### 4) 轮询执行语义
- 轮询仅在满足以下条件时运行：
  - 配置启用；
  - replication runtime 可用；
  - replication network 可用；
  - DHT 句柄可用；
  - 本轮可采样到 content hash。
- 轮询失败写入 `last_error`，不阻断 tick 主流程。

#### 当前状态
- 状态：已完成
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：M0、M1、M2
- 进行中：无
- 未开始：无

## 5. Risks & Roadmap
- Phased Rollout:
  - M0：设计与任务拆解。
  - M1：Runtime 配置/状态/轮询接线与执行器实现。
  - M2：单测与跨 crate 回归，文档日志收口。
- Technical Risks:
  - 风险：多节点并发规划产生重复任务。
  - 缓解：执行器只接收本地 target 任务，天然抑制跨节点误执行；重复写同 hash 在 CAS 层幂等。
  - 风险：provider 定向请求不可用导致拉取不稳定。
  - 缓解：先尝试按 provider 定向请求，失败时按网络默认请求兜底并保留错误。
  - 风险：轮询耗时影响 tick 周期。
  - 缓解：通过 `max_content_hash_samples_per_round` 与维护策略配额限制单轮开销。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-079-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-079-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
