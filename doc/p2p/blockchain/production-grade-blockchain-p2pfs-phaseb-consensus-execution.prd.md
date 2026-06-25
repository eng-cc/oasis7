# oasis7 Runtime：生产级区块链 + P2P FS 路线图 Phase B（共识内生执行）

- 对应设计文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md`

审计轮次: 5

> 现行状态：本文是历史 P2PFS 路线图的 Phase B 阶段文档，保留用于追溯共识内生执行收口。当前链上大世界状态底座的聚合闭环、测试层级与 claim boundary 以 `testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. Executive Summary
- Problem Statement: 将当前“reward runtime 外围 execution bridge 驱动执行”的模式推进为“节点共识主循环内生执行”。
- Proposed Solution: 让共识提交高度与执行高度/状态根在同一条节点主链路中推进，降低双循环一致性风险。
- Success Criteria:
  - SC-1: 保持与现有报表、CAS 记录目录兼容，并提供平滑 fallback。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：生产级区块链 + P2P FS 路线图 Phase B（共识内生执行） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: **PRG-B1：节点执行 Hook 接口与快照字段**
  - AC-2: 在 `oasis7_node` 引入可注入的执行驱动接口（commit 后触发）。
  - AC-3: `NodeConsensusSnapshot` 增加 execution 相关字段（执行高度、执行块哈希、执行状态根）。
  - AC-4: 运行时重启后恢复 execution 快照字段。
  - AC-5: **PRG-B2：oasis7_viewer_live 内生执行接线**
  - AC-6: 新增 `NodeRuntimeExecutionDriver`，将现有 execution bridge 的步进/落盘逻辑封装为节点执行驱动。
- Non-Goals:
  - DistFS challenge-response 跨节点请求/应答协议化（PRG-M5）。
  - 需求侧交易撮合、报价和订单清结算（已移除，维持系统预算池主导）。
  - 治理层签名阈值升级与密钥轮换编排。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.prd.md`
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phaseb-consensus-execution.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) 节点执行驱动输入（新增）
```rust
NodeExecutionCommitContext {
  world_id: String,
  node_id: String,
  height: u64,
  slot: u64,
  epoch: u64,
  node_block_hash: String,
  committed_at_unix_ms: i64,
}
```

### 2) 节点执行结果（新增）
```rust
NodeExecutionCommitResult {
  execution_height: u64,
  execution_block_hash: String,
  execution_state_root: String,
}
```

### 3) 节点共识快照字段扩展（新增）
```rust
NodeConsensusSnapshot {
  // existing fields...
  last_execution_height: u64,
  last_execution_block_hash: Option<String>,
  last_execution_state_root: Option<String>,
}
```

### 4) 快照持久化扩展（新增）
```rust
PosNodeStateSnapshot {
  // existing fields...
  last_execution_height: u64,
  last_execution_block_hash: Option<String>,
  last_execution_state_root: Option<String>,
}
```

## 5. Risks & Roadmap
- Phased Rollout:
  - **PRG-BM0**：完成 Phase B 设计文档与项目管理文档。
  - **PRG-BM1**：`oasis7_node` 执行 hook + 快照/持久化扩展。
  - **PRG-BM2**：`oasis7_viewer_live` execution driver 接入 NodeRuntime。
  - **PRG-BM3**：测试回归、文档与 devlog 收口。
- Technical Risks:
  - 节点线程内执行耗时若过长，可能影响共识 tick 周期；需保持执行驱动幂等且轻量。
  - 内生执行与旧 bridge 并存阶段，若切换条件不清晰可能导致重复执行。
  - 快照字段扩展需要 `serde(default)` 兼容旧状态文件，避免升级失败。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-054-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-054-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
