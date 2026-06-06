# PostOnboarding 阶段目标链（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`

审计轮次: 1

## 任务拆解

- [x] TASK-GAMEPLAY-POD-001 (`PRD-GAME-007`) [test_tier_required]: 冻结 `PostOnboarding` 阶段 PRD / design / project，完成 `game` 根入口、索引、顶层设计主文档与 devlog 挂载。
- [x] TASK-GAMEPLAY-POD-002 (`PRD-GAME-007`) [test_tier_required]: `viewer_engineer` / `runtime_engineer` 对齐阶段机与目标选择信号契约，明确 `FirstSessionLoop` 完成、主目标来源、阻塞分类与恢复状态。
- [x] TASK-GAMEPLAY-POD-003 (`PRD-GAME-007`) [test_tier_required]: `viewer_engineer` 落地阶段切换卡、主目标卡、阶段完成卡与恢复逻辑，关闭当前 `#46` 的 UI / 产品缺口。
- [x] TASK-GAMEPLAY-POD-004 (`PRD-GAME-007`) [test_tier_required]: `qa_engineer` 新增 `#46` required-tier 验证、Web 闭环脚本 / 证据和 playability 卡片，产出通过 / 阻断结论。
- [x] TASK-GAMEPLAY-POD-005 (`PRD-GAME-007`) [test_tier_required]: `qa_engineer` 新增 `#46` 无 UI live-protocol smoke，验证同会话快照推进、`control_completion_ack` 与 `RuntimeEvent` feed，补齐 headed Web 之外的可重复协议证据。

## 依赖

- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
- `doc/playability_test_result/topics/industrial-onboarding-required-tier-cards-2026-03-15.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-03-19
- 当前状态: completed
- 当前 owner: `producer_system_designer`
- 下一任务: 无（当前专题已完成，等待新的 `PostOnboarding` 设计 / 实现需求）
- 阻断条件:
  - 若阶段目标缺少进度 / 阻塞 / 下一步三要素，则不得认定 `#46` 已关闭。
  - 若只有 UI 文案，没有同会话阶段切换与 required-tier 证据，则不得给出 go 结论。
- 说明:
  - 本 project 文档只承接 `PostOnboarding` 阶段目标链，不重写工业 / 治理 / 高风险对抗模块底层规则。
  - v1 先以工业优先的首个持续能力里程碑关闭 `#46`，再扩展中循环分支表达。
  - 2026-03-18 进展：`viewer_engineer` 已基于既有工业事件 / runtime economy event / `lastControlFeedback` 落地 `PostOnboarding` 目标卡、阻塞解释与分支解锁；QA Web 证据仍待 `TASK-GAMEPLAY-POD-004`。
  - 2026-03-19 进展：`qa_engineer` 已新增 `scripts/viewer-post-onboarding-qa.sh`，在 fresh bundle Web 会话中完成 `step(8) -> select(first_agent) -> step(24)` 闭环，并沉淀 `output/playwright/playability/post-onboarding-20260319-094056/` 证据包与 `doc/playability_test_result/card_2026_03_19_09_40_56.md`。截图人工复核确认 `4/4` 后左侧 Mission HUD 已切到 `PostOnboarding`，顶部首局总结改为进入下一阶段语义，onboarding 提示不再持续占据主视图，因此 `#46` 的 required-tier / Web 收口完成。
  - 2026-03-19 补充：`qa_engineer` 已新增 `scripts/viewer-post-onboarding-headless-smoke.sh`，并通过 live TCP 协议在无 UI 模式下完成 `request_snapshot -> step(8) -> request_snapshot -> step(24) -> request_snapshot` smoke。该证据确认 `#46` 的同会话控制推进、快照时间推进与 `RuntimeEvent` feed 在非浏览器路径上也成立，但不替代 Web/UI 卡片渲染验收。
