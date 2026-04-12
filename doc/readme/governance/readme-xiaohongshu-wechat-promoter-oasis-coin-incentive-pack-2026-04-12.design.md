# oasis7 Xiaohongshu Blogger and WeChat Official Account Oasis Coin Incentive Pack（2026-04-12）设计

- 对应需求文档: `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.project.md`

## 1. 设计定位
把“宣传方也是生态参与者和受益者”收口成一份只面向“小红书博主 + 微信公众号”的绿洲币激励操作包，避免泛化媒体对象导致执行过宽、举证过散、风控过弱。

## 2. 设计结构
- 对象层：只允许 `xiaohongshu_blogger` 与 `wechat_official_account` 两类。
- 资产 intake 层：统一记录 `asset_url`、归档证据、发布时间、奖励账户和 audience fit。
- 审核层：先过事实边界与反作弊，再进入评分。
- 评分层：基础分 + 质量/讨论/回流加分 + 作弊/重复/越界扣分。
- 发放层：继续使用 `eligible-small / eligible-medium / eligible-large / no-token-recommendation` 档位，但固定映射为 `300 / 800 / 1500 / 0 OC`，并在 producer 审批后回填 `actual_amount` 与 `distribution_ref`。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`
- `doc/readme/governance/readme-xiaohongshu-wechat-promoter-oasis-coin-incentive-pack-2026-04-12.md`

## 4. 约束与边界
- 当前专题公开固定档位金额 `300 / 800 / 1500 OC`，但不公开“阅读量/播放量 -> 绿洲币”的固定换算。
- 不把宣传激励写成 `广告投放价目表` 或 `保底发币承诺`。
- 不奖励买量、互刷、抄袭搬运、截图造假或越界宣传。
- 官方正式产品名使用 `绿洲币 / Oasis Coin`；如涉及 runtime symbol/ticker，不在本专题中展开。

## 5. 设计演进计划
- 先固定小红书博主 / 微信公众号两类对象、证据字段、评分逻辑、safe copy 和 anti-fraud gate。
- 再跑首轮真实审核，校正哪些小红书笔记和公众号文章更容易带来高质量讨论与生态回流。
- 如果后续确需扩展到其它媒体对象，再另开专题，不在当前文档中继续泛化。
