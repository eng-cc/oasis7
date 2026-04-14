# oasis7 Limited Preview Early Contributor Reward Pack（2026-03-22）（项目管理）

- 对应设计文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.design.md`
- 对应需求文档: `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] LTPR-1 (PRD-README-LTPR-001/003): 完成专题 PRD / design / project 建档，明确贡献类型、评分模板与禁语边界。
- [x] LTPR-2 (PRD-README-LTPR-001/002/003): 输出执行版操作包，覆盖基础分、质量修正、奖励建议档位与证据字段。
- [x] LTPR-3 (PRD-README-LTPR-002/003): 对齐 `readme` 模块主追踪与 `p2p token` 项目互链，并完成 devlog / 门禁收口。
- [x] LTPR-4 (PRD-README-LTPR-001/002): 将 reward intake 的必填字段收口为 `Reward Account`，不把 raw `public key` 写进奖励模板名称层。
- [x] LTPR-5 (PRD-README-LTPR-001/002): 当贡献来源是 GitHub PR 时，新增可选 reward intake block，统一收集 `Reward Account`，不在 PR 模板里索要 raw `public_key` 名称。
- [x] LTPR-6 (PRD-README-LTPR-001/002): 新增 PR intake import 脚本，支持从 PR body 解析 `Reward Account` 并输出 `ready/deferred/no_reward_review_requested`。
- [x] LTPR-7 (PRD-README-LTPR-002): 收紧普通 merged PR 的默认真实发放 ceiling 到 `150 OC`，并明确 `1500 OC` 仅适用于极少数 exceptional case，不再作为常规 MR 预期。

## 依赖
- `doc/readme/governance/readme-limited-preview-contributor-reward-pack-2026-03-22.prd.md`
- `doc/readme/project.md`
- `doc/readme/prd.index.md`
- `doc/p2p/token/mainchain-token-initial-allocation-and-early-contribution-reward-2026-03-22.project.md`
- `doc/devlog/2026-03-22.md`

## 状态
- 更新日期: 2026-04-13
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步: 后续若出现 `>150 OC` 的普通 merged PR 提案，必须先补 exceptional case note，再进入实际 round 审批；不得再把 `1500 OC` 当成常规 MR 档位。
