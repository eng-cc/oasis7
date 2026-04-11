# oasis7 Limited Preview Early Contributor Reward Pack（2026-03-22）设计

- 对应需求文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.project.md`

## 1. 设计定位
把 limited preview 的 early contributor reward 收成 liveops 可直接执行的操作包，避免“有奖励想法，但没有统一评分与禁语模板”的执行漂移。

## 2. 设计结构
- 贡献定义层：明确什么可以计分，什么默认不计分。
- 领取身份层：用户侧统一使用 `Oasis ID`，并把 `Reward Account` 保留为执行字段。
- PR intake 层：当贡献来源是 GitHub PR 时，允许作者通过可选 reward intake block 直接提交 `Oasis ID + Reward Account`。
- 评分层：基础分 + 质量修正 + 奖励建议档位。
- 证据层：为每条建议固定 proof 字段。
- 沟通层：为所有对外说明固定 safe phrase / forbidden phrase。

## 3. 关键接口 / 入口
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- `doc/testing/governance/token-genesis-allocation-audit-checklist-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
- `.github/pull_request_template.md`

## 4. 约束与边界
- 不把 raw `public key` 当作奖励领取名称；用户侧统一写 `Oasis ID`。
- `Reward Account` 只作执行/发放字段，不替代 claimant 的用户侧命名。
- GitHub PR intake block 只在作者主动申请 reward review 时填写，不强制所有 PR 都填奖励字段。
- 不公开固定 token 数额或固定 token/point 汇率。
- 不把奖励资格绑定到 product-level invite-only。
- 默认不奖励登录、试玩或在线时长。

## 5. 设计演进计划
- 先固定模板与禁语。
- 再用真实 round 数据校正分值。
- 最后由 producer 决定是否把档位映射到真实 token 数量。
