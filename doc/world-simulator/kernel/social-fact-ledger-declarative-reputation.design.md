# 社会事实账本与声明式关系层设计

- 对应需求文档: `doc/world-simulator/kernel/social-fact-ledger-declarative-reputation.prd.md`
- 当前任务状态与历史变更：GitHub task issue evidence 与 Git history。

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
- 事实必须可追溯到证据引用；质疑与裁决受当前生命周期、身份和资源条件限制，历史 challenge 不授予当前资格。
- 多 schema/关系维度并行共存，互不覆盖。
- `social_fact_impact_quote_missing` 只表示玩家无法判断本次社会动作的影响对象、可见社交表面、合作机会变化、争议/stake 风险或推荐理由；不得被解释为要新增全局声誉分、经济定价联动、复杂外交 UI 或 NPC 谈判系统。
- 本阶段不做性能压缩和经济定价联动，优先保证语义完整性。

<a id="social-lifecycle-design"></a>
## 5. 当前状态校验与资源提交

承接 [SOCIAL-AC-1–5](social-fact-ledger-declarative-reputation.prd.md#social-lifecycle-contract)。
实现范围是 Simulator 的 social 动作、只读 quote、结算与既有 replay 边界，不扩成全 runtime 事务改造。

### 5.1 共享资格校验

quote 和 apply 复用只读资格判定；apply 不能信任预览时的状态，而应在当前动作执行边界重新检查。
Challenge 保留 Active/Confirmed 且 challenge 为空的既有资格；Adjudicate 只接受 Challenged 且有 challenge，
再核发布者身份与其他业务条件。Confirmed 的新重复裁决和 Retracted/Revoked/Expired 均拒绝。
上层对同请求返回已完成结果属于幂等查询，不重新运行本动作或新增结算。challenge 历史始终保留作审计。

### 5.2 先验证，再提交一次

Challenge 的顺序是 owner/reason/stake 格式→事实存在及 lifecycle/challenge→库存和必要写入条件→
准备同额 challenge 记录→在一个不可部分提交的动作边界内扣资源并安装记录→生成成功事件。
扣 stake 之后不能再留下普通业务拒绝分支；必要时使用暂存变更或等价回滚，不引入第二套余额真值。
Adjudicate 在取走 stake、移除事实或修改生命周期之前完成状态和结算容量检查；结算与新状态一起提交。
任何失败不得留下半次库存、池或事实变更；既有合法路径和资源上限检查保持。

### 5.3 终态与 backing

撤销和过期可以保留 challenge 对象，但后续裁决仍按当前 lifecycle 拒绝。支持 backing 的资格只取
当前 Active/Confirmed 状态；终态被拒裁决后不能通过新边重新取得有效性，也不自动恢复旧失效关系边。
本设计不通过清空历史字段、降低检查或新增“上诉/重开”通道消除反例。

<a id="social-replay-compatibility"></a>
### 5.4 新动作与历史事件重放分开

新动作资格约束施加在 quote/apply 入口；历史 journal 是已记录事件，不应简单重新送入收紧后的动作检查。
兼容读取沿用已支持记录版本的确定性事件语义，不读取墙钟、随机数或外部系统，不改写历史事件和已确认状态。
若实现确须改变 replay 约束，先明确受影响记录版本、旧记录读取/转换方案与正负 fixture，再启用新约束；
不能把一个新 guard 直接加到所有历史重放并宣称兼容。无法由旧 ActionRejected 还原的历史扣款不凭空修补；
应保留缺证和受影响范围，按相应恢复决策另行处置，本修复只证明新动作的拒绝不变性。

### 5.5 验证场景

| 验收 | fixture / 执行层 | 必须区分的结果 |
| --- | --- | --- |
| SOCIAL-AC-1 | 同一事实状态矩阵分别调用只读 quote 和公开 apply | 同一当前资格判定；quote 无副作用；quote 后撤销/过期仍由 apply 拒绝 |
| SOCIAL-AC-2 | 正 Data stake，缺失/重复/终态事实；重复提交拒绝请求 | 库存、stake pool、social facts/edges 不变；时间和拒绝事件 ID 可正常推进 |
| SOCIAL-AC-3 | Publish→Challenge→Revoke→Confirm→新建 backing；另测 Retract/Expired | 最后裁决拒绝且终态保持，新边拒绝、旧边不变，挑战历史仍存在 |
| SOCIAL-AC-4 | 合法首次挑战与正常 Confirm/Retract、撤销/过期 | 一次扣款/对应记录/一次原结算；重复业务裁决不再结算，既有上层幂等语义不变 |
| SOCIAL-AC-5 | 动作前 snapshot＋后续 journal 与 live 对照；固定旧版本 journal | 新拒绝序列资源/社会状态一致；历史兼容另验，不用只读 quote 测试代替 apply/replay |

回归复用既有 social/social_persist/social_revoke_quote fixture，并通过真实公开动作建立合法前置；
不通过直接篡改模型、伪造事件或删除断言来制造通过。上述是验证设计，不是已执行的测试或所有 runtime-live 入口证明。

### 2.1 需求承接与分配表

| 上游 requirement / professional acceptance | 具体 obligation 与适用条件 | 本设计条款 | 外部 owner / dependency | 明确排除或未覆盖范围 |
| --- | --- | --- | --- | --- |
| [生命周期与拒绝不变性](social-fact-ledger-declarative-reputation.prd.md#social-lifecycle-contract) | SOCIAL-AC-1–5：已有 Simulator 社会动作共享资格、拒绝无资源副作用、终态保持及历史兼容 | [当前状态校验与资源提交](#social-lifecycle-design) | Simulator social 维护者、玩法规则维护者及适用 QA | 不增加玩法、仲裁权限或 runtime-live 可达性承诺；不是行为已修证明 |

### 11.1 验证映射表

| 上游 requirement / professional acceptance | 本设计条款 | 独立 obligation 与适用条件 | 准确验证方法、source / scenario / layer | evidence target | 未证明范围 |
| --- | --- | --- | --- | --- | --- |
| [生命周期与拒绝不变性](social-fact-ledger-declarative-reputation.prd.md#social-lifecycle-contract) | [当前状态校验与资源提交](#social-lifecycle-design) | SOCIAL-AC-1–5 逐项判定，不以单个成功代替状态矩阵或 replay | [现有 social fixture](../../../crates/oasis7/src/simulator/tests/social.rs) 为局部接收入口；在同一实施候选的 Simulator required 回归补齐 §5.5 的合法动作、正 Data stake 拒绝、终态和 snapshot+journal 对照；历史版本兼容按 §5.4 独立覆盖 | 实施 PR 的实际候选、命令、各场景结果与未覆盖范围 | 本次仅修文档，未执行上述行为测试；不证明真实节点或所有运行模式 |

## 6. 设计演进计划
- 先固定社会事实与关系边的状态机语义。
- 再补治理动作、证据校验与回放持久化闭环。
- 最后通过测试矩阵验证跨节点重放、仲裁和过期逻辑稳定。
