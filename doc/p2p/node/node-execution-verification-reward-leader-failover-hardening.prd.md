# oasis7 Runtime：节点执行校验与奖励 Leader/Failover 生产化收口

- 对应设计文档: `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.design.md`
- 对应项目管理文档: `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.project.md`

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 收口执行一致性缺口：将节点对等 commit 的执行绑定从“仅透传/记录”提升为“可校验、可拒绝、可在补洞路径重放验证”。
- Proposed Solution: 收口执行一致性缺口：将节点对等 commit 的执行绑定从“仅透传/记录”提升为“可校验、可拒绝、可在补洞路径执行一致性验证”。
- Success Criteria:
  - SC-1: 收口奖励结算编排缺口：为 reward runtime 引入显式 leader/failover 策略，避免“隐式只有 sequencer 发布”导致的运行不可观测和不可配置。
  - SC-2: 在不依赖完整玩法模块完工的前提下，先收口链上大世界状态底座中的节点执行校验、peer commit 绑定与 reward leader/failover 子链路；该专题只提供底座子链路证据，不单独代表 transport-only P2P ready、S9A `module_full`、`integration_required` 或 `release_full`。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：节点执行校验与奖励 Leader/Failover 生产化收口 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: `crates/oasis7_node`
  - AC-2: 增加执行校验策略配置：
  - AC-3: `require_execution_on_commit`
  - AC-4: `require_peer_execution_hashes`
  - AC-5: 对等 commit 入站校验增强：
  - AC-6: 可配置要求 commit 必须携带 `execution_block_hash + execution_state_root`。
- Non-Goals:
  - 共识算法重写（PoS 出块规则保持不变）。
  - 浏览器 wasm32 端完整分布式节点协议栈。
  - 跨世界结算与跨分片事务。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.prd.md`
  - `doc/p2p/node/node-execution-verification-reward-leader-failover-hardening.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) NodeConfig 新增策略字段
```rust
NodeConfig {
  require_execution_on_commit: bool,
  require_peer_execution_hashes: bool,
}
```

### 2) 节点快照新增字段
```rust
NodeConsensusSnapshot {
  last_committed_at_ms: Option<i64>,
  peer_heads: Vec<NodePeerCommittedHead>,
}

NodePeerCommittedHead {
  node_id: String,
  height: u64,
  block_hash: String,
  committed_at_ms: i64,
  execution_block_hash: Option<String>,
  execution_state_root: Option<String>,
}
```

### 3) reward runtime leader/failover 配置
```rust
CliOptions {
  reward_runtime_leader_node_id: Option<String>,
  reward_runtime_leader_stale_ms: u64,
  reward_runtime_failover_enabled: bool,
}
```

### 4) reward settlement 触发规则（网络模式）
- 仅当满足以下条件才允许本地发布 settlement：
  - 观察者阈值满足。
  - 本地共识状态处于 committed。
  - 本节点是 leader；或 leader 已 stale 且本节点成为 failover 候选发布者。

## 5. Risks & Roadmap
- Phased Rollout:
  - M0：设计与项目管理文档冻结。
  - M1：`oasis7_node` 执行校验策略与补洞执行一致性校验落地。
  - M2：`oasis7_viewer_live` leader/failover 策略与运行默认语义收口。
  - M3：测试回归（required-tier 定向）与文档/devlog 收口。
- Technical Risks:
  - 严格执行校验可能暴露历史“宽松路径”下未显化的问题，初期可能增加拒绝日志与排障成本。
  - failover 判定依赖本地观测视图，极端网络分区下可能出现临时双发布，需要幂等去重兜底。
  - triad 全角色执行会增加 CPU/IO 开销，需要后续压测评估阈值。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-094-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-094-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
