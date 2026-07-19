# 发行沟通与公开口径

## 文档身份

- 所属产品模块：玩家入口与发行
- 上位产品 PRD：[`prd.md`](prd.md)
- 生命周期：`active`
- Owner role：`producer_system_designer`
- 公开状态权威：[`README.md`](../../../README.md)
- 专业与执行入口：[`doc/readme/prd.md`](../../readme/prd.md)、[`doc/testing/prd.md`](../../testing/prd.md)

本文是长期产品分册，定义内部候选证据如何形成可审查、可发布、可纠正和可撤回的公开说明。它不保存具体候选 verdict、公告正文、渠道排期、字段模板、任务状态或历史发布记录。

## 1. 产品目标

玩家和社区应能明确区分内部候选状态、待审核文案和已经发布的公开事实。任何渠道说明都必须忠实反映同一候选、primary mode 与当前证据范围，并在事实变化时及时纠正，而不能把内部 `go`、QA green 或已批准草稿包装成已经发布或更高阶段。

## 2. 证据与公开口径边界

- 每份发行沟通必须绑定候选或版本、primary mode、适用入口、证据范围与更新时间；不同版本、模式或历史样本不得互相代签。
- 内部 readiness、go/no-go、QA 结论和产品决策只是公开沟通的输入，不会自动生成公开 claim。
- 对外内容按“已证实事实 → 适用范围与已知限制 → 未承诺内容与残余风险 → 下一步或恢复动作”组织；亮点不能覆盖限制。
- 根 `README.md` 是当前公开状态与 claim envelope 的唯一权威。站点、公告、changelog、帖子和回复不得独立升级阶段或扩大承诺。

## 3. 沟通生命周期

- `draft`、`reviewed`、`publish_ready`、`published`、`withdrawn/stale` 是不同状态，不能互相代签。草稿和审核通过都不是发布证据。
- 正式发布必须记录 channel、发布时间、URL 或 message ID 与发布 owner；缺少这些信息时只能声称文案已准备，不能声称已发布。
- 内部状态、证据或风险变化时，受影响旧文案立即失效；所有仍活跃的公开 surface 必须纠正、降级或撤回，并保留变更依据。
- claim rollback 只撤回或修正公开表述，不替代软件回滚、QA blocker、事故恢复或 operator 决策。

## 4. 角色与审核

- `liveops_community` 起草内容并执行渠道同步，确保语气、反馈入口和 surface 一致。
- `producer_system_designer` 审核产品承诺、适用范围、未承诺项与阶段边界。
- `qa_engineer` 复核证据是否支撑对应表述并给出 blocker；QA 不通过模板自行宣布产品发布。
- 公开口径变化必须能追踪上述输入、审核与发布记录。任何角色都不能绕过根 README 或当前证据单独扩大 claim。

## 5. 事故、回退与最小披露

- 事故或回退说明只陈述已确认影响、受影响版本/入口、当前恢复动作和下一次可信更新时间；未获权威确认时使用“正在调查”，不得承诺恢复时间或宣称风险清零。
- rollback communication 必须来自权威 rollback/incident 决策；LiveOps 不独立决定技术回滚、发布时间或恢复承诺。
- 公开内容只披露理解影响和采取行动所需的最小信息，不暴露内部运行目录、命令、敏感配置、私密 issue/comment、账号标识、未脱敏事故报告或不必要的个人复现数据。
- 内部 evidence link 与可公开 source link 必须区分；内部审核路径不得原样外发。

## 6. 组合验收

- RC-1：每条发行沟通都绑定同一候选或版本、primary mode、适用入口、当前证据范围与更新时间。
- RC-2：`draft`、`reviewed` 或 `publish_ready` 不会被当前公开入口引用为已发布事实；发布证据包含 channel、时间、URL/message ID 与 owner。
- RC-3：公开说明明确区分 confirmed facts、scope/limitations、non-promises/known risks 与 next/recovery。
- RC-4：缺证、状态漂移或 blocker 会使旧文案失效，并在所有 active surface 触发纠正、降级或撤回。
- RC-5：根 README、站点、公告、changelog 与渠道说明不存在阶段或 claim 分叉。
- RC-6：沟通可追踪 LiveOps 起草、producer 承诺审核、QA 证据复核和实际发布记录。
- RC-7：事故或 rollback 不沿用旧 go 结论，不把 messaging rollback 冒充软件恢复。
- RC-8：公开内容遵循最小披露，不泄露敏感配置、内部证据或个人信息。

### 6.1 验收追踪

| 成功标准 | 专业 owner | 权威文档 | 验证证据 | 测试层级 |
| --- | --- | --- | --- | --- |
| RC-1 / RC-3 | liveops_community / producer_system_designer / qa_engineer | `doc/readme/prd.md`; `doc/testing/prd.md` | 候选、模式、证据和消息结构抽样审计 | test_tier_required |
| RC-2 / RC-6 | liveops_community / producer_system_designer | `README.md`; `doc/readme/prd.md` | 审核记录及 channel/time/URL/message ID/owner 发布 trace | test_tier_required |
| RC-4 / RC-5 | producer_system_designer / liveops_community / qa_engineer | `README.md`; `doc/readme/prd.md`; `doc/testing/prd.md` | active surface claim 对账与 stale/withdrawn 纠正记录 | test_tier_required |
| RC-7 / RC-8 | liveops_community / qa_engineer / runtime_engineer / viewer_engineer / blockchain_ops_engineer | `doc/readme/prd.md`; `doc/testing/prd.md` | incident/rollback 抽样与最小披露审计 | test_tier_required |

## 7. Non-Goals

- 不固化任何候选 ID、历史 go/no-go、当前阶段、公告正文、FAQ 内容或发布日期。
- 不定义具体模板字段、渠道排期、运营 runbook、事故流程或技术回滚步骤。
- 不替代根 README、正式 changelog、GitHub task evidence、QA gate 或 LiveOps 执行记录。
- 不把内部审批、草稿完成或互动量当作 publication evidence。
