# oasis7 Runtime：分布式存储自愈控制面（2026-02-23）

- 对应设计文档: `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.design.md`
- 对应项目管理文档: `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md`

审计轮次: 5
## ROUND-002 主从口径
- 本文档为 distfs-self-healing 主文档；完整主从说明见 `doc/p2p/prd.index.md` 的 ROUND-002 distfs/self-healing 条目。

## 1. Executive Summary
- Problem Statement: 补齐此前 out-of-scope 的控制面空缺：让系统在 provider 异构与节点波动下可持续自动修复副本并做负载重平衡。
- Proposed Solution: 将“副本修复 / 重平衡”从人工运维动作收敛为可执行、可测试的控制面闭环。
- Success Criteria:
  - SC-1: 不引入“任意单机可完整提供所有数据”的假设，修复与迁移均以多 provider 索引为前提。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：分布式存储自愈控制面（2026-02-23） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: `oasis7_net` 新增 `replica_maintenance` 模块，提供：
  - AC-2: 副本维护策略（目标副本数、每轮最大任务、负载阈值等）。
  - AC-3: 维护计划生成器（Repair + Rebalance）。
  - AC-4: 维护计划执行器（执行成功后写回 DHT provider 索引）。
  - AC-5: 对外导出维护计划/报告结构，便于 node/runtime 后续调度接线。
  - AC-6: `oasis7_net` 单测覆盖：
- Non-Goals:
  - 跨机真实数据搬运协议（本轮仅定义执行器接口，不绑定具体传输实现）。
  - 跨 DC 拓扑感知、地域感知放置策略。
  - 纠删码编码/解码与碎片修复协议。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md`
  - `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口/数据
### 1) 维护策略
- `ReplicaMaintenancePolicy`：
  - `target_replicas_per_blob: usize`（默认 3）
  - `max_repairs_per_round: usize`（默认 32）
  - `max_rebalances_per_round: usize`（默认 32）
  - `rebalance_source_load_min_per_mille: u16`（默认 850）
  - `rebalance_target_load_max_per_mille: u16`（默认 450）

### 2) 计划与执行
- `plan_replica_maintenance(...) -> ReplicaMaintenancePlan`
  - 输入：`world_id` + 目标 blob 列表 + DHT + 策略
  - 输出：
    - `repair_tasks`
    - `rebalance_tasks`
    - `warnings`（如缺少可用 source/target）
- `execute_replica_maintenance_plan(...) -> ReplicaMaintenanceReport`
  - 通过抽象执行器接口执行任务，成功后 `publish_provider` 到 DHT。

### 3) 任务模型
- `ReplicaTransferTask`
  - `content_hash`
  - `source_provider_id`
  - `target_provider_id`
  - `kind`（`Repair` / `Rebalance`）

#### 当前状态
- 状态：已完成
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：M0、M1、M2、M3
- 进行中：无
- 未开始：无

## 5. Risks & Roadmap
- Phased Rollout:
  - M0：设计与任务拆解。
  - M1：维护计划生成器（Repair + Rebalance）落地。
  - M2：计划执行器落地并形成索引回写闭环。
  - M3：回归、文档与日志收口。
- Technical Risks:
  - 风险：DHT provider 信息滞后导致计划不最优。
  - 缓解：计划按轮次短周期运行，单轮任务上限与可回滚错误报告。
  - 风险：执行器失败可能造成局部修复停滞。
  - 缓解：失败不写回索引，报告中保留失败项，供下一轮重试或降级处理。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-077-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-077-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
