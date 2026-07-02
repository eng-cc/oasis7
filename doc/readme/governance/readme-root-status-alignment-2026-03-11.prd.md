# oasis7: 根 README 公开状态口径对齐（2026-03-11）

- 对应设计文档: `doc/readme/governance/readme-root-status-alignment-2026-03-11.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-root-status-alignment-2026-03-11.project.md`

审计轮次: 4

## 治理状态（2026-06-18）
- 本专题已完成，保留为 2026-03-11 根 README 公开状态对齐的历史证据。
- 当前公开状态真值以根 `README.md` 为准；README 治理与沟通节奏继续由 `doc/readme/governance/readme-project-overview-whitepaper-2026-04-25.md`、`doc/readme/governance/readme-consistency-audit-checklist-2026-03-11.prd.md`、`doc/readme/governance/readme-quarterly-review-cycle-2026-03-11.prd.md` 和 release communication surfaces 承接。
- 本文件不保留当前下一步；若公开状态发生变化，应更新根 `README.md` 及当前治理 / release communication 入口，而不是在此历史专题追加新承诺。

## 1. Executive Summary
- Problem Statement: 根 `README.md` 仍保留“基础推进游戏层已有，正在测试完善阶段”这类旧表述，已经落后于当前 `site` 与 release communication 链中的“技术预览（尚不可玩）/ 公告准备态”统一口径。
- Proposed Solution: 对齐根 `README.md` 的项目状态描述，明确当前为技术预览、尚不可玩、公开公告仍在准备中，并引导读者按站点 / 文档入口理解当前阶段。
- Success Criteria:
  - SC-1: 根 `README.md` 的项目状态与 `site` 主页口径一致。
  - SC-2: 根 `README.md` 明确区分技术预览、预览构建包、正式公告准备态三层含义。
  - SC-3: `readme` 主项目能够追踪这次状态对齐任务。

## 2. User Experience & Functionality
- User Personas:
  - 仓库访客：需要从第一屏就理解当前不是正式可玩发布。
  - `producer_system_designer`：需要保证仓库首页承诺不超出当前候选状态。
  - `liveops_community`：需要仓库首页与站点公开入口保持同一口径。
- User Scenarios & Frequency:
  - 首次打开仓库首页时：快速理解当前状态。
  - 对外分享仓库链接时：避免将仓库首页误读为正式发布页。
  - 版本状态变化后：更新统一状态说明。
- User Stories:
  - PRD-README-008: As a 仓库访客, I want the root README to reflect the current preview posture, so that I do not mistake the repo for a live release landing page.
  - PRD-README-009: As a `producer_system_designer`, I want repo-home copy aligned with site and communication docs, so that promises stay consistent.
- Critical User Flows:
  1. `打开 README -> 读取项目状态 -> 理解技术预览 / 尚不可玩 / 公告准备态`
  2. `需要进一步信息 -> 跳转 site / doc 入口 -> 继续查看详细说明`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| root status summary | 当前状态、可玩性、公开说明状态、下一入口 | 更新 README 状态段 | `stale -> aligned` | 先状态，再边界，再入口 | `producer_system_designer` 审核 |
| entry links | site / docs / testing 入口 | 给出后续阅读路径 | `implicit -> explicit` | 公开入口优先 | 文档 owner 可维护 |
- Acceptance Criteria:
  - AC-1: 产出根 README 状态对齐专题 PRD / Design / Project。
  - AC-2: `README.md` 的项目状态段更新为当前统一口径。
  - AC-3: `doc/readme/project.md` 能追踪该任务。
- Non-Goals:
  - 不重写整个 README。
  - 不把内部治理文档直接暴露为首页主入口。
  - 不宣称正式发布已发生。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题位于 readme/governance，聚焦根 README 与 site / communication brief 的状态对齐。
- Integration Points:
  - `README.md`
  - `doc/readme/governance/readme-release-candidate-communication-brief-2026-03-11.prd.md`
  - `site/index.html`
- Edge Cases & Error Handling:
  - 如果后续正式公告发布：README 状态段需同步更新，不能长期保留“准备态”。
  - 如果候选状态变化：README 需跟随最新公开口径更新。
- Non-Functional Requirements:
  - NFR-ROOT-1: README 状态段在 1 屏内可读完。
  - NFR-ROOT-2: 状态表述不得与 site 主页冲突。
- Security & Privacy: README 只公开当前允许公开的阶段信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`ROOT-1`): 对齐根 README 状态段。
  - v1.1 (`ROOT-2`): 若正式公告发布，再更新为正式入口。
- Technical Risks:
  - 风险-1: 根 README 若长期滞后，会再次制造公开口径分叉。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-008 | `TASK-README-010` | `test_tier_required` | 检查 README 状态段含技术预览 / 尚不可玩 / 公告准备态 | 仓库首页状态理解 |
| PRD-README-009 | `TASK-README-010` | `test_tier_required` | 检查 README 与 site / brief 口径一致 | 公开口径一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-ROOT-001` | 只对齐状态段，不重写整份 README | 为了一次状态修正重做整份首页 | 最小改动即可消除口径冲突。 |
| `DEC-ROOT-002` | README 显式写出“公告准备态” | README 只写技术预览，不补公开说明状态 | 需要让仓库首页跟上新 release communication 链。 |
