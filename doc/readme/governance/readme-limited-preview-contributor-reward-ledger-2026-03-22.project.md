# oasis7 Limited Preview Contributor Reward Ledger（2026-03-22）（项目管理）

- 对应设计文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.design.md`
- 对应需求文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LTRL-1 (PRD-README-LTRL-001): 完成专题 PRD / design / project 建档，明确 round ledger 的结构、状态与边界。
- [x] LTRL-2 (PRD-README-LTRL-001/002): 输出执行版 ledger 模板，覆盖 round meta、ledger rows、band summary、approval 与 distribution closure。
- [x] LTRL-3 (PRD-README-LTRL-002/003): 对齐 `readme` 模块主追踪与 `p2p token` 项目互链，并完成 devlog / 门禁收口。
- [x] LTRL-4 (PRD-README-LTRL-001/002): 将 reward claimant 的用户侧身份命名统一收口为 `Oasis ID`，保留 `Reward Account` 作为执行字段，不把 raw `public key` 写进台账名称层。
- [x] LTRL-5 (PRD-README-LTRL-001/002): 对于 GitHub PR 来源的贡献，优先从可选 PR reward intake block 导入 `Oasis ID + Reward Account`，缺字段则保持 `deferred`。

## 依赖
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
- `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- `doc/devlog/2026-03-22.md`

## 状态
- 更新日期: 2026-04-12
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步: 用该模板填写首轮真实 limited preview contributor reward ledger，并补 producer 审批与 distribution ref。
