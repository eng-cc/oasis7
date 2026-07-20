# game 文档索引

产品层归属：`game` 是“世界规则与核心玩法”的专业域权威；区域设施专题同时向“大世界基础设施”产品 PRD 汇报组合关系，但不形成并列产品入口。产品总导航见 `doc/product/README.md`。

审计轮次: 12

## 从这里开始
- 想先确认当前产品目标、阶段口径与完成定义：先读 `doc/game/prd.md`。
- 想看当前正在推进什么、谁在阻断、下一步做什么：先读 `doc/game/project.md`。
- 想直接按文件名定位某个 gameplay 专题：先读 `doc/game/prd.index.md`。
- 想先进入 gameplay 热点子域，而不是顺扫近期长名单：先读 `doc/game/gameplay/README.md`。
- 想快速理解核心玩法骨架：先读 `doc/game/gameplay/gameplay-top-level-design.prd.md`。
- 想直接看“接下来两周只做什么”：先读 `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`。
- 想确认“间接控制为什么仍然应该感觉像我在控制，而不是旁观 AI”：先读 `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`。
- 想确认“成熟世界里小玩家/新玩家靠什么继续有独立价值，而不是只能投靠大组织”：先读 `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md`，再按需下钻 gameplay 顶层合同。
- 想确认当前试玩放行与发行口径：先读 `doc/product/player-entry-distribution/access-modes-and-release-readiness.prd.md`；当前执行状态看 `doc/game/project.md` 与 round execution record，公开状态只看根 `README.md`。
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
- `gameplay/README.md` 是 `gameplay/` 热点子域 landing page，负责把玩法骨架、留存、agency、preview/beta gate 与 economy/claim 按问题分流。
- 高频专题文档继续承担专业域真值：`gameplay-top-level-design` 管核心玩法骨架，`gameplay-ten-minute-retention-recovery-2026-04-09` 管当前冲刺窗口，`gameplay-indirect-control-feeling-contract-2026-05-14` 管间接控制下的 agency 合同，`gameplay-agent-claim-token-cost-2026-03-27` 管近期高频经济规则；访问模式与受控 preview / 放行组合承诺统一由产品专题承载，当前执行由 game project 与对应证据承载。
- `doc/product/world-rules-core-gameplay/mature-world-progression.prd.md` 管 mature-world 小玩家的产品承诺；`gameplay-top-level-design.prd.md` 管对应专业玩法合同。

## 活跃阅读面边界
- 当前页只保留 `what / where / next / risk` 所需入口，不再把 `gameplay/` 下近期专题长名单直接平铺在首屏。
- 默认活跃入口保留在 `doc/game/prd.md`、`doc/game/project.md`、`doc/game/prd.index.md` 与少量仍承担当前阶段判断职责的正式专题。
- runbook、证据、checklist、handoff 与历史执行补充材料继续保留可检索性，但默认从 `prd.index.md` 或具体专题路径按需进入。

## 模块职责
- 维护玩法目标态、核心循环与发布前可玩性口径。
- 汇总 gameplay 主题下的规则、经济、治理、协作与生产闭环专题。
- 承接体验优化、长期在线硬化与发布阻断相关设计追踪。
- 承接当前阶段判断、封闭 Beta 准入门禁与对外口径收口。
- 承接 `limited playable technical preview` 的受控执行、回流与继续/暂停决策。
- 承接 `PostOnboarding` 后 10 分钟留存修复与跨角色冲刺排序。
- 承接 agent 认领的 token 成本、claim bond、upkeep 与 reclaim 规则。
- 承接 agent claim restricted grant 的运营发放、撤销、过期与 incident runbook。

## 热点子域导航
- `gameplay/`：先看 `gameplay/README.md`，再按簇进入玩法骨架、留存修复、preview/beta gate、claim economy、长稳治理、agency 合同、mature-world 小玩家承接与可编程区域设施。
- `gameplay/` 正式专题三件套：用于已知专题后的精确下钻。
- `gameplay/` 补充材料：runbook、evidence、checklist 与跨角色执行留痕。
- 模块根入口：`README.md`、`prd.md`、`project.md`、`design.md`、`prd.index.md`。

## 高密度提示
- 本页不维护容易漂移的文件数量快照；当前模块库存与热点子目录统一以 `./scripts/doc-inventory-report.sh` 为准。默认入口只负责把你送到 `gameplay/README.md`，不再尝试把 gameplay 长表直接摊平到模块首页。
- 需要完整活跃专题清单时，进入 `doc/game/prd.index.md`；需要 runbook、evidence、checklist 或跨角色执行留痕时，再按 `gameplay/` 中的补充文件精确进入。

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-governance/doc-structure-standard.design.md` 为准。
- 玩法行为、发布门禁或体验验收变化时，优先更新 `doc/game/prd.md` 与 `doc/game/project.md`；新增 gameplay 专题或默认首读入口变化时，再同步回写 `doc/game/gameplay/README.md`、`doc/game/prd.index.md` 与本页“从这里开始”。
