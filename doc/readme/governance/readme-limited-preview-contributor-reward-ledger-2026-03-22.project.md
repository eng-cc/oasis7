# oasis7 Limited Preview Contributor Reward Ledger（2026-03-22）（项目管理）

- 对应设计文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.design.md`
- 对应需求文档: `doc/readme/governance/readme-limited-preview-contributor-reward-ledger-2026-03-22.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LTRL-1 (PRD-README-LTRL-001): 完成专题 PRD / design / project 建档，明确 round ledger 的结构、状态与边界。
- [x] LTRL-2 (PRD-README-LTRL-001/002): 输出执行版 ledger 模板，覆盖 round meta、ledger rows、band summary、approval 与 distribution closure。
- [x] LTRL-3 (PRD-README-LTRL-002/003): 对齐 `readme` 模块主追踪与 `p2p token` 项目互链，并完成 devlog / 门禁收口。
- [x] LTRL-4 (PRD-README-LTRL-001/002): 将 reward intake 与台账执行层的必填字段收口为 `Reward Account`，不把 raw `public key` 写进台账名称层。
- [x] LTRL-5 (PRD-README-LTRL-001/002): 对于 GitHub PR 来源的贡献，优先从可选 PR reward intake block 导入 `Reward Account`，缺字段则保持 `deferred`。
- [x] LTRL-6 (PRD-README-LTRL-001/002): 新增 PR intake import 脚本，输出 `ready/deferred/no_reward_review_requested` 与 ledger-ready row 草案，减少手工抄写。
- [x] LTRL-7 (PRD-README-LTRL-001/002): 新增 merged PR round scan 脚本，按时间窗批量扫描已合入 PR，复用单 PR intake contract 输出窗口级状态汇总与 ledger-ready 候选。
- [x] LTRL-8 (PRD-README-LTRL-002/003): 收紧普通 merged PR 的默认审批/待发放 ceiling 到 `150 OC`，并要求任何 `>150 OC` 的 row 写明 exceptional case note。

## 依赖
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.md`
- `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- `doc/devlog/2026-03-22.md`

## 状态
- 更新日期: 2026-04-13
- 当前阶段: 首轮真实 round ledger 已批准，待执行发放
- 阻塞项: 等待 execution owner 按普通 merged PR `<=150 OC` ceiling 回填最终执行引用。
- 下一步: 按收紧后的 ordinary merged PR ceiling 更新本轮 distribution closure，再回填 `Distribution Ref / Distribution Date / Execution Owner` 并归档本轮 ledger。
