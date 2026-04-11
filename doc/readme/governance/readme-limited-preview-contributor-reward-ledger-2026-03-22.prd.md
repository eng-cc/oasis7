# oasis7 Limited Preview Contributor Reward Ledger（2026-03-22）

- 对应设计文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: oasis7 已有 early contributor reward 的评分模板和禁语边界，但还缺少“真实一轮怎么记、怎么审、怎么回填发放记录”的统一 ledger。没有 ledger，团队会在真实发放前退回聊天记录、issue 评论或临时表格，导致奖励审核与执行不可审计。
- Proposed Solution: 新增 round-based contributor reward ledger 专题，固定 round meta、逐条贡献记录、producer 审批状态、实际发放引用与归档字段，让 limited preview 的真实贡献奖励具备可复用、可复核、可归档的台账模板。
- Success Criteria:
  - SC-1: 每轮真实贡献奖励都能用同一份 ledger 模板记录 `Round ID / Candidate ID / Window / Status`。
  - SC-2: 每条奖励建议都必须含 contributor、`Oasis ID`、`Reward Account`、evidence link、score、recommended band、review status。
  - SC-3: 审批后的真实发放必须回填 `Approval ID / Actual Amount / Distribution Ref`，不允许只停留在“口头批准”。
  - SC-4: 归档前必须输出 round summary、band summary 与 unresolved items，保证后续治理复盘可追溯。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要在 limited preview round 结束后汇总真实贡献并形成正式台账。
  - `producer_system_designer`：需要逐条审看奖励建议档位与审批结果，而不是阅读零散聊天记录。
  - reward execution owner：需要根据审批结果回填实际发放数量和执行引用。
  - `qa_engineer`：需要在必要时复核台账字段完整性与证据可达性。
- User Scenarios & Frequency:
  - 每轮 limited preview 结束后至少维护 1 份 ledger。
  - 每次 producer 完成一轮审批时，更新 review status 与 approval 字段。
  - 每次实际发放完成时，补回 distribution ref 并关闭本轮 ledger。
  - 每次某条贡献来自 GitHub PR 时，优先从 PR intake block 导入 `Oasis ID + Reward Account`，减少后补沟通。
- User Stories:
  - PRD-README-LTRL-001: As a `liveops_community`, I want one round-based ledger template, so that real contribution review no longer depends on ad-hoc notes.
  - PRD-README-LTRL-002: As a `producer_system_designer`, I want each contribution row to show score, band, and approval status, so that I can approve conservatively and audibly.
  - PRD-README-LTRL-003: As an execution owner, I want approved rows to carry actual amount and distribution reference fields, so that distribution closure is traceable later.
- Critical User Flows:
  1. Flow-LTRL-001: `结束一轮 limited preview -> liveops 汇总可计分贡献 -> 填写 round meta 与 ledger rows -> 标记 draft`
  2. Flow-LTRL-002: `producer 审阅逐条 recommended band -> 标记 approved/rejected/deferred -> 输出 round summary`
  3. Flow-LTRL-003: `execution owner 根据 approved rows 执行发放 -> 回填 actual amount / distribution ref -> 标记 distributed`
  4. Flow-LTRL-004: `本轮关闭 -> 记录 unresolved items / next action -> 归档供 future governance review`
  5. Flow-LTRL-005: `某条贡献来自 GitHub PR -> 从 PR reward intake block 读取 Oasis ID / Reward Account / evidence link -> 生成或补齐 ledger row -> 再进入 producer review`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| Round Meta | `round_id`、`candidate_id`、`window`、`status`、`owner_role` | 初始化本轮 ledger 头部信息 | `planned -> draft -> under_review -> closed` | 每轮只能有 1 个主 ledger | `liveops_community` 维护 |
| Ledger Row | `ledger_id`、`contributor`、`oasis_id`、`reward_account`、`source_link`、`contribution_type`、`total_score`、`recommended_band` | 逐条录入真实贡献 | `captured -> reviewed` | 用户侧领取身份统一写 `Oasis ID`；同 contributor 可多条，但 `ledger_id` 必须唯一 | `liveops_community` 维护 |
| PR Intake Import | `reward_review_request`、`oasis_id`、`reward_account`、`evidence_link` | 从 GitHub PR reward intake block 导入或补齐身份字段 | `deleted -> submitted -> imported -> reviewed` | 仅当 source type=`PR` 且作者保留该区块并主动申请 reward review 时使用；raw `public key` 不进入名称层 | `liveops_community` 导入 |
| Producer Review | `review_status`、`producer_decision`、`approval_id` | 审阅并批准/拒绝/延后 | `reviewed -> approved/rejected/deferred` | 缺证据默认不得批准 | `producer_system_designer` 决策 |
| Distribution Closure | `actual_amount`、`distribution_ref`、`distribution_date` | 回填真实执行引用 | `approved -> distributed` | 未执行前允许留空；执行后必须补全 | execution owner 回填 |
| Round Summary | `band_totals`、`approved_rows`、`distributed_rows`、`unresolved_items` | 输出本轮汇总与遗留事项 | `draft -> summarized -> archived` | 汇总值按 ledger rows 聚合 | `liveops_community` 汇总，producer 审核 |
- Acceptance Criteria:
  - AC-1: ledger 模板必须包含 `Meta`、`Ledger`、`Band Summary`、`Approval Summary`、`Distribution Closure`、`Next Actions` 六个区块。
  - AC-2: 每条 ledger row 必须至少包含 `Ledger ID`、`Contributor`、`Oasis ID`、`Reward Account`、`Source Link`、`Contribution Type`、`Total Score`、`Recommended Band`、`Review Status`。
  - AC-3: 任何 `approved` row 在后续实际发放后都必须回填 `Approval ID / Actual Amount / Distribution Ref`。
  - AC-4: 模板必须允许 `rejected / deferred / distributed / archived` 等状态，而不是只记录“建议发放”。
  - AC-5: 模板不得包含任何固定 token/point 汇率、公开营销文案或 `play-to-earn` 叙事。
  - AC-6: 若 row 来源是 GitHub PR，ledger 必须能回溯到 PR intake block 里的 `Oasis ID + Reward Account`，或显式记录为何改为 `deferred`。
- Non-Goals:
  - 本专题不决定每个档位对应的具体 token 数量。
  - 不替代 `readme-limited-preview-contributor-reward-pack-2026-03-22` 的评分规则定义。
  - 不替代创世参数或 treasury 审计模板。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 本专题是 reward pack 的执行收口层。`readme-limited-preview-contributor-reward-pack` 定义“怎么评分”，本 ledger 定义“真实一轮怎么记账、怎么审、怎么回填发放记录、怎么归档”。
- Integration Points:
  - `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
  - `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
  - `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- Edge Cases & Error Handling:
  - 同一贡献被多人重复提交：只保留主记录 full row，其余记录在 `Notes` 标记 duplicate。
  - 同一 contributor 有多条有效贡献：允许多行，但每行必须独立 `Ledger ID`。
  - producer 已批准但发放尚未执行：`Actual Amount / Distribution Ref` 可暂空，但 `Review Status` 不能写成 `distributed`。
  - 没有链上 reward account：允许先 `deferred`，不得跳过账户字段直接执行。
  - PR 来源且已删除 reward intake block：视为未申请 reward review；若后续要进入台账，必须补齐 `Oasis ID + Reward Account` 并保留 approved follow-up 记录，否则不进入 producer 审批。
  - 若只拿到 raw `public key` 或账户派生材料，必须先收口为 `Oasis ID + Reward Account`，不得把 raw `public key` 直接作为台账名称层字段。
  - 证据链接失效：该 row 退回 `draft` 或 `deferred`，不得进入正式批准。
- Non-Functional Requirements:
  - NFR-LTRL-1: 每轮 ledger 的 `Round ID / Candidate ID / Window / Status` 完整率必须为 `100%`。
  - NFR-LTRL-2: 每条非 `rejected` row 至少包含 1 个有效 evidence/source link。
  - NFR-LTRL-3: 每条 `distributed` row 必须具备 `Approval ID` 与 `Distribution Ref`。
  - NFR-LTRL-4: ledger 模板必须可直接复制复用到下一轮，而无需重新设计字段。
- Security & Privacy: ledger 只记录公开 handle、`Oasis ID`、证据链接、链上奖励账户与必要审批引用；不记录私人聊天原文或不必要个人隐私。raw `public key` 仅保留在底层签名/账户绑定流程中，不作为奖励台账名称字段。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 先交付 ledger 模板并接回 `readme` / `p2p token` 主追踪。
  - v1.1: 跑完首轮真实 ledger 后，根据遗漏字段补小修。
  - v2.0: 若 future governance 需要 fully on-chain closure，再补 governance proposal / treasury reference 字段。
- Technical Risks:
  - 风险-1: 若 ledger 不回填实际执行引用，模板仍会退化成“漂亮的建议清单”。
  - 风险-2: 若 round meta 不冻结，后续治理复盘会丢失 candidate / window 上下文。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-LTRL-001 | LTRL-1/2 | `test_tier_required` | 模板结构、round meta、ledger row 字段完整性检查 | liveops 真实贡献记录一致性 |
| PRD-README-LTRL-002 | LTRL-2/3 | `test_tier_required` | review status、approval 字段与 round summary 检查 | producer 审批可追溯性 |
| PRD-README-LTRL-003 | LTRL-3 | `test_tier_required` | `Actual Amount / Distribution Ref` 回填字段与 archive 流程检查 | 真实发放闭环与后续治理复盘 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-LTRL-001 | 用独立 round ledger 承接真实贡献奖励结算 | 继续把真实发放散落在 issue、聊天和临时表格 | 没有统一 ledger 就没有统一审计面。 |
| DEC-LTRL-002 | ledger 行同时记录建议档位与真实执行引用 | 只记录建议档位，不记录后续发放引用 | 不记录执行引用就无法形成完整闭环。 |
| DEC-LTRL-003 | 允许 `deferred / rejected / distributed` 多状态 | 所有 row 只有“推荐/未推荐”二元状态 | 真实执行一定存在待补资料、被拒或已执行三类分叉。 |
| DEC-LTRL-004 | reward claimant 的用户侧身份统一写为 `Oasis ID`，`Reward Account` 只保留为执行字段 | 直接把 raw `public key` 或账户派生材料写进台账名称层 | 台账要先服务审核与归档阅读，claimant identity 必须可读；底层签名材料应继续留在技术专题。 |
| DEC-LTRL-005 | 对于 GitHub PR 来源的贡献，优先从 PR intake block 导入 `Oasis ID + Reward Account` | 继续在 ledger 建档后再到评论/私聊里补身份字段 | 让 PR 在提交时就带齐 claimant-facing 字段，可以减少二次追问，并让 ledger 更快形成可审阅条目。 |
