# oasis7 Runtime：生产级区块链 + P2P FS 路线图 Phase C（PRG-M5 跨节点 DistFS 挑战证明网络化）

- 对应设计文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.design.md`
- 对应项目管理文档: `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md`

审计轮次: 5

> 现行状态：本文是历史 P2PFS 路线图的 Phase C 阶段文档，保留用于追溯 DistFS challenge/proof 网络化收口。当前链上大世界状态底座的聚合闭环、测试层级与 claim boundary 以 `testing-manual.md#s9a链上大世界状态底座自闭环` 和 `PRD-P2P-031` 为准；本文不单独证明 S9A `module_full`、`integration_required` 或 `release_full`。

## 1. Executive Summary
- Problem Statement: 将存储可用性证明从“本地 self-challenge/self-answer”升级为“跨节点 challenge-response 网络证明”。
- Proposed Solution: 在现有 reward runtime 中建立可审计的 DistFS challenge request/proof 通道，使存储节点收益依赖跨节点可验证证明。
- Success Criteria:
  - SC-1: 保持当前系统预算池主导的奖励结算模型，不引入需求侧交易市场。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：生产级区块链 + P2P FS 路线图 Phase C（PRG-M5 跨节点 DistFS 挑战证明网络化） 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: **PRG-C1：挑战请求/证明消息协议化**
  - AC-2: 新增 DistFS challenge request/proof 主题与消息结构。
  - AC-3: 增加 request/proof 编解码、签名与验签。
  - AC-4: **PRG-C2：跨节点 challenge-response 运行时**
  - AC-5: 新增 `DistfsChallengeNetworkDriver`：
  - AC-6: 维护待确认 challenge 队列；
- Non-Goals:
  - 需求侧订单撮合与交易化支付市场（PRG-M6 已移除）。
  - 完整 PoRep/PoSt 协议（本期为 challenge-response+签名验证闭环）。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.prd.md`
  - `doc/p2p/blockchain/production-grade-blockchain-p2pfs-phasec-distfs-proof-network.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 1) 主题命名（新增）
- `aw.<world_id>.distfs.challenge.request`
- `aw.<world_id>.distfs.challenge.proof`

### 2) Request Envelope（新增）
```rust
DistfsChallengeRequestEnvelope {
  version: u8,
  world_id: String,
  challenger_node_id: String,
  challenger_public_key_hex: String,
  target_node_id: String,
  challenge: StorageChallenge,
  emitted_at_unix_ms: i64,
  signature: String, // distfschreq:v1:<sig_hex>
}
```

### 3) Proof Envelope（新增）
```rust
DistfsChallengeProofEnvelope {
  version: u8,
  world_id: String,
  responder_node_id: String,
  responder_public_key_hex: String,
  challenge: StorageChallenge,
  receipt: StorageChallengeReceipt,
  emitted_at_unix_ms: i64,
  signature: String, // distfschproof:v1:<sig_hex>
}
```

### 4) 运行时报告（新增）
```rust
DistfsChallengeNetworkTickReport {
  mode: "network" | "fallback_local",
  request_topic: String,
  proof_topic: String,
  known_storage_targets: Vec<String>,
  issued_challenge_ids: Vec<String>,
  answered_challenge_ids: Vec<String>,
  accepted_proof_ids: Vec<String>,
  timed_out_challenge_ids: Vec<String>,
  probe_report: Option<StorageChallengeProbeReport>,
}
```

## 5. Risks & Roadmap
- Phased Rollout:
  - **PRG-CM0**：Phase C 文档与任务拆解。
  - **PRG-CM1**：challenge/proof 消息协议与签名验签落地。
  - **PRG-CM2**：跨节点 challenge-response driver 落地。
  - **PRG-CM3**：reward runtime 接线与报表输出。
  - **PRG-CM4**：回归测试与文档/devlog 收口。
- Technical Risks:
  - 低活跃网络下 proof 回包延迟会导致短时 `total_checks=0` 或 timeout，需要在阈值策略上容忍。
  - challenge/proof 双向签名增加排障成本，需要报表输出完整 challenge_id 轨迹。
  - 若不同节点分片数据不一致，跨节点 challenge 会出现高失败率，需要结合复制策略与可用数据集治理。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-055-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-055-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
