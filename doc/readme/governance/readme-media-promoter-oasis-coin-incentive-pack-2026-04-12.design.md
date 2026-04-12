# oasis7 Media Promoter Oasis Coin Incentive Pack（2026-04-12）设计

- 对应需求文档: `doc/readme/governance/readme-media-promoter-oasis-coin-incentive-pack-2026-04-12.prd.md`
- 对应项目管理文档: `doc/readme/governance/readme-media-promoter-oasis-coin-incentive-pack-2026-04-12.project.md`

## 1. 设计定位
把“媒体推广者也是生态参与者和受益者”收成一份可执行的绿洲币激励操作包，既允许不同宣传角色进入同一套审核机制，又明确阻断按播放量买量、刷量和越界宣传。

## 2. 设计结构
- 覆盖对象层：用 `promoter_lane` 统一媒体、KOL、自媒体创作者、社区搬运号和普通宣传参与者。
- 资产 intake 层：统一记录 `asset_url`、归档证据、发布时间、奖励账户和 audience fit。
- 审核层：先过事实边界与反作弊，再进入评分。
- 评分层：基础分 + 质量/讨论/回流加分 + 作弊/重复/越界扣分。
- 发放层：继续使用 `eligible-small / eligible-medium / eligible-large / no-token-recommendation` 建议档位，并在 producer 审批后回填 `distribution_ref`。

## 3. 关键接口 / 入口
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`
- `doc/readme/governance/readme-moltbook-promotion-plan-2026-03-19.prd.md`
- `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`
- `doc/readme/governance/readme-media-promoter-oasis-coin-incentive-pack-2026-04-12.md`

## 4. 约束与边界
- 不公开固定绿洲币数额，也不公开“播放量/发帖数 -> 绿洲币”的固定换算。
- 不把宣传激励写成 `广告投放价目表` 或 `保底发币承诺`。
- 不奖励买量、互刷、抄袭搬运、截图造假或越界宣传。
- 官方正式产品名使用 `绿洲币 / Oasis Coin`；如涉及 runtime symbol/ticker，不在本专题中展开。

## 5. 设计演进计划
- 先固定覆盖对象、证据字段、评分逻辑、safe copy 和 anti-fraud gate。
- 再跑首轮真实媒体推广者审核，校正哪些资产类型更容易带来高质量讨论与生态回流。
- 最后如确有长期 creator roster，再补稳定合作专题，而不是污染当前奖励治理边界。
