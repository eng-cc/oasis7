# oasis7 Media Promoter Oasis Coin Incentive Pack（2026-04-12）

- 对应设计文档: `doc/readme/governance/readme-media-promoter-oasis-coin-incentive-pack-2026-04-12.design.md`
- 对应项目管理文档: `doc/readme/governance/readme-media-promoter-oasis-coin-incentive-pack-2026-04-12.project.md`

审计轮次: 1

## 1. Executive Summary
- Problem Statement: `oasis7` 已经有 early contributor reward 和渠道 runbook，但还没有一套专门面向媒体、KOL、自媒体创作者、社区搬运号和普通宣传参与者的绿洲币激励机制。没有这套机制，宣传生态要么退化成临时买量，要么只能靠情绪支持，无法把“宣传方也是参与者和受益者”变成可执行规则。
- Proposed Solution: 在 `readme/governance` 建立独立的 media promoter Oasis Coin incentive pack，用统一的覆盖对象分层、证据字段、质量评分、生态回流、审批链、发放回填、反作弊和禁语边界，定义媒体推广者如何按贡献而不是按流量被审计和激励。
- Success Criteria:
  - SC-1: 每条进入奖励评审的媒体推广记录都必须包含 `asset_url`、归档证据、事实边界检查、奖励账户和 reviewer。
  - SC-2: 方案显式覆盖外部媒体、KOL、自媒体创作者、社区搬运号和普通宣传参与者，并明确可计分/不可计分边界。
  - SC-3: 原始播放量、阅读量、点赞量不能单独触发绿洲币奖励建议，所有建议都必须附带内容质量与生态回流说明。
  - SC-4: 对外文案不得出现固定 token/播放量汇率、固定“发一条给多少币”或“只要转发就能领币”的承诺。
  - SC-5: 每轮已批准的奖励记录都必须具备 `Approval ID` 与 `Distribution Ref`，并可回链到原始宣传资产。

## 2. User Experience & Functionality
- User Personas:
  - `liveops_community`：需要把宣传生态激励从临场判断收成可执行、可复盘、可控风险的操作包。
  - `producer_system_designer`：需要确保绿洲币激励奖励的是有助于宣传生态长期积累的真实贡献，而不是短期刷量。
  - 媒体推广者：包括媒体、KOL、自媒体创作者、社区搬运号和普通宣传参与者，需要知道什么样的宣传贡献能进入奖励建议池。
- User Scenarios & Frequency:
  - 新资产入池：每次出现可验证的宣传资产后记录一次。
  - 轮次复核：每 1-2 周做一次集中评分和 producer 审核。
  - 对外协作说明：每次向潜在宣传方解释激励规则前，先用禁语清单复核。
  - 发放回填：每次 producer 批准后，由执行 owner 回填分发结果。
- User Stories:
  - PRD-README-MPI-001: As a `liveops_community`, I want one scoring framework for all media-promoter lanes, so that external media and ordinary promoters can be reviewed under the same audited rules.
  - PRD-README-MPI-002: As a `producer_system_designer`, I want Oasis Coin recommendations tied to contribution depth, claim safety, and ecosystem return, so that rewards do not collapse into paid traffic.
  - PRD-README-MPI-003: As a media promoter, I want clear countable and non-countable behaviors, so that I understand how to contribute in a way that genuinely helps the ecosystem.
  - PRD-README-MPI-004: As a `liveops_community`, I want every rewarded asset to be traceable back to a real promotion artifact and follow-up effect, so that the incentive loop can improve future channel strategy.
- Critical User Flows:
  1. Flow-MPI-001: `发现宣传资产 -> 记录 channel / asset_url / creator / reward_account -> 进入待筛选池`
  2. Flow-MPI-002: `检查事实边界与禁语 -> 识别是否存在 overclaim / 抄袭 / 刷量 -> 通过后进入评分`
  3. Flow-MPI-003: `按宣传资产类型给基础分 -> 叠加原创性/讨论质量/生态回流加分 -> 输出建议档位`
  4. Flow-MPI-004: `producer_system_designer 审核建议档位 -> 批准或驳回 -> execution owner 回填 distribution ref -> 归档`
  5. Flow-MPI-005: `外部问“发一条给多少币” -> 使用 safe copy 解释贡献审核制 -> 阻断任何固定汇率或上线承诺`
- Functional Specification Matrix:
| 功能点 | 字段定义 | 动作行为 | 状态转换 | 计算规则 | 权限逻辑 |
| --- | --- | --- | --- | --- | --- |
| 覆盖对象分层 | `promoter_lane`、`channel`、`creator_handle`、`reward_account` | 按深度报道/原创创作/社区扩散/普通宣传参与分类 | `captured -> classified` | 先按贡献形态分类，不按平台贵贱分类 | `liveops_community` 维护 |
| 宣传资产 intake | `asset_url`、`archive_proof`、`publish_at`、`audience_fit_note` | 记录资产并准备后续评分 | `captured -> screened` | 缺链接或归档证据不得进入评分 | `liveops_community` 维护 |
| 事实边界检查 | `claim_safety_status`、`forbidden_phrase_hit`、`preview_boundary_note` | 过滤越界宣传与错误表述 | `screened -> blocked/passed` | 先过边界，再谈奖励 | `liveops_community` 初筛，`producer_system_designer` 可否决 |
| 评分模型 | `base_score`、`quality_bonus`、`discussion_bonus`、`ecosystem_bonus`、`penalty` | 计算建议档位 | `passed -> scored -> recommended` | 原始播放量只能作为辅助说明，不能独立决定档位 | `liveops_community` 初评 |
| 反作弊检查 | `fraud_flag`、`duplicate_flag`、`plagiarism_flag`、`traffic_risk_note` | 标记刷量、抄袭、重复申报 | `screened -> held/rejected/passed` | 命中严重作弊默认驳回 | `liveops_community` 初筛，`producer_system_designer` 审核 |
| 奖励审批与发放 | `recommended_band`、`approval_id`、`distribution_ref`、`archive_note` | producer 审批后回填发放结果 | `recommended -> approved/rejected -> distributed -> archived` | 只输出 `eligible-small/medium/large` 或 `no-token-recommendation` | `producer_system_designer` 审批，distribution owner 回填 |
| 对外说明模板 | `safe_phrase`、`forbidden_phrase`、`faq_answer` | 规范外部说明 | `draft -> approved -> adopted` | 命中禁语即阻断 | `liveops_community` 起草，`producer_system_designer` 审核 |
- Acceptance Criteria:
  - AC-1: 专题必须明确覆盖对象分层，至少包含媒体、KOL、自媒体创作者、社区搬运号和普通宣传参与者。
  - AC-2: 专题必须明确以下行为不能单独获得奖励建议：纯转发无新内容、纯播放量、买量、互刷、抄袭搬运、单纯情绪吹捧。
  - AC-3: 每条建议记录必须至少包含 `asset_url`、归档证据、事实边界检查、原创性或新增价值说明、reward account。
  - AC-4: 评分模型必须显式包含内容质量、讨论质量、生态回流和反作弊扣分，且 raw reach 不能单独决定奖励档位。
  - AC-5: 奖励档位必须继续使用 `no-token-recommendation / eligible-small / eligible-medium / eligible-large`，不得公开固定绿洲币数额或固定换算比例。
  - AC-6: 对外模板必须显式阻断 `发一条就发币`、`按播放量发币`、`转发领币`、`已上线可玩`、`官方买量` 等表述。
  - AC-7: producer 审批与 distribution ref 回填流程必须在文档中显式出现，并可接入现有 reward ledger / distribution 路径。
- Non-Goals:
  - 本专题不决定具体每档绿洲币额度。
  - 不把宣传激励写成广告投放 rate card。
  - 不替代 existing early contributor reward pack；后者仍覆盖 bug、PR、长时样本等非宣传类贡献。
  - 不承诺 `oasis7` 已正式上线、开放公测或进入买量阶段。

## 3. AI System Requirements (If Applicable)
- Tool Requirements: 不适用。
- Evaluation Strategy: 不适用。

## 4. Technical Specifications
- Architecture Overview: 该专题位于 `readme/governance`，作为宣传生态的激励治理层，承接各渠道宣传资产，把外部传播贡献转成可审计的绿洲币奖励建议，再回流 producer 审批与后续 distribution 流程。
- Integration Points:
  - `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
  - `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
  - `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.prd.md`
  - `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`
  - `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.prd.md`
- Edge Cases & Error Handling:
  - 同一内容跨平台重复发布：默认只保留一个主资产 full credit，其他平台记录为补充证明或小额加分，不重复给 full credit。
  - 高播放但低质量：若只有播放量，没有准确性、原创性或生态回流，默认 `no-token-recommendation`。
  - 越界宣传：若把 `limited playable technical preview` 说成已上线、公测或官方深度集成，直接阻断奖励。
  - 资产被删除或转私密：先进入 `held`，补齐归档证据后再评审。
  - 多人共同产出：允许一条资产对应多个贡献者，但必须拆清分工并避免重复 full credit。
  - 刷量或抄袭嫌疑：标记 `fraud_flag` 并暂停审批，未澄清前不得发放。
- Non-Functional Requirements:
  - NFR-MPI-1: 每条非拒绝记录必须包含可访问的宣传资产链接或可审计归档证据。
  - NFR-MPI-2: 每条已发放记录必须包含 `Approval ID` 与 `Distribution Ref`。
  - NFR-MPI-3: 对外固定汇率、固定发币承诺和上线 overclaim 命中率必须为 `0`。
  - NFR-MPI-4: 每轮候选奖励的 producer 复核 SLA 目标为 `<= 7 days`。
  - NFR-MPI-5: 被判定为买量、互刷、抄袭或虚假宣传的资产，不得漏入已批准发放记录。
- Security & Privacy: 只记录公开 handle、公开资产链接、最小必要的奖励账户与审批引用；不得要求宣传方提供不必要的私密身份信息。

## 5. Risks & Roadmap
- Phased Rollout:
  - MVP: 固定覆盖对象、评分维度、证据字段、审批链、禁语与反作弊规则。
  - v1.1: 跑完首轮真实媒体推广者审核后，根据误判和高价值样本调整权重。
  - v2.0: 如后续形成长期 creator roster，再补稳定合作档位与非 token 支持策略，但仍不退化为买量 rate card。
- Technical Risks:
  - 风险-1: 如果团队过早把传播激励写成播放量兑换，会被刷量和越界宣传迅速污染。
  - 风险-2: 如果评分过于主观，媒体、创作者和普通宣传参与者之间会产生预期混乱。
  - 风险-3: 如果不强制归档证据，已删除资产或平台限流会导致审核无法追溯。

## 6. Validation & Decision Record
- Test Plan & Traceability:
| PRD-ID | 对应任务 | 测试层级 | 验证方法 | 回归影响范围 |
| --- | --- | --- | --- | --- |
| PRD-README-MPI-001 | TASK-README-067 | `test_tier_required` | 覆盖对象分层、可计分/不可计分规则与 evidence 字段抽样复核 | 宣传生态激励执行性 |
| PRD-README-MPI-002 | TASK-README-067 | `test_tier_required` | 评分模型、审批链、反作弊与不按播放量买量边界抽样复核 | 绿洲币激励风控与可信度 |
| PRD-README-MPI-003 | TASK-README-067 | `test_tier_required` | safe phrase / forbidden phrase 与 FAQ 边界抽样复核 | 对外说明一致性 |
| PRD-README-MPI-004 | TASK-README-067 | `test_tier_required` | `./scripts/doc-governance-check.sh` | 文档互链与治理一致性 |
- Decision Log:
| 决策ID | 选定方案 | 备选方案（否决） | 依据 |
| --- | --- | --- | --- |
| DEC-MPI-001 | 按贡献深度、事实准确、讨论转化与生态回流审核绿洲币建议 | 按播放量、转发量或 flat buyout 直接发币 | 用户要的是“宣传方也是参与者和受益者”，不是粗放买量；必须保留治理和抗刷量能力。 |
| DEC-MPI-002 | 覆盖所有宣传参与者，但按贡献形态分层而不是按平台等级分层 | 只奖励头部媒体/KOL，普通宣传参与者排除在外 | 用户明确要求所有渠道参与者都纳入；分层应由贡献质量决定，而不是由身份头衔决定。 |
| DEC-MPI-003 | 继续复用 `eligible-small/medium/large` 奖励建议档位 | 新建一套公开可见的固定币量表 | 现阶段更适合保持 producer 审批和实际额度弹性，避免过度金融化与对外过度承诺。 |
| DEC-MPI-004 | 宣传资产必须先过 claim boundary 和 anti-fraud gate，再进入评分 | 先给流量分，再在事后补做口径检查 | 一旦把错误口径或刷量资产纳入奖励，会直接损害 `oasis7` 的公共可信度。 |
