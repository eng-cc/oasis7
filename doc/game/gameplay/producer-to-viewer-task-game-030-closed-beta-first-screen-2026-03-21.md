# Role Handoff Brief

审计轮次: 1

## Meta
- Handoff ID: `HANDOFF-GAME-030-2026-03-21-CLOSED-BETA-VIEWER`
- Date: `2026-03-21`
- From Role: `producer_system_designer`
- To Role: `viewer_engineer`
- Related PRD-ID: `PRD-GAME-009`
- Related Task ID: `TASK-GAME-030`
- Priority: `P0`

## Goal
- 将 `PostOnboarding` 首屏收成封闭 Beta 候选级玩家入口：主目标优先、噪音降级、核心首屏可直接放行评审。

## Why Now
- 当前 Viewer 已能连续游玩，但仍偏“工程工具 + 任务卡”，首屏噪音与 full-coverage gate 缺口阻止其被判为 Beta 级入口。

## Inputs
- 代码 / 文档入口：`doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`、`doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- 已完成内容：`#46` 已关闭，Mission HUD / PostOnboarding 已切到 canonical 玩家语义
- 已知约束：本轮先做最小产品化包，不扩写完整商业化 UI
- 依赖前置项：`doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`

## Expected Output
- 接收方交付物 1：首屏主目标优先、无无关高显著噪音的实现与证据
- 接收方交付物 2：玩家入口相关 full-coverage gate 抽样结果
- 需要回写的文档 / 日志：相关 viewer 专题、`doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`、`doc/devlog/2026-03-21.md`

## Done Definition
- [ ] `PostOnboarding` 首屏以目标 / 进度 / 阻塞 / 下一步为主
- [ ] 与主目标无关的高显著噪音被移除、降级或折叠
- [ ] headed Web/UI required-tier 证据可被 QA 直接复核

## Risks / Blockers
- 风险：若继续叠加调试信息，会稀释玩家首屏焦点
- 阻断项：若核心首屏仍出现高显著无关噪音，则不得给 Beta 体验结论
- 需要升级给谁：`producer_system_designer`、`qa_engineer`

## Validation
- 建议测试层级：`test_tier_required`
- 建议验证命令：沿用 headed Web/UI `PostOnboarding` 与 full-coverage gate 相关命令

## Notes
- 接收方确认范围：`待 viewer_engineer 确认`
- 接收方确认 ETA：`待 viewer_engineer 确认`
- 接收方新增风险：`待 viewer_engineer 回写`
