# oasis7 Runtime：分布式存储自愈定时轮询（2026-02-23）

- 对应设计文档: `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.design.md`
- 对应项目管理文档: `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.project.md`

审计轮次: 5
## ROUND-002 主从口径
- 本文档仅描述 self-healing 轮询策略/状态/入口增量；主文档为 `doc/p2p/distfs/distfs-self-healing-control-plane-2026-02-23.prd.md`，完整主从说明见 `doc/p2p/prd.index.md` 的 ROUND-002 distfs/self-healing 条目。

## 1. Executive Summary
- Problem Statement: 在现有 `plan_replica_maintenance + execute_replica_maintenance_plan` 基础上补齐按周期触发的轮询入口。
- Proposed Solution: 让自愈控制面从“可手工触发”升级为“可自动周期巡检触发”，避免长期运行依赖人工干预。
- Success Criteria:
  - SC-1: 保持无单机完整依赖假设：轮询仅驱动维护计划，不引入对单机全量数据的回退路径。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：分布式存储自愈定时轮询（2026-02-23） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: `oasis7_net::replica_maintenance` 新增轮询模型：
  - AC-2: 轮询策略（间隔毫秒）。
  - AC-3: 轮询状态（上次执行时间）。
  - AC-4: 轮询结果（计划+执行报告）。
  - AC-5: 新增轮询入口：
  - AC-6: 根据 `now_ms` 与 `last_polled_at_ms` 判断是否到期；到期则执行计划与维护任务。
- Non-Goals:
  - NodeRuntime 侧具体线程调度接线（本轮先提供可直接复用的轮询函数）。
  - 真实跨机传输协议优化（沿用现有执行器抽象）。
  - 多租户/多 world 的任务优先级编排。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.prd.md`
  - `doc/p2p/distfs/distfs-self-healing-polling-loop-2026-02-23.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口/数据
### 1) 轮询策略
- `ReplicaMaintenancePollingPolicy`
  - `poll_interval_ms: i64`（默认 60_000）

### 2) 轮询状态
- `ReplicaMaintenancePollingState`
  - `last_polled_at_ms: Option<i64>`

### 3) 轮询入口
- `run_replica_maintenance_poll(...) -> Result<Option<ReplicaMaintenanceRoundResult>, WorldError>`
  - 未到间隔：返回 `Ok(None)`。
  - 到间隔：执行 `plan + execute`，返回 `Ok(Some(...))`。

### 4) 轮询结果
- `ReplicaMaintenanceRoundResult`
  - `polled_at_ms`
  - `plan`
  - `report`

#### 当前状态
- 状态：已完成
- 最近更新：2026-03-06（ROUND-005 I5-001 字段补齐）
- 已完成：M0、M1、M2
- 进行中：无
- 未开始：无

## 5. Risks & Roadmap
- Phased Rollout:
  - M0：设计与任务拆解。
  - M1：轮询策略/状态/入口与单测落地。
  - M2：回归、文档与日志收口。
- Technical Risks:
  - 风险：轮询周期过短可能造成无效高频检查。
  - 缓解：策略要求 `poll_interval_ms > 0`，并在接口层提前拦截非法配置。
  - 风险：时间回拨导致执行节奏异常。
  - 缓解：使用基于 `last_polled_at_ms` 的显式间隔判断，未到期则跳过。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-078-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-078-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
