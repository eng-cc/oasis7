# Viewer 控制反馈迭代清单设计（2026-02-27）

- 对应需求文档: `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-control-feedback-iteration-checklist-2026-02-27.project.md`

> 2026-06-25 状态说明：本文保留为早期控制反馈迭代清单和历史验收锚点；当前正式玩家控制反馈 surface 以 `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md`、`PRD-GAME-014` 和 `software_safe` 的 `Control Proof` / `Agency Moves` 测试为准。本文不再单独代表当前 canonical viewer 方案。

## 1. 设计定位
定义基于可玩性卡片提炼出的控制反馈迭代方案：通过 Web Test API 三态增强、Mission HUD 控制反馈卡片和恢复快捷动作，把“控制生效了没有”从黑盒体验变成可见、可恢复、可量化的闭环。

## 2. 设计结构
- 问题画像层：从反馈卡片提炼 P0/P1/P2 和量化目标。
- 协议增强层：Web Test API 暴露 `received/executing/applied/blocked` 与阻断原因。
- HUD 反馈层：Mission HUD 展示控制反馈卡片和恢复动作。
- 验收层：同时使用量化指标和主观评分卡验证迭代效果。

## 3. 关键接口 / 入口
- `web_test_api` 控制反馈三态
- Mission HUD 控制反馈卡片
- play / step / seek 恢复动作
- playability A/B 结果

## 4. 约束与边界
- Viewer 侧只做可见性、反馈和恢复引导，不重写 runtime 控制算法。
- 阻断原因枚举优先覆盖高频场景，未知情况允许回落 `unknown`。
- 量化指标与主观卡片口径必须同步更新。
- 本轮优先恢复玩家感知，不追求一次性解决所有控制问题。

## 5. 设计演进计划
- 先完成问题画像和验收标准冻结。
- 再增强 Web Test API 与 HUD 反馈卡片。
- 最后通过 A/B 测试与文档回写收口控制反馈迭代。
