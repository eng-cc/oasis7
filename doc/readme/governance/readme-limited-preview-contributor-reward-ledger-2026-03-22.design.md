# oasis7 Limited Preview Contributor Reward Ledger（2026-03-22）设计

- 对应需求文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.project.md`

## 1. 设计定位
把 early contributor reward 从“评分模板”推进到“真实结算台账”，让 limited preview 每一轮奖励都能以同一份文档完成记录、审批、执行和归档。

## 2. 设计结构
- Round 层：固定 `round_id/candidate_id/window/status`。
- Ledger 层：逐条记录 contributor、`Reward Account`、证据、分数、档位与审核状态。
- PR intake import 层：对于 GitHub PR 来源的贡献，优先从 PR reward intake block 导入 `Reward Account`。
- 脚本导入层：由仓库脚本输出 `ready/deferred/no_reward_review_requested` 与可直接落表的 ledger row 草案。
- Approval 层：记录 producer 审批结果与审批引用，并允许在 pending distribution 前追加 actual-value review 结论。
- Distribution 层：只有 producer 完成 actual-value review 后，才回填实际发放数量、执行时间与引用。
- Archive 层：汇总 band summary、未解决项与下轮动作。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
- `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.md`
- `.github/pull_request_template.md`
- `scripts/readme-reward-pr-intake-import.py`

## 4. 约束与边界
- ledger 不定义固定 token 档位额度。
- ledger 不替代评分规则，只承接真实 round 的结算记录。
- planned grant 只是待执行候选值，在 distribution 前仍可因 actual-value review 被下调。
- raw `public key` 不进入奖励台账名称层。
- `Reward Account` 只作执行/发放字段。
- 对普通 merged PR，producer 的默认真实发放 ceiling 为 `150 OC`；超过该值必须留下 exceptional case note，不能把 `1500 OC` 当作常规待发放值。
- PR intake import 只适用于 `Source Type=PR` 的 row，不替代其它来源的证据补录。
- 脚本导入结果至少要区分 `ready / deferred / no_reward_review_requested / invalid_intake`，避免普通 PR、待补资料 PR 与错误保留的模板块混在一起。
- 任何 `distributed` 状态都必须能指回真实执行引用。
- 若 planned grant 被下调，ledger 必须同时保留最终金额和下调理由，不能只留结果不留审计说明。

## 5. 设计演进计划
- 先交付模板并纳入主追踪。
- 再填首轮真实 ledger，复核是否缺字段。
- 后续视治理成熟度补 proposal-bound / treasury-bound 引用字段。
