# Viewer 首局目标与控制反馈可解释化设计

- 对应需求文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.prd.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.project.md`

## 1. 设计定位
定义首局目标与控制反馈可解释化方案：把主目标结构、控制动作字典、payload 示例和结构化反馈统一到 Viewer/Web Test API，减少“目标不清”和“系统没听懂”的双重不确定性。

## 2. 设计结构
- 目标呈现层：首局 Mission HUD 固化为 `1 个主目标 + 2 个短目标`。
- 动作词典层：`describeControls()` 提供动作清单、参数 schema 与示例，降低协议探索成本。
- 反馈状态层：`sendControl()` 返回 accepted、reason、hint、parsedControl 等结构化字段。
- 状态回放层：`getState().lastControlFeedback` 负责暴露最近一次控制反馈给玩家与测试脚本。

## 3. 关键接口 / 入口
- `describeControls()`
- `fillControlExample(action)`
- `sendControl(action, payload)`
- `getState().lastControlFeedback`

## 4. 约束与边界
- 不调整 runtime 执行动作语义，只增强 Viewer 和 Web Test API 的解释层。
- 反馈字段必须稳定、可脚本断言，避免为人看优化却破坏自动化。
- 控制示例应覆盖高频动作，不在本轮扩展完整玩法脚本生成。
- 玩家视图中的解释信息要克制，不能让调试细节淹没主任务。

## 5. 设计演进计划
- 先固定首局目标结构。
- 再补动作字典、示例填充和结构化返回。
- 最后把反馈状态并入 `getState` 与测试闭环。

## 6. 2026-06-25 Follow-up: Formal Gameplay Summary
- `software_safe` 已把原 `getState().lastControlFeedback` / `player_gameplay` 状态回放层延展到 `Formal Gameplay Summary`：顶部新增 `Control Proof`，并展示 `Player Intent`、`World Consequence`、`Recovery Move`、`Next Move`。
- 同一 summary 追加 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation` 与 `Share Replay`，用于承接 P1/P2 制作人落点。
- 该 follow-up 不改变 runtime action 语义、Web Test API 控制协议或 canonical truth；它只把既有状态聚合成正式玩家可读 surface，并由 `software-safe-feedback-contract.test.mjs` 与 `main.test.jsx` 锁定。
