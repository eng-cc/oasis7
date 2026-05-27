# oasis7 Runtime：Node libp2p wasm32 编译兼容守卫

- 对应设计文档: `doc/p2p/node/node-wasm32-libp2p-compile-guard.design.md`
- 对应项目管理文档: `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md`

审计轮次: 5
## 1. Executive Summary
- Problem Statement: 修复 `oasis7_node` 在 `wasm32-unknown-unknown` 目标上的编译失败，避免阻塞 pre-commit 的 Web Viewer wasm 编译门禁。
- Proposed Solution: 保持 native 目标下现有 libp2p replication 能力不变。
- Success Criteria:
  - SC-1: 稳定 `viewer::web_bridge` 相关回归测试，消除提交门禁中的偶发 `WouldBlock/Disconnected` 失败。
  - SC-2: 明确 wasm32 节点网络定位：不做 full node 协议栈，只提供编译占位与显式 unavailable。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：Node libp2p wasm32 编译兼容守卫 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: `crates/oasis7_node/src/lib.rs`
  - AC-2: `crates/oasis7_node/src/libp2p_replication_network_wasm.rs`（新增）
  - AC-3: `crates/oasis7/src/viewer/web_bridge.rs`
  - AC-4: `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md`
  - AC-5: `doc/devlog/README.md`
  - AC-6: 调整 PoS/gossip 共识业务语义。
- Non-Goals:
  - 不扩展超出原文边界的新需求。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/node/node-wasm32-libp2p-compile-guard.prd.md`
  - `doc/p2p/node/node-wasm32-libp2p-compile-guard.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口/数据
- 对外接口保持不变：继续导出 `Libp2pReplicationNetwork` 与 `Libp2pReplicationNetworkConfig`。
- 在 `wasm32` 目标下提供等价 API 的最小占位实现：
  - `new/peer_id` 可用；
  - `DistributedNetwork` 能力返回 `NetworkProtocolUnavailable`（显式声明当前目标不支持 native libp2p replication）。

## 5. Risks & Roadmap
- Phased Rollout:
  - M1：完成文档立项与任务拆解。
  - M2：完成 `oasis7_node` 的 wasm32 编译守卫与占位实现。
  - M3：稳定 `web_bridge` 重连测试并完成回归验证（`oasis7_node` native check + `oasis7_viewer` wasm32 check）后收口文档。
- Technical Risks:
  - 若后续在 wasm 侧误用该网络实现，运行期会收到 unavailable 错误；需由调用方按目标区分能力。
  - 占位实现需保持接口稳定，避免对现有 native 调用路径造成回归。
  - 测试稳定性修复依赖本地调度时序，需避免再次引入短超时或非阻塞 socket 继承导致的间歇失败。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-104-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-104-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
