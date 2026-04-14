# Rust 超限文件拆分（第三轮，2026-02-23）

- 对应设计文档: `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.design.md`
- 对应项目管理文档: `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md`

审计轮次: 4

- 对应标准执行入口: `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md`

## 1. Executive Summary
- Problem Statement: 仓库存在 22 个 Rust 文件超过单文件 1200 行约束，导致可维护性下降并增加评审与回归风险。
- Proposed Solution: 在不改变行为和对外接口的前提下，采用结构拆分（`include!` 分段或子模块拆分）将全部超限文件收口到阈值内。
- Success Criteria:
  - SC-1: 扫描范围内 22 个超限 Rust 文件全部完成拆分并降到 <= 1200 行。
  - SC-2: 拆分后 `cargo check` 通过。
  - SC-3: required-tier 定向回归通过，且未引入协议/行为变更。
  - SC-4: round3 任务与证据在 `.project.md` 与 `doc/devlog` 可追溯。

## 2. User Experience & Functionality
- User Personas:
  - 工程维护者：负责代码体量治理与长期可维护性。
  - 贡献开发者：需要在拆分后继续稳定开发。
  - 评审者：需要低风险、可审计的拆分方案。
- User Scenarios & Frequency:
  - 超限治理批次执行：按治理批次触发，本轮为 round3 一次性执行。
  - 提交前验证：每次拆分提交前执行编译与定向回归。
  - 治理复核：批次完成后执行一次“零超限”复核。
- User Stories:
  - PRD-ENGINEERING-RSPLIT-001: As an 工程维护者, I want all oversized Rust files reduced under the limit, so that maintenance cost stays bounded.
  - PRD-ENGINEERING-RSPLIT-002: As a 贡献开发者, I want file organization refactoring without behavior changes, so that feature development is not blocked.
  - PRD-ENGINEERING-RSPLIT-003: As a 评审者, I want auditable split evidence, so that governance conclusions are defensible.
- Critical User Flows:
  1. Flow-RSPLIT-001: `扫描超限文件 -> 标注拆分策略(include!/mod) -> 执行拆分 -> 编译验证`
  2. Flow-RSPLIT-002: `定向回归 -> 超限复核 -> 文档与日志回写 -> 批次收口`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 超限文件治理 | 文件路径、当前行数、目标行数、拆分策略 | 执行结构拆分并提交 | `identified -> split -> verified` | 按超限程度优先处理 | 工程维护者制定策略，贡献者执行 |
| 结构拆分实现 | `include!` 分段或 `mod` 子模块 | 调整源文件组织，不改行为 | `draft -> compiled -> merged` | 优先低风险拆分路径 | 评审者确认语义不变 |
| 治理证据收口 | 编译结果、回归结果、复核统计 | 回写 project/devlog | `collecting -> recorded -> closed` | 以“零超限”作为收口条件 | 维护者负责最终收口 |
- Acceptance Criteria:
  - AC-1: 覆盖 22 个超限 Rust 文件（含测试、bin、生产模块）。
  - AC-2: 对外 API 与协议字段保持不变。
  - AC-3: 拆分后通过编译与 required-tier 回归。
  - AC-4: 复核结论为“Rust 超限文件 = 0”（排除 `target/third_party` 生成目录）。
  - AC-5: 任务分解、依赖、状态、测试证据均可从 project/devlog 追溯。
- Non-Goals:
  - 不做协议/业务语义调整。
  - 不对非超限文件做风格性重排。
  - 不修改 `third_party/` 代码。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本任务为代码结构治理，不依赖 AI 推理链路）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 仅调整文件组织结构，不修改业务语义。拆分策略以低风险优先，保持调用关系与可见性边界稳定。
- Integration Points:
  - Rust 源文件（测试文件、bin 文件、生产模块）
  - 构建验证链路：`env -u RUSTC_WRAPPER cargo check`
  - 回归验证链路：`test_tier_required` 定向测试
  - 治理记录：本设计文档、对应 project 文档、`doc/devlog`
- Edge Cases & Error Handling:
  - 切分点错误：出现语法/可见性异常时回退该文件拆分并改用更细粒度分段。
  - `include!` 路径错误：编译失败时先修复相对路径再继续批次执行。
  - 模块循环依赖：发生循环引用时改为子模块拆分并重新组织导出层。
  - 回归覆盖不足：若 required-tier 未覆盖改动边界，补充定向用例后再收口。
  - 并发提交冲突：以主干最新版本重放拆分并重新验证。
- Non-Functional Requirements:
  - NFR-RSPLIT-1: 单个 Rust 文件行数上限 1200，新增违规数为 0。
  - NFR-RSPLIT-2: round3 批次拆分后构建通过率 100%。
  - NFR-RSPLIT-3: 任务闭环证据完整率 100%（文档/测试/devlog）。
  - NFR-RSPLIT-4: 治理批次输出可复核且具备可追溯命令记录。
- Security & Privacy: 本任务不涉及新增用户数据与权限模型；仅代码结构调整。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: M1 文档与任务拆解（T0）。
  - v1.1: M2 批量完成 22 个超限文件拆分并通过编译（T1）。
  - v2.0: M3/M4 完成回归、零超限复核与文档收口（T2/T3）。
- Technical Risks:
  - 风险-1: 批量拆分导致语法、可见性或模块边界错误。
  - 风险-2: `include!` 相对路径配置错误导致构建失败。
  - 风险-3: 重构不改语义但可能触发边界行为回归，需要定向回归兜底。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-RSPLIT-001 | T0/T1/T2 | `test_tier_required` | 超限扫描、编译验证、零超限复核 | Rust 文件体量治理 |
| PRD-ENGINEERING-RSPLIT-002 | T1/T2 | `test_tier_required` | 拆分前后行为一致性与定向回归 | 模块结构稳定性 |
| PRD-ENGINEERING-RSPLIT-003 | T3 | `test_tier_required` | project/devlog 追溯检查 | 工程治理可审计性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-RSPLIT-001 | 优先结构拆分（`include!` / `mod`） | 语义重写 | 结构拆分风险更低、可快速收口超限。 |
| DEC-RSPLIT-002 | 全量批次执行后统一复核 | 文件级零散治理 | 批次治理便于统一验证与追溯。 |
| DEC-RSPLIT-003 | required-tier 定向回归兜底 | 仅编译通过即收口 | 可降低结构重构引入行为回归的风险。 |

## 原文约束点映射（内容保真）
- 原“目标（全部 22 个超限文件收口 + 不改语义）” -> 第 1 章 Success Criteria、第 2 章 AC、第 4 章架构约束。
- 原“In/Out of Scope” -> 第 2 章 Acceptance Criteria 与 Non-Goals。
- 原“接口/数据不变” -> 第 4 章 Architecture Overview / Integration Points。
- 原“里程碑 M1~M4 与完成进展” -> 第 5 章 Phased Rollout（含 T0~T3）。
- 原“风险” -> 第 5 章 Technical Risks 与第 4 章 Edge Cases。
