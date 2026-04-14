# game 文档索引

审计轮次: 12

## 从这里开始
- 想先确认当前产品目标、阶段口径与完成定义：先读 `doc/game/prd.md`。
- 想看当前正在推进什么、谁在阻断、下一步做什么：先读 `doc/game/project.md`。
- 想直接按文件名定位某个 gameplay 专题：先读 `doc/game/prd.index.md`。
- 想快速理解核心玩法骨架，而不是顺扫近期长名单：先读 `doc/game/gameplay/gameplay-top-level-design.prd.md`。
- 想直接看“接下来两周只做什么”：先读 `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`。
- 想确认当前试玩放行、limited preview 与 closed beta 口径：先读 `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md` 与 `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`。
- 想跟进最近最活跃的经济/运营规则变化：先读 `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`，再按需进入对应 design / project / runbook。

## 入口
- PRD: `doc/game/prd.md`
- 设计总览: `doc/game/design.md`
- 标准执行入口: `doc/game/project.md`
- 文件级索引: `doc/game/prd.index.md`

## 入口分工
- `README.md` 只承担 landing page 职责：帮助读者先决定去模块 PRD、项目台账、文件级索引，还是少量仍承担当前阶段判断的高频专题。
- `prd.md` 是玩法目标态与阶段口径真值，适合先理解当前 game 模块在管什么、哪些边界已经冻结。
- `project.md` 是执行入口，适合确认 retention、preview、经济规则与放行门禁当前推进到哪一步。
- `prd.index.md` 是精确检索索引，适合已经知道专题名或需要完整文件清单时使用，不适合作为第一次进入模块时的首读入口。
- 高频专题文档继续承担专题真值：`gameplay-top-level-design` 管核心玩法骨架，`gameplay-ten-minute-retention-recovery-2026-04-09` 管当前冲刺窗口，`gameplay-limited-preview-execution-2026-03-22` / `gameplay-closed-beta-readiness-2026-03-21` 管试玩与放行边界，`gameplay-agent-claim-token-cost-2026-03-27` 管近期高频经济规则。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再把 `gameplay/` 下近期专题长名单直接平铺在首屏。
- 默认活跃入口保留在 `doc/game/prd.md`、`doc/game/project.md`、`doc/game/prd.index.md` 与少量仍承担当前阶段判断职责的正式专题。
- runbook、证据、checklist、handoff 与历史执行补充材料继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径按需进入。

## 模块职责
- 维护玩法目标态、核心循环与发布前可玩性口径。
- 汇总 gameplay 主题下的规则、经济、治理、战争与生产闭环专题。
- 承接体验优化、长期在线硬化与发布阻断相关设计追踪。
- 承接当前阶段判断、封闭 Beta 准入门禁与对外口径收口。
- 承接 `limited playable technical preview` 的受控执行、回流与继续/暂停决策。
- 承接 `PostOnboarding` 后 10 分钟留存修复与跨角色冲刺排序。
- 承接 agent 认领的 token 成本、claim bond、upkeep 与 reclaim 规则。
- 承接 agent claim restricted grant 的运营发放、撤销、过期与 incident runbook。

## 热点子域导航（2026-04-10 快照）
- `gameplay/` 正式专题三件套（54）：玩法骨架、留存修复、preview/beta gate、claim economy、长稳治理与发布闭环。
- `gameplay/` 补充材料（18）：runbook、evidence、checklist、handoff 与跨角色执行留痕。
- 模块根入口（5）：`README.md`、`prd.md`、`project.md`、`design.md`、`prd.index.md`。

## 高密度提示
- `doc/game/` 当前共有 77 份文件，其中 `doc/game/gameplay/` 占 72 份；默认入口不再尝试把 gameplay 长表直接摊平到模块首页。
- 需要完整活跃专题清单时，进入 `doc/game/prd.index.md`；需要 runbook、evidence、handoff 或 checklist 时，再按 `gameplay/` 中的补充文件精确进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 玩法行为、发布门禁或体验验收变化时，优先更新 `doc/game/prd.md` 与 `doc/game/project.md`；新增 gameplay 专题或默认首读入口变化时，再同步回写 `doc/game/prd.index.md` 与本页“从这里开始”。
