# 全量 PRD 体系审读与对齐（2026-03-03）

- 对应设计文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.design.md`
- 对应项目管理文档: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md`

审计轮次: 4
- 对应标准执行入口: `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md`

## 1. Executive Summary
- Problem Statement: 仓库 PRD 文档规模已达 708 份（含 `prd.md` 与 `project.md`），缺少统一“逐篇审读 + 已读清单 + 代码一致性回写”机制，导致口径漂移风险持续累积。
- Proposed Solution: 建立工程侧 PRD 全量审读机制，按模块执行“逐篇阅读、代码对齐、重复治理、上下游对齐”，并以可勾选清单沉淀可审计证据。
- Success Criteria:
  - SC-1: 全量 PRD 文档 100% 纳入已读清单。
  - SC-2: 每篇文档均具备“代码一致性 / 重复性 / 上下游对齐”评审状态。
  - SC-3: 发现不一致项后以代码为准回写文档，并在同批次完成追溯记录。
  - SC-4: 模块主入口（`prd.md`/`project.md`/`prd.index.md`）审读覆盖率维持 100%。

## 2. User Experience & Functionality
- User Personas:
  - 文档治理维护者：负责建立审读机制、分批推进与收口。
  - 模块负责人：负责专题文档语义准确性、上下游口径对齐。
  - 评审者：负责抽检重复内容与追溯链完整性。
- User Scenarios & Frequency:
  - 每日审读批次：按模块推进并更新已读状态。
  - 变更前审读：涉及跨模块改动前先审读上下游 PRD。
  - 发布前收口：核对 high-risk 文档是否全部与代码一致。
- User Stories:
  - PRD-ENGINEERING-012: As a 文档治理维护者, I want a per-document readable checklist, so that full PRD review progress is auditable.
  - PRD-ENGINEERING-013: As a 模块负责人, I want code-first discrepancy handling, so that PRD behavior always matches implementation.
  - PRD-ENGINEERING-014: As a 评审者, I want duplicate and upstream/downstream alignment checks, so that the PRD tree remains clear and non-conflicting.
- Critical User Flows:
  1. Flow-ENG-PRD-001: `盘点全量 PRD -> 生成分模块清单 -> 标记已读初始状态`
  2. Flow-ENG-PRD-002: `逐篇阅读 -> 核对代码实现 -> 记录一致性结论 -> 发现偏差即回写`
  3. Flow-ENG-PRD-003: `检查重复专题 -> 合并/重定向/裁剪 -> 更新索引与引用`
  4. Flow-ENG-PRD-004: `检查上下游文档链路 -> 修复断链与过时路径 -> 复跑治理门禁`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 全量清单生成 | 文档路径、模块、初始状态 | 生成 checklist 文件并按模块落盘 | `inventory -> published` | 入口优先，风险优先 | 维护者可生成，所有人可读 |
| 逐篇已读标记 | 已读勾选、阅读时刻、结论字段 | 阅读后更新条目并记录结论 | `unread -> read` | 按风险与依赖优先阅读 | 评审者/维护者可写 |
| 代码一致性核对 | 代码路径、行为条款、偏差说明 | 偏差按代码回写 PRD 并留痕 | `pending -> aligned` | 高风险链路优先修复 | 模块负责人审批 |
| 重复性治理 | 重复文档对、保留文档、重定向策略 | 合并条款并修复索引 | `detected -> merged` | 先同模块后跨模块 | 维护者执行，评审者复核 |
| 上下游对齐 | 上游文档、下游文档、引用状态 | 修复断链、更新口径 | `drifted -> synced` | 以模块入口链路优先 | 维护者可写 |
- Acceptance Criteria:
  - AC-1: 产出分模块清单，且每篇 PRD 可被勾选追踪。
  - AC-2: 每条已读记录必须包含阅读时刻与三类结论（代码/重复/上下游）。
  - AC-3: 一旦发现文档与代码不一致，必须在同批次按代码回写并记录处理动作。
  - AC-4: 模块 `prd.index.md` 与专题文档保持可达，不得遗漏活跃专题。
  - AC-5: 审读任务可映射到模块级 `PRD-ID -> Task -> test_tier`。
- Non-Goals:
  - 不在本专题内新增业务功能代码。
  - 不重新引入 `doc/**/archive/` 归档目录。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: `rg`、`scripts/doc-governance-check.sh`、`scripts/site-manual-sync-check.sh`。
- Evaluation Strategy: 以“清单覆盖率、已读完成率、偏差修复闭环率、引用断链数”为核心指标。

## 4. Technical Specifications
- Architecture Overview: 采用“入口文档先行 + 模块清单执行”的审读架构；所有审读结果通过 checklist 固化在 `doc/engineering/prd-review/checklists/`。
- Integration Points:
  - `doc/engineering/prd.md`
  - `doc/engineering/project.md`
  - `doc/engineering/prd.index.md`
  - `doc/engineering/prd-review/prd-full-system-audit-2026-03-03.project.md`
  - `doc/engineering/prd-review/checklists/active-*.md`
  - root legacy redirect checklist（后续已删除；root PRD/project shells no longer have active review targets）
  - `doc/*/prd.md`
  - `doc/*/project.md`
  - `doc/*/prd.index.md`
  - `scripts/doc-governance-check.sh`
- Edge Cases & Error Handling:
  - 旧路径残留：若条目指向历史路径，立即回写到当前 `.prd` 路径并记录。
  - 历史专题替代关系：历史专题若被后续方案替代，需标注替代链并保持索引可追溯。
  - 一文多义：同主题多文档语义重叠时，保留一个主文档并在其余文档明确 redirect。
  - 引用断链：任何 `doc/...` 引用不可达时，视为阻断项，必须在当前任务修复。
  - 高并发更新：审读批次冲突时，以模块 owner 最新提交为准并补充冲突说明。
- Non-Functional Requirements:
  - NFR-1: 审读清单覆盖率 100%（以全量 PRD 清单统计）。
  - NFR-2: 每次批次提交后 `doc-governance-check` 必须通过。
  - NFR-3: 模块入口三件套（`prd.md`/`project.md`/`prd.index.md`）已读状态保持 100%。
  - NFR-4: 偏差修复任务需在 1 个批次内闭环，不跨批次悬挂。
  - NFR-5: 清单文件应保持可人工审阅粒度，避免单文件不可读膨胀。
- Security & Privacy: 审读记录不得引入凭据、私密路径或敏感运行参数。

## 5. Risks & Roadmap
- Phased Rollout:
  - Phase-0 (2026-03-03): 建立全量审读专题、生成清单并完成入口文档首批审读。
  - Phase-1 (完成于 2026-03-04): 按模块逐篇推进审读与偏差修复。
  - Phase-2 (完成于 2026-03-05): 清理 archive 目录与历史引用，统一清单口径。
  - Phase-3: 将审读机制纳入工程例行治理（周度更新）。
- Technical Risks:
  - 风险-1: 文档规模大，单批次审读深度不足。
  - 风险-2: 历史专题语义复杂，替代链容易遗漏。
  - 风险-3: 代码快速演进导致“审读完成后再漂移”。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-ENGINEERING-012 | TASK-ENGINEERING-020/024 | `test_tier_required` | 清单文件数量/覆盖率核对、入口文档已读抽样 | 全量 PRD 审读可追溯性 |
| PRD-ENGINEERING-013 | TASK-ENGINEERING-021/022 | `test_tier_required` | 代码一致性抽样、偏差回写核验、重复项处理记录检查 | 文档行为与实现一致性 |
| PRD-ENGINEERING-014 | TASK-ENGINEERING-022/023/024 | `test_tier_required` + `test_tier_full` | 引用可达扫描、上下游链路核验、门禁脚本执行 | 文档树清晰度与跨模块一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-ENG-PRD-001 | 以“逐篇已读清单”作为全量审读唯一执行面 | 仅模块级进度百分比 | 逐篇清单可审计、可复核、可追责。 |
| DEC-ENG-PRD-002 | 文档偏差按“代码为准”回写 | 以文档为准要求改代码 | 当前目标是恢复文档与实现一致性，先收口事实再讨论重构。 |
| DEC-ENG-PRD-003 | 单一清单 + 文档内审计轮次标记 | active/archive 双轨清单 | 归档目录已移除，单一清单更易维护且与审计轮次一致。 |
