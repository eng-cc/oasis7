# Viewer 控制反馈 Step 恢复 P0 设计（2026-02-27）

- 对应需求文档: `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-control-feedback-step-recovery-p0-2026-02-27.project.md`

> 2026-06-25 状态说明：本文保留为历史 P0 step 恢复方案与失败样本锚点；当前正式玩家控制反馈 surface 以 `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md`、`PRD-GAME-014` 和 `software_safe` 的 `Control Proof` / `Agency Moves` 测试为准。本文不再单独代表当前 canonical viewer 方案。

## 1. 设计定位
定义 `step` 出现 accepted 但无推进时的 P0 恢复方案：在不改 runtime 规则的前提下，用 `seek(tick+1)` 自动补救，并把 Cause/Next 强反馈直接暴露给玩家。

## 2. 设计结构
- 卡住识别层：识别 connected 状态下 `step` 后 `tick/eventSeq` 不变化的无进展场景。
- 自动恢复层：优先触发最小步长 `seek(tick+1)`。
- 强反馈层：将 Cause/Next/Effect 写入统一反馈载体并展示给用户。
- A/B 验证层：以 Web 闭环和 playability A/B 结果确认恢复是否有效。

## 3. 关键接口 / 入口
- `WebTestApiControlFeedback`
- `stage/reason/hint/effect`
- `seek(tick+1)`
- `publish_web_test_api_state`
- Mission HUD 反馈卡片

## 4. 约束与边界
- 自动恢复必须显式告知用户已触发，不得静默掩盖问题。
- 恢复动作优先使用最小步长，避免跳跃过大。
- 本轮只治理 `step` 卡住，不扩展到普通按钮协议升级。
- 残留 seek 语义问题需单独登记，不在 P0 内混做。

## 5. 设计演进计划
- 先建立 P0 边界和反馈载体语义。
- 再实现 `step` 卡住检测与自动恢复。
- 最后通过 A/B 实测与文档回写完成收口。
