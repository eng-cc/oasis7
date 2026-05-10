# Gameplay 封闭 Beta 准入门禁（2026-03-21）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`

审计轮次: 1

## 目标
- 把“当前是什么阶段、下一阶段靠什么证据升级”从经验判断变成正式执行结构。
- 将 runtime、viewer、QA、liveops 的收口工作拆成并行但可汇总的四条线。

## 设计原则
- 单一事实源：阶段判断由 `PRD-GAME-009` 和 `doc/game/project.md` 统一维护。
- 先统一 gate，再放宽口径：只有四条线都进入同一 release gate，制作人才允许升级阶段。
- 失败可回退：任何关键 lane 失败都回退到 `internal_playable_alpha_late`，而不是“部分放行”。

## 工作流设计

### 1. 阶段冻结
- `producer_system_designer` 先冻结当前阶段为 `internal_playable_alpha_late`。
- 同步回写 `doc/game/prd.md`、`doc/game/project.md`、`doc/game/gameplay/gameplay-top-level-design.project.md`、索引与 devlog。

### 2. 四条收口线
- `runtime_engineer`
  - 收口 five-node no-LLM soak、replay/rollback drill、长期在线 release gate。
- `viewer_engineer`
  - 收口 `PostOnboarding` 首屏降噪、主目标优先级和玩家入口 full-coverage gate。
- `qa_engineer`
  - 将 headed Web/UI、pure API、no-UI smoke、longrun/recovery 统一到一个候选版本 gate。
- `liveops_community`
  - 收口封闭 Beta 候选 runbook、招募/反馈/事故回流模板与禁语清单。

### 3. 汇总与拍板
- `qa_engineer` 输出统一 gate 结论。
- `producer_system_designer` 基于 gate、趋势和 liveops 口径决定：
  - 继续 `internal_playable_alpha_late`
  - 升级为 `closed_beta_candidate`

## 关键依赖
- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/game/gameplay/gameplay-longrun-p0-production-hardening-2026-03-06.prd.md`
- `doc/testing/evidence/testing-quality-trend-baseline-2026-03-11.md`
- `doc/world-simulator/viewer/viewer-web-software-safe-mode-2026-03-16.prd.md`
- `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`

## 风险控制
- 若趋势样本不足，则只允许保留当前阶段，不允许按主观感觉升阶。
- 若 fresh bundle 与 source tree 结果不一致，以 fresh bundle 候选版本为准。
- 若 liveops 未完成禁语清单和反馈回路，即便技术 gate 通过，也只能维持 `technical preview`。
