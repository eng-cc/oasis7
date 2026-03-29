# Gameplay 受控 Limited Preview 执行（2026-03-22）设计

- 对应需求文档: `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md`

审计轮次: 1

## 1. 设计定位
把当前 `limited playable technical preview` 口径从文案边界推进为一轮可执行、可纠偏、可回流的真实受控预览流程。

## 2. 设计结构
- Meta: owner、阶段前提、claim envelope、禁止承诺项。
- Execution Loop: controlled builder-facing thread、固定巡检窗口、信号分类、same-day 回写。
- QA Guard: unified gate watch、趋势窗口、failure signature、`go / conditional go / no-go`。
- Producer Review: 继续维持、收紧节奏或触发下一轮阶段评审。

## 3. 关键接口 / 入口
- `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
- `doc/testing/evidence/closed-beta-candidate-release-gate-2026-03-22.md`
- `doc/readme/governance/readme-closed-beta-candidate-runbook-2026-03-22.prd.md`
- `doc/readme/governance/readme-moltbook-liveops-runbook-2026-03-21.md`
- `doc/playability_test_result/templates/closed-beta-candidate-feedback-log-guide-2026-03-22.md`
- `doc/playability_test_result/templates/closed-beta-candidate-incident-templates-2026-03-22.md`

## 4. 约束与边界
- 阶段仍是 `internal_playable_alpha_late`，本专题不自动升级阶段。
- 对外只能使用 `limited playable technical preview`、`candidate access`、`builder feedback`、`GitHub issue/PR CTA` 一类表述。
- `closed beta / play now / live now / public launch / official integration announced` 仍为禁语。
- round-1 primary channel 可按执行真值切到 GitHub issue 等 builder-facing thread，但不能把“渠道切换”解释成阶段升级或公开上线。
- 先验证“受控执行是否稳定”，再讨论是否扩大节奏或触发下一轮阶段评审。

## 5. 设计演进计划
- 先建立第 1 轮受控执行文档与 handoff。
- 再落地 1 轮真实外放与 QA 守门摘要。
- 最后由 `producer_system_designer` 基于真实样本决定是否继续维持当前节奏。
