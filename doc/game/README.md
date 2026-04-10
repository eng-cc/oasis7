# game 文档索引

审计轮次: 11

## 入口
- PRD: `doc/game/prd.md`
- 设计总览: `doc/game/design.md`
- 标准执行入口: `doc/game/project.md`
- 文件级索引: `doc/game/prd.index.md`

## 从这里开始
- 想先确认当前产品目标、阶段口径与完成定义：先读 `doc/game/prd.md`。
- 想看当前正在推进什么、谁在阻断、下一步做什么：先读 `doc/game/project.md`。
- 想快速理解核心玩法骨架，而不是逐篇翻近期专题：先读 `doc/game/gameplay/gameplay-top-level-design.prd.md`。
- 想确认当前对外试玩口径、limited preview 执行边界与放行条件：先读 `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md` 与 `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`。
- 想直接看“接下来两周只做什么”，而不是继续翻全量专题：先读 `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`。
- 想跟进最近最活跃的经济/运营规则变化：先读 `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`，再按需进入对应 design / project / runbook。

## 模块职责
- 维护玩法目标态、核心循环与发布前可玩性口径。
- 汇总 gameplay 主题下的规则、经济、治理、战争与生产闭环专题。
- 承接体验优化、长期在线硬化与发布阻断相关设计追踪。
- 承接当前阶段判断、封闭 Beta 准入门禁与对外口径收口。
- 承接 `limited playable technical preview` 的受控执行、回流与继续/暂停决策。
- 承接 `PostOnboarding` 后 10 分钟留存修复与跨角色冲刺排序。
- 承接 agent 认领的 token 成本、claim bond、upkeep 与 reclaim 规则。
- 承接 agent claim restricted grant 的运营发放、撤销、过期与 incident runbook。

## 主题文档
- `gameplay/`：玩法、经济、治理、战争、长稳与发布闭环专题。

## 高频专题
- `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
- `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
- `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
- `doc/game/gameplay/gameplay-distributed-consensus-governance-longrun-2026-03-06.prd.md`
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- `doc/game/gameplay/gameplay-release-production-closure.prd.md`

## 共享约定
- 模块根入口、专题落位与 README/legacy redirect 的共享规则统一以 `doc/engineering/doc-structure-standard.design.md` 为准。
- 玩法行为、发布门禁或体验验收变化时，优先更新 `doc/game/prd.md` 与 `doc/game/project.md`；新增 gameplay 专题时，再同步回写 `doc/game/prd.index.md`。
