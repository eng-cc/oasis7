# 社会事实账本与声明式关系层设计

- 对应需求文档: `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md`
- 对应项目管理文档: `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.project.md`

## 1. 设计定位
定义面向生产级社会系统的最小语义内核：以内核事实账本记录可验证事实，以声明式关系边承载信任/合作/黑名单/声誉等多制度关系，并保持可回放、可审计、可治理。

## 2. 设计结构
- 事实账本层：支持发布、质疑、仲裁、撤销、过期等社会事实生命周期。
- 关系声明层：主体可基于事实声明关系边，不预设单一评分公式。
- 后果预览层：把事实/关系动作转换成玩家可读的信任、合作、黑名单、治理、claim 或交易协作表面影响。
- 治理护栏层：约束证据引用、可选质押、角色权限和状态机合法性。
- 事件溯源层：所有状态变更落入事件，支持跨节点重放与审计。
- 测试验证层：以 required/full 分层覆盖事实状态机、争议闭环与回放确定性。

## 3. 关键接口 / 入口
- `PublishSocialFact`
- `ChallengeSocialFact`
- `social_fact_impact_quote` / `relationship_consequence_preview`: `actor_id`、`action_kind`、`schema_id`、`subject_id`、`object_id`、`claim_summary`、`confidence_ppm`、`stake_at_risk`、`ttl_ticks`、`affected_relationships`、`affected_social_surfaces`、`cooperation_opportunity_delta`、`blacklist_or_dispute_risk`、`governance_or_claim_relevance`、`recommended_social_action`、`why_this_action_matters`
- 社会事实 ledger 与关系边状态机
- 证据引用 `WorldEventId`
- simulator 回放与持久化链路

## 4. 约束与边界
- 内核只维护可验证事实与状态，不解释为全局唯一声誉分。
- 任何事实都必须可追溯到证据引用，并可被质疑与裁决。
- 多 schema/关系维度并行共存，互不覆盖。
- `social_fact_impact_quote_missing` 只表示玩家无法判断本次社会动作的影响对象、可见社交表面、合作机会变化、争议/stake 风险或推荐理由；不得被解释为要新增全局声誉分、经济定价联动、复杂外交 UI 或 NPC 谈判系统。
- 本阶段不做性能压缩和经济定价联动，优先保证语义完整性。

## 5. 设计演进计划
- 先固定社会事实与关系边的状态机语义。
- 再补治理动作、证据校验与回放持久化闭环。
- 最后通过测试矩阵验证跨节点重放、仲裁和过期逻辑稳定。
