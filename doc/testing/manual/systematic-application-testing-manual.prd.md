# oasis7：系统性应用测试手册工程化收口（2026-02-26）

- 对应设计文档: `doc/testing/manual/systematic-application-testing-manual.design.md`
- 对应项目管理文档: `doc/testing/manual/systematic-application-testing-manual.project.md`

审计轮次: 4


## 1. Executive Summary
- Problem Statement: 测试分层模型、触发矩阵与证据标准若分散在多处文档/脚本，容易出现执行口径漂移，导致“通过门禁但风险未覆盖”。
- Proposed Solution: 以 `testing-manual.md` 作为统一入口，配套 Web 闭环分册与脚本入口，固化 Human/AI 共用的可审计测试流程。
- Success Criteria:
  - SC-1: `testing-manual.md` 稳定承载分层模型（L0~L5）与套件映射（S0~S10）。
  - SC-2: 手册、脚本入口与 CI 门禁口径一致，不出现冲突说明。
  - SC-3: Web 闭环分册与主手册引用稳定，执行路径唯一。
  - SC-4: 改动路径到必跑套件映射可复用，发布前可直接判定 required/full 组合。
  - SC-5: 文档迁移后命名统一为 `.prd.md/.project.md`，并通过文档治理检查。

## 2. User Experience & Functionality
- User Personas:
  - 测试维护者：负责维护分层模型、触发矩阵与证据标准。
  - 功能开发者：根据改动路径选择最小必跑套件。
  - 发布负责人：基于证据包进行放行判断。
- User Scenarios & Frequency:
  - 每次核心功能改动后：执行 required 路径并核对证据。
  - 每次发布前：执行 required + full，并回收审计证据。
  - 每次测试体系变更后：同步手册与脚本入口，避免口径漂移。
- User Stories:
  - PRD-TESTING-MANUAL-001: As a 测试维护者, I want one canonical manual, so that test rules stay consistent.
  - PRD-TESTING-MANUAL-002: As a 开发者, I want clear suite mapping, so that I can run the right tests efficiently.
  - PRD-TESTING-MANUAL-003: As a 发布负责人, I want auditable test evidence, so that release decisions are traceable.
- Critical User Flows:
  1. Flow-TMAN-001: `识别改动范围 -> 对照 L0~L5/S0~S10 -> 生成必跑清单`
  2. Flow-TMAN-002: `执行脚本入口 -> 收集命令/日志/截图 -> 填写结论`
  3. Flow-TMAN-003: `发布前复核证据包 -> 校验缺失项 -> 放行或阻断`
  4. Flow-TMAN-004: `更新 Web 分册或门禁脚本 -> 回写主手册引用 -> 重新校验`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 按钮/动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 手册主入口维护 | `testing-manual.md`、章节索引、版本日期 | 更新主手册并发布统一口径 | `draft -> active -> archived` | 主手册优先级最高 | 测试维护者维护 |
| 分层与套件映射 | `L0~L5`、`S0~S10`、触发条件 | 按改动路径选择 required/full | `planned -> running -> verified` | required 为默认下限，发布叠加 full | 开发/测试共用 |
| 证据规范 | 命令、日志、截图、结论、责任人 | 执行后归档证据包并审阅 | `collecting -> reviewed -> accepted/rejected` | 缺字段即不通过 | 发布负责人审核 |
| Web 分册联动 | 分册路径、脚本入口、GPU/headed 约束 | 按分册执行 Web 闭环并回填结果 | `linked -> executed -> traced` | 主手册引用必须可达 | 测试维护者与执行人 |
- Acceptance Criteria:
  - AC-1: 主手册包含分层模型、套件矩阵、执行与证据规则。
  - AC-2: Web 闭环细节以分册维护，主手册保留唯一引用入口。
  - AC-3: `scripts/ci-tests.sh` 与发布脚本在手册中可追踪到对应章节。
  - AC-4: required/full 判定标准可直接用于任务收口。
  - AC-5: 本专题迁移后命名统一且引用无断链。
- Non-Goals:
  - 不在本专题新增业务功能测试代码。
  - 不在本专题引入新的测试框架。
  - 不在本专题重写所有历史 devlog。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用（本专题为测试手册工程化与执行口径收口，不涉及 AI 推理系统改造）。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 以 `testing-manual.md` 为主入口，`doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md` 为 Web 闭环操作分册，`doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md` 为需求/验收权威源，脚本入口负责执行层闭环，四者通过引用关系保持一致。
- Integration Points:
  - `testing-manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.manual.md`
  - `doc/testing/manual/web-ui-agent-browser-closure-manual.prd.md`
  - `scripts/ci-tests.sh`
  - `scripts/viewer-release-qa-loop.sh`
  - `scripts/viewer-release-full-coverage.sh`
- Edge Cases & Error Handling:
  - 文档与脚本不同步：以主手册为准，变更同批次修复引用。
  - 套件入口分散：强制通过主手册索引归并，避免重复入口。
  - 证据缺失：判定为测试未完成，禁止发布放行。
  - Web 闭环环境异常：按分册 fail-fast 分级归档并阻断。
- Non-Functional Requirements:
  - NFR-TMAN-1: 手册更新后 1 个工作日内完成相关入口引用修正。
  - NFR-TMAN-2: 发布证据字段完整率维持 100%。
  - NFR-TMAN-3: 主手册与分册引用检查零断链。
  - NFR-TMAN-4: required 路径说明应在 10 分钟内可定位执行命令。
- Security & Privacy: 文档与证据包不得记录密钥/凭据，日志需遵循最小暴露原则。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (TMAN-1): 完成手册迁移与命名统一。
  - v1.1 (TMAN-2): 收口分层模型（L0~L5）与套件矩阵（S0~S10）。
  - v2.0 (TMAN-3): 完成 Web 分册拆分、引用与门禁口径对齐。
  - v2.1 (TMAN-4): 持续维护增量规则并同步 CI 变更。
  - v2.2 (TMAN-5): 本专题 strict schema 人工迁移与命名统一收口。
- Technical Risks:
  - 风险-1: 手册更新滞后于脚本变更，导致执行失败。
  - 风险-2: Web 分册与主手册引用漂移，造成双口径。
  - 风险-3: 团队绕过手册直接执行脚本，证据质量下降。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-TESTING-MANUAL-001 | TMAN-1/2/5 | `test_tier_required` | 手册章节与触发矩阵人工审阅 + 文档治理检查 | 测试主入口一致性 |
| PRD-TESTING-MANUAL-002 | TMAN-2/3/4 | `test_tier_required` | 分层模型与套件映射抽样校验 | 改动路径必跑集合判定 |
| PRD-TESTING-MANUAL-003 | TMAN-3/4/5 | `test_tier_required` | 证据规范字段核验 + 引用回归扫描 | 发布放行与审计追溯 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-TMAN-001 | 主手册 + 分册双层结构 | 所有细节堆叠在单文档 | 保持可读性并降低维护冲突。 |
| DEC-TMAN-002 | required/full 分层执行策略 | 每次全量执行 | 兼顾执行效率与发布覆盖。 |
| DEC-TMAN-003 | 证据包作为发布门禁硬要求 | 仅口头确认结果 | 无法支撑审计与追溯。 |
| DEC-TMAN-004 | legacy 文档逐篇人工迁移 | 脚本批量改写 | 保证内容语义与约束完整保真。 |

## 原文约束点映射（内容保真）
- 原“目标：统一分层测试模型、触发矩阵、证据标准” -> 第 1 章 Problem/Solution/SC 与第 2 章功能矩阵。
- 原“In Scope/Out of Scope” -> 第 2 章 AC/Non-Goals。
- 原“接口/数据（主手册、分册、脚本入口）” -> 第 4 章 Integration Points。
- 原“M1~M4 里程碑” -> 第 5 章 phased rollout（TMAN-1~TMAN-5）。
- 原“风险（手册脚本不同步、入口分散）” -> 第 4 章边界处理 + 第 5 章风险。
