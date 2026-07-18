# oasis7: Moltbook 首批发帖草案包（2026-03-19）

- 对应设计文档: `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-moltbook-post-drafts-2026-03-19.project.md`

审计轮次: 6

## 1. Executive Summary
- Problem Statement: 已有 Moltbook 平台化推广方案，但如果首轮冷启动贴文仍靠现场即兴撰写，`liveops_community` 很容易在简介、CTA、限制和评论回复里超出当前技术预览 claim envelope。
- Proposed Solution: 基于已批准的 Moltbook 推广方案，沉淀一份首批发帖草案包，包含 6 条英文主贴、每条主贴的首条补充评论、常见评论回复模板、禁宣称提醒和发布顺序。
- Success Criteria:
  - SC-1: 至少提供 6 条适合 Moltbook 的英文主贴草案。
  - SC-2: 每条主贴都带有 proof-first 叙事、CTA 和限制语句。
  - SC-3: 提供高频评论回复模板，降低临场 overclaim 风险。
  - SC-4: 全部草案都通过 `producer_system_designer` 审核链约束。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`: 需要一套可直接拿去排期和微调的原生文案。
  - `producer_system_designer`: 需要对外文案在发布前具备稳定边界。
  - Moltbook 上的 builder / creator / observer: 需要读到像平台原生内容，而不是生硬的广告文案。
- User Scenarios & Frequency:
  - 首轮开账号后的前 2 周：直接按草案包发 4-6 条主贴。
  - 评论区出现高频问题时：优先复用回复模板，再按实际情况微调。
  - 状态变化时：只改草案包相关字段，不重写整套策略。
- User Stories:
  - PRD-README-MOLT-DRAFT-001: As a `liveops_community`, I want a first-wave post pack, so that Moltbook publishing can start from reviewed copy instead of live improvisation.
  - PRD-README-MOLT-DRAFT-002: As a `producer_system_designer`, I want each post to include safe boundaries and CTA, so that outreach stays within current evidence.
  - PRD-README-MOLT-DRAFT-003: As a Moltbook builder or creator, I want posts that feel native and specific, so that I can tell what oasis7 actually does.
- Critical User Flows:
  1. `读取推广方案 -> 选择发布周目标 -> 从草案包挑选对应主贴`
  2. `发布主贴 -> 在评论区追加首条补充评论 -> 根据回复模板承接互动`
  3. `碰到越界提问 -> 使用禁宣称规则和升级路径 -> 回流 owner`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 排序/计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 主贴草案 | `post_id`、`goal`、`copy`、`cta`、`do_not_say` | 生成可发布的首条文案 | `draft -> reviewed -> publish_ready` | 按 identity -> proof -> builder hook 排序 | `liveops_community` 起草，`producer_system_designer` 审核 |
| 补充评论 | `comment_id`、`linked_post_id`、`asset_note`、`extra_link` | 在主贴下补更硬的证据或限制说明 | `draft -> reviewed -> publish_ready` | 每贴 1 条优先 | `liveops_community` 维护 |
| 回复模板 | `reply_type`、`safe_answer`、`escalation_rule` | 评论互动时快速复用 | `draft -> reviewed -> active` | 先答状态，再答机制，再答合作 | `liveops_community` 使用 |
| 禁宣称提醒 | `forbidden_claim`、`replacement_copy` | 发帖前复核 | `defined -> adopted` | 高风险承诺优先 | `producer_system_designer` 拍板 |
- Acceptance Criteria:
  - AC-1: 产出 PRD / Design / Project / Draft Pack 文档。
  - AC-2: Draft Pack 至少包含 6 条主贴、6 条补充评论和 6 类回复模板。
  - AC-3: 每条主贴必须显式保持“limited playable technical preview”边界。
  - AC-4: 文案必须保持 Moltbook-native，而不是照搬公告口气。
- Non-Goals:
  - 不在本专题中批准真实发布日期或执行时间。
  - 不保证每条草案都适合一字不改直接发布。
  - 不在本专题中创建图片、视频或设计资产。

## 3. AI System Requirements (If Applicable)
- Tool Requirements:
  - `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
  - `README.md`
  - `site/index.html`
- Evaluation Strategy:
  - 检查每条主贴是否有定位、证据点、限制和 CTA。
  - 检查回复模板是否覆盖“是不是已上线”“怎么体验”“是否已集成 Moltbook”等高频问题。
  - 若文案出现“play now”“live now”“official Moltbook integration”等字样，判为不通过。

## 4. Technical Specifications
- Architecture Overview: 该专题是 Moltbook 推广方案的执行层文案包，不替代平台策略本身；它面向 `liveops_community` 提供首批原生主贴和评论回复模板。
- Integration Points:
  - `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.md`
  - `README.md`
  - `site/index.html`
  - `doc/product/player-entry-distribution/player-access-mode-contract-2026-03-19.prd.md`
- Edge Cases & Error Handling:
  - 若平台出现新话题或贴型变化：在不越界的前提下可微调 CTA 和 opening hook，但不得改 claim envelope。
  - 若用户直接要求试玩链接：统一回复技术预览口径，不伪造 waitlist 或 playtest 承诺。
  - 若用户追问集成或合作：只保留兴趣并升级 owner，不给未批准承诺。
- Non-Functional Requirements:
  - NFR-MOLTDRAFT-1: 文案包在 15 分钟内可完成初次阅读和排期。
  - NFR-MOLTDRAFT-2: 每条主贴尽量控制在短帖长度，适合原生 feed 阅读。
  - NFR-MOLTDRAFT-3: 所有草案必须保持 proof-first、限制明确、CTA 单一。
- Security & Privacy: 不泄露内部工程实现细节、非公开合作和敏感运行参数。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP (`MOLTDRAFT-1`): 首批 6 条主贴和回复模板。
  - v1.1 (`MOLTDRAFT-2`): 基于首轮互动结果做 A/B 变体。
  - v2.0 (`MOLTDRAFT-3`): 抽象成多平台短帖文案模板库。
- Technical Risks:
  - 风险-1: 如果文案太像新闻稿，会不适合 Moltbook 原生语境。
  - 风险-2: 如果 CTA 太重，会被看成外链广告而非 agent-native 内容。
  - 风险-3: 如果限制写得不够明确，评论区容易出现二次 overclaim。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| `PRD-README-MOLT-DRAFT-001` | `TASK-README-015` | `test_tier_required` | 检查存在 6 条主贴与补充评论 | 首批文案完备性 |
| `PRD-README-MOLT-DRAFT-002` | `TASK-README-015` | `test_tier_required` | 检查每条文案均含限制语句和 CTA | 对外口径安全性 |
| `PRD-README-MOLT-DRAFT-003` | `TASK-README-015` | `test_tier_required` | 检查回复模板覆盖高频问答 | 评论区承接能力 |
| `PRD-README-MOLT-DRAFT-001/002/003` | `TASK-README-015` | `test_tier_required` | `./scripts/doc-governance-check.sh` | 文档治理一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| `DEC-MOLTDRAFT-001` | 先做英文原生短帖，再考虑长文外链 | 首轮就发长篇公告式文案 | Moltbook 当前 feed 语境更偏原生短内容。 |
| `DEC-MOLTDRAFT-002` | 每条主贴都配一条补充评论 | 只发主贴不补证据 | 补充评论更适合承载链接、资产和限制说明。 |
| `DEC-MOLTDRAFT-003` | 同时沉淀回复模板 | 评论区临场发挥 | 评论区最容易出现过度承诺，必须先设边界。 |
