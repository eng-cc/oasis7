# 社会系统生产级方案：事实账本 + 声明式关系层（设计文档）

- 对应设计文档: `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.design.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

审计轮次: 5

## 1. Executive Summary
- 在不预设单一“正确声誉模型”的前提下，为世界提供生产级社会系统底座。
- 将“信任/声誉/关系”从硬编码规则改为可组合的事实与声明机制，保障高自由度。
- 保证跨节点可重放、可审计、可治理，并具备完整争议处理闭环。

## 2. User Experience & Functionality
### In scope
- Simulator 内核新增“社会事实账本”能力：发布、质疑、仲裁、撤销、过期。
- 新增“声明式关系边”能力：任意主体可基于事实声明关系（信任、合作、黑名单、声誉维度等）。
- 新增治理/安全护栏：证据引用校验、权益质押（可选）、角色权限校验、状态机约束。
- 新增回放与持久化闭环：事件驱动回放保持确定性。
- 新增 `test_tier_required` / `test_tier_full` 测试覆盖。

### Out of scope
- 单一固定评分公式（例如“全局唯一信誉分”）。
- 绑定经济系统的硬编码定价联动（后续由模块/策略接入）。
- 性能优化与索引压缩（本轮优先语义完整性）。

## 设计原则
- 最小语义内核：内核只记录可验证事实与状态，不强行解释含义。
- 多制度并存：不同 schema/关系维度可并行存在、互不覆盖。
- 证据优先：事实必须携带可追溯证据引用（事件或动作）。
- 争议可闭环：任何事实可被质疑，并由治理动作给出终局裁决。
- 事件溯源：所有状态变更均以事件入账，可回放还原。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
### 1) Action 扩展（simulator）
- `PublishSocialFact`
  - 字段：
    - `actor: ResourceOwner`
    - `schema_id: String`
    - `subject: ResourceOwner`
    - `object: Option<ResourceOwner>`
    - `claim: String`
    - `confidence_ppm: i64`（`1..=1_000_000`）
    - `evidence_event_ids: Vec<WorldEventId>`
    - `ttl_ticks: Option<u64>`
    - `stake: Option<SocialStake>`
- `ChallengeSocialFact`
  - 字段：
    - `challenger: ResourceOwner`
    - `fact_id: u64`
    - `reason: String`
    - `stake: Option<SocialStake>`
- `AdjudicateSocialFact`
  - 字段：
    - `adjudicator: ResourceOwner`
    - `fact_id: u64`
    - `decision: SocialAdjudicationDecision`（`Confirm` / `Retract`）
    - `notes: String`
- `RevokeSocialFact`
  - 字段：
    - `actor: ResourceOwner`
    - `fact_id: u64`
    - `reason: String`
- `DeclareSocialEdge`
  - 字段：
    - `declarer: ResourceOwner`
    - `schema_id: String`
    - `relation_kind: String`
    - `from: ResourceOwner`
    - `to: ResourceOwner`
    - `weight_bps: i64`（`-10_000..=10_000`）
    - `backing_fact_ids: Vec<u64>`
    - `ttl_ticks: Option<u64>`

### 1.1) 玩家侧社会动作后果预览
- 当玩家准备发布事实、质疑事实、仲裁事实、撤销事实或声明关系边时，玩家侧必须能读取 `social_fact_impact_quote` / `relationship_consequence_preview`，用于判断该社会动作会影响哪些信任、合作、黑名单、治理、claim 或交易协作表面。
- 最小字段：
  - `actor_id`
  - `action_kind`: `publish_fact` / `challenge_fact` / `adjudicate_fact` / `revoke_fact` / `declare_edge`
  - `schema_id`
  - `subject_id`
  - `object_id`
  - `claim_summary`
  - `confidence_ppm`
  - `stake_at_risk`
  - `ttl_ticks`
  - `affected_relationships`
  - `affected_social_surfaces`
  - `cooperation_opportunity_delta`
  - `blacklist_or_dispute_risk`
  - `governance_or_claim_relevance`
  - `recommended_social_action`
  - `why_this_action_matters`
- Edge case: 若玩家准备发布/质疑事实或声明关系边，但看不到影响对象、可见社交表面、合作机会变化、争议/stake 风险或推荐理由，标记为 `social_fact_impact_quote_missing`。
- Acceptance: 玩家提交社会事实或关系声明前，至少能看懂“这条事实/关系会影响谁、影响哪些协作/交易/治理入口、我冒了什么风险、为什么现在值得发或应该暂缓”。
- Non-goal: 本预览不新增全局唯一声誉分公式，不把社会事实硬绑定到经济定价，不重做仲裁权限、stake 结算或事实状态机，不新增复杂外交 UI、社交网络图或完整 NPC 谈判系统，也不改变 `PublishSocialFact` / `ChallengeSocialFact` / `DeclareSocialEdge` runtime ABI。

### 2) 事件扩展（simulator）
- `SocialFactPublished`
- `SocialFactChallenged`
- `SocialFactAdjudicated`
- `SocialFactRevoked`
- `SocialFactExpired`
- `SocialEdgeDeclared`
- `SocialEdgeExpired`

### 3) 世界状态扩展（simulator `WorldModel`）
- `next_social_fact_id: u64`
- `social_facts: BTreeMap<u64, SocialFactState>`
- `social_edges: BTreeMap<u64, SocialEdgeState>`
- `next_social_edge_id: u64`

### 4) 关键结构
- `SocialStake`
  - `kind: ResourceKind`
  - `amount: i64`
- `SocialFactLifecycleState`
  - `Active` / `Challenged` / `Confirmed` / `Retracted` / `Revoked` / `Expired`
- `SocialChallengeState`
  - `challenger` / `reason` / `stake` / `challenged_at`
- `SocialAdjudicationDecision`
  - `Confirm` / `Retract`
- `SocialFactState`
  - 事实核心字段 + 生命周期字段 + 可选挑战信息 + 时间戳
- `SocialEdgeState`
  - 声明核心字段 + 生命周期字段

## 规则与约束
- `schema_id` / `claim` / `relation_kind` 必须非空。
- `confidence_ppm` 必须在闭区间 `[1, 1_000_000]`。
- `evidence_event_ids` 必须非空，且所有事件 ID 均已存在。
- `backing_fact_ids` 必须全部存在，且不可引用 `Retracted/Revoked` 事实。
- 仲裁权限：仅 `ResourceOwner::World` 或事实发布者可仲裁。
- 撤销权限：仅事实发布者可撤销。
- 过期规则：带 `ttl_ticks` 的事实/关系在到期 tick 自动标记 `Expired`。
- 质押规则：若设置 `stake`，发布/质疑动作先扣减资源；
  - 仲裁 `Confirm`：质疑者质押记入系统池；
  - 仲裁 `Retract`：发布者质押记入系统池。

## 可重放与一致性
- 所有状态变更仅由事件驱动。
- replay 仅依赖事件顺序，不读取外部时间或随机数。
- 过期事件由 `step` 的确定性逻辑触发，重放时保持一致结果。

## 5. Risks & Roadmap
- M1：文档与任务拆解完成。
- M2：数据模型与动作/事件类型落地。
- M3：内核执行逻辑 + 回放逻辑落地。
- M4：测试与回归（required/full）完成。
- M5：文档与 devlog 收口。

### Technical Risks
- 语义风险：无固定评分会提升上层使用复杂度；通过示例 schema 文档缓解。
- 安全风险：证据字段若校验不足会引入垃圾事实；通过强校验与质押约束缓解。
- 治理风险：仲裁权限配置不当可能集中化；后续可引入多签/委员会策略。
- 体量风险：新增事件与状态增多，需持续关注单文件长度约束。

## 6. Validation & Decision Record
- 追溯: GitHub task issue evidence 与 Git history 保留原实施过程；本文与 design 保持现行约束语义。
