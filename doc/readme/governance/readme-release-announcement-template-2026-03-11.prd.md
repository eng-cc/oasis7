# oasis7: 公告 / Changelog 模板（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-release-announcement-template-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-release-announcement-template-2026-03-11.project.md`

审计轮次: 4

## 1. Executive Summary
- Problem Statement: 首份 announcement / changelog 底稿已经存在，但如果不进一步抽象为模板，未来每次候选仍需要从零重构标题、摘要、限制、FAQ 与下一步结构。
- Proposed Solution: 从首份底稿中抽象一份公告 / changelog 模板，固定 draft 状态、摘要结构、限制段、FAQ、下一步与审核字段，供后续候选直接实例化。
- Success Criteria:
  - SC-1: 模板覆盖 `draft` 状态、Summary、Known Limitations、FAQ、Next Steps、Approval 六类必填字段。
  - SC-2: 模板可与现有 announcement draft 一一映射。
  - SC-3: 后续候选只需填内容而无需重新设计文风骨架。
  - SC-4: readme 主项目能追踪该模板化任务。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要稳定复用的公告底稿模板。
  - `producer_system_designer`：需要固定审核面，快速判断是否越界。
  - `qa_engineer`：需要知道外部文案仍能回链到内部依据。
- User Scenarios & Frequency:
  - 新候选进入公告准备态时：从模板生成新底稿。
  - 审核时：核对模板必填字段是否完整。
  - 状态反复变化时：用模板保持结构一致。
- User Stories:
  - PRD-README-ANN-TEMPLATE-001: As a `liveops_community`, I want a reusable announcement draft template, so that each candidate starts from the same safe structure.
  - PRD-README-ANN-TEMPLATE-002: As a `producer_system_designer`, I want the template to separate summary, limitations, and next steps, so that review remains fast and bounded.
  - PRD-README-ANN-TEMPLATE-003: As a `qa_engineer`, I want source-link placeholders in the template, so that public-facing copy remains traceable.
- Critical User Flows:
  1. `复制模板 -> 填写候选 ID / 状态 / 来源 -> 生成新 draft`
  2. `补齐摘要 / 限制 / FAQ / 下一步 -> 提交审核`
  3. `审核通过后作为后续正式文案底稿`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| draft header | 标题、候选 ID、状态、来源、owner | 复制模板后填写 | `template -> instantiated -> reviewed` | 头部字段必须先完整 | `liveops_community` 维护 |
| narrative sections | Summary、What This Means、Highlights、Limitations、FAQ、Next Steps | 填写实际内容 | `empty -> drafted` | 先事实后限制后行动 | `producer_system_designer` 审核 |
| source links | brief / go-no-go / handoff | 保持可追溯 | `missing -> linked` | 缺少来源不得 approved | `qa_engineer` 可复核 |
- Acceptance Criteria:
  - AC-1: 产出 announcement 模板专题 PRD / Design / Project。
  - AC-2: 产出一份可直接复制复用的公告 / changelog 模板。
  - AC-3: 模板显式包含 source links 与 review status。
  - AC-4: `doc/readme/project.md` 能追踪模板任务完成。
- Non-Goals:
  - 不再生成新的候选实例。
  - 不等于正式公告模板最终版。
  - 不改变现有 draft 结论。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 模板位于 readme/governance，复用首份 announcement draft 的结构，把单次实例上升为标准化文案底稿能力。
- Integration Points:
  - `doc/readme/governance/readme-release-announcement-draft-2026-03-11.prd.md`
  - `doc/readme/governance/readme-release-communication-template-2026-03-11.md`
  - `doc/readme/project.md`
- Edge Cases & Error Handling:
  - 若候选无内部来源：模板必须阻止进入 `approved`。
  - 某节不适用：必须写 `N/A + reason`。
  - 状态变化：模板要求同步更新时间与状态字段。
- Non-Functional Requirements:
  - NFR-ANN-T-1: 模板实例化耗时 <= 15 分钟。
  - NFR-ANN-T-2: 模板结构与现有 draft 对齐率 100%。
  - NFR-ANN-T-3: 模板必须显式标注 `draft/reviewed/publish_ready` 状态。
- Security & Privacy: 模板不预填敏感信息，只保留安全占位字段。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`ANN-T1`): 抽出首份 announcement draft 模板。
  - v1.1 (`ANN-T2`): 在下个候选上验证模板可复用性。
  - v2.0 (`ANN-T3`): 视发布节奏决定是否升级为正式公告模板体系。
- Technical Risks:
  - 风险-1: 若模板过重，会降低实际使用率。
  - 风险-2: 若模板与实例脱节，会重新回到手写模式。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-ANN-TEMPLATE-001 | `TASK-README-009` | `test_tier_required` | 检查模板与现有底稿结构互链 | 公告底稿模板复用能力 |
| PRD-README-ANN-TEMPLATE-002 | `TASK-README-009` | `test_tier_required` | 检查限制/FAQ/下一步段落存在 | 审核一致性 |
| PRD-README-ANN-TEMPLATE-003 | `TASK-README-009` | `test_tier_required` | 检查 source-link 与 review status 字段存在 | 可追溯性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-ANN-T-001` | 从首份底稿反推模板 | 继续未来每次手写公告底稿 | 现有结构已经足够稳定，适合模板化。 |
| `DEC-ANN-T-002` | 模板保留 source links 与 review status | 只保留面向外部读者的正文 | 模板仍需服务内部审核与追溯。 |
| `DEC-ANN-T-003` | 模板继续由 liveops 起草、producer 审核 | 将审核链移出模板体系 | 审核链本身就是模板能力的一部分。 |
