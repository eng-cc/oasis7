# oasis7 Runtime：Observer 同步源策略化

- 对应设计文档: `doc/p2p/observer/observer-sync-source-mode.design.md`
- 对应项目管理文档: `doc/p2p/observer/observer-sync-source-mode.project.md`

审计轮次: 5

## 专业权威口径
- 本文件是 Observer 非 DHT 与 DHT 组合同步源策略的当前专业权威。
- 原 DHT 增量专题 `PRD-P2P-MIG-109-001` 的有效语义与任务追踪已合并到本三件套；源文件删除后只从 Git history 与 GitHub task evidence 追溯。

## 1. Executive Summary
- Problem Statement: 为 `ObserverClient` 增加可配置的 head 同步源策略，显式控制“网络路径”与“路径索引路径”的使用方式。
- Proposed Solution: 支持网络失败时回退路径索引，提升本地恢复弹性。
- Success Criteria:
  - SC-1: 保持现有 API 可用，不破坏当前调用方。

## 2. User Experience & Functionality
- User Personas: 协议维护者、任务执行者、质量复核者。
- User Scenarios & Frequency: 每次专题改动前后执行需求核对、测试回归与状态回写。
- User Stories: As a 维护者, I want oasis7 Runtime：Observer 同步源策略化 的需求结构化, so that implementation is auditable.
- Critical User Flows: `阅读旧文档 -> 重写为 strict PRD -> 回写项目文档 -> 校验提交`。
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 专题迁移 | 需求/任务/依赖/状态/测试层级 | 逐篇重写并校验 | `draft -> active -> done` | 以原文约束点映射为主线 | 维护者写入，复核者抽检 |
- Acceptance Criteria:
  - AC-1: 定义 `HeadSyncSourceMode` 策略枚举（非 DHT 链路）。
  - AC-2: 在 `ObserverClient` 增加 `sync_heads_with_mode` 与对应报告/结果/循环接口。
  - AC-3: 支持模式：
  - AC-4: `NetworkOnly`
  - AC-5: `PathIndexOnly`
  - AC-6: `NetworkThenPathIndex`
  - AC-7: DHT 组合模式提供 `NetworkWithDhtOnly`、`PathIndexOnly` 与 `NetworkWithDhtThenPathIndex`。
  - AC-8: 只有网络+DHT 链路报错时才允许回退路径索引；若回退也失败，必须保留两个错误的可诊断上下文。
- Non-Goals:
  - 全局配置中心或动态热更新配置。
  - 指标埋点/告警联动。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题不涉及 AI 模型能力改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 保持原文技术边界，按 strict PRD 结构重排。
- Integration Points:
  - `doc/p2p/observer/observer-sync-source-mode.prd.md`
  - `doc/p2p/observer/observer-sync-source-mode.project.md`
  - `testing-manual.md`
- Edge Cases & Error Handling: 命名不一致、章节缺失、引用断链需在同提交修复。
- Non-Functional Requirements: PRD-ID/任务映射完整；治理检查通过。
- Security & Privacy: 不引入敏感信息与本地绝对路径。

### 原文技术约束（保真）
#### 接口 / 数据
### 策略枚举（草案）
- `HeadSyncSourceMode::NetworkOnly`
- `HeadSyncSourceMode::PathIndexOnly`
- `HeadSyncSourceMode::NetworkThenPathIndex`

### 语义约束
- `NetworkOnly`：仅走现有网络恢复链路，失败直接返回错误。
- `PathIndexOnly`：仅走路径索引恢复链路。
- `NetworkThenPathIndex`：先走网络；仅在网络恢复报错时回退路径索引。

### DHT 组合策略
- `HeadSyncSourceModeWithDht::NetworkWithDhtOnly`：仅调用 `sync_from_heads_with_dht`，失败直接返回。
- `HeadSyncSourceModeWithDht::PathIndexOnly`：仅走路径索引恢复。
- `HeadSyncSourceModeWithDht::NetworkWithDhtThenPathIndex`：先走网络+DHT；只有该链路报错才回退路径索引。
- DHT 是同步源组合能力增强，不改变 Observer 主同步与一致性语义。两段链路都失败时不得用最终错误覆盖首段网络/DHT 失败原因。

## 5. Risks & Roadmap
- Phased Rollout:
  - OSSM-1：设计文档与项目管理文档落地。
  - OSSM-2：策略枚举与 `ObserverClient` 模式化接口实现。
  - OSSM-3：补齐测试并完成 `oasis7_net` 回归。
  - OSSM-4：状态文档与 devlog 收口。
- Technical Risks:
  - 模式过多可能引入调用歧义，需保持命名清晰。
  - 回退策略若吞掉网络错误，定位问题成本会提升，需要保留错误上下文。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-P2P-MIG-110-001 | T0~Tn | `test_tier_required` | 文档治理检查 + 章节完整性核验 | 专题文档可维护性 |
| PRD-P2P-MIG-109-001 | observer-sync-dht-authority | `test_tier_required` | DHT 组合模式与回退错误上下文回归 | Observer DHT 同步源 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-PRD-P2P-MIG-110-001 | 逐篇阅读后人工重写 | 直接重命名 | 保证语义保真和可审计性。 |

## 原文约束点映射（内容保真）
- 原“目标” -> 第 1 章。
- 原“范围” -> 第 2 章。
- 原“接口/数据、里程碑、风险” -> 第 4~6 章。
