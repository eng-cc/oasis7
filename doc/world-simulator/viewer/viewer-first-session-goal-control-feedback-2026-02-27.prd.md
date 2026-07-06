# Viewer 首局目标与控制语义可解释反馈优化（2026-02-27）

- 对应设计文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.design.md`
- 对应项目管理文档: `doc/world-simulator/viewer/viewer-first-session-goal-control-feedback-2026-02-27.project.md`

审计轮次: 5

## 1. Executive Summary
- 提升首局可理解性：首屏/首阶段明确展示 `1 个主目标 + 2 个短目标`，降低“目标模糊”。
- 提升控制可发现性：为 `sendControl` 提供内置动作字典、payload 示例和可直接填充入口，降低“盲试协议”。
- 提升输入可解释性：每次输入返回结构化反馈（是否接受、失败原因、解析结果、建议下一步），减少“系统没听懂”的不确定感。

## 2. User Experience & Functionality

### In Scope
- `crates/oasis7_viewer` 玩家引导 HUD 文案与结构改造。
- Web Test API 新增动作描述接口与 `sendControl` 结构化返回。
- 控制输入反馈状态纳入 `getState` 输出，便于自动化与人工复盘。
- 对应 viewer 单元测试补充（test_tier_required）。

### Out of Scope
- 不改 runtime 业务规则与 action 执行语义。
- 不改 LLM 策略生成逻辑。
- 不做新的玩法系统，仅优化首局引导与控制语义可解释性。

## 3. AI System Requirements (If Applicable)
- N/A: 本专题不新增 AI 专属要求。

## 4. Technical Specifications
- 玩家目标文案：
  - 现有 `PlayerGuideStep` 扩展为“主目标 + 两个短目标”渲染结构。
- Web Test API：
  - 新增 `describeControls(): object`，返回动作清单、payload schema、示例。
  - `sendControl(action, payload): object` 返回结构化结果：
    - `accepted: bool`
    - `action: string`
    - `parsedControl: string | null`
    - `reason: string | null`
    - `hint: string | null`
- `getState()` 扩展：
  - 新增 `lastControlFeedback`，用于显示最近控制输入反馈摘要。

## 5. Risks & Roadmap
- M1：完成文档建档与任务拆解。
- M2：完成首局主/短目标 UI 改造与测试。
- M3：完成控制语义可发现 + 输入可解释反馈改造与测试。
- M4：完成验证、项目文档收口与日志沉淀。

### Technical Risks
- Web Test API 在 wasm 场景下返回对象结构新增字段，可能影响旧自动化脚本断言。
- 引导文案改造若与现有玩家模式布局冲突，可能造成 UI 拥挤。
- 过多调试信息可能干扰普通玩家视图，需要在呈现层保持克制。

## 完成态（2026-02-27）
- 首局目标已升级为“主目标 + 两个短目标”结构，并接入玩家 Mission HUD。
- Web Test API 已新增：
  - `describeControls()`
  - `fillControlExample(action)`
  - 结构化 `sendControl(action, payload)` 返回
  - `getState().lastControlFeedback`
- 输入解释链路已覆盖“入参解析、失败原因、建议下一步、世界状态增量（logicalTime/eventSeq）”。

## 2026-06-25 Follow-up: P0 Control Proof Surface
- `viewer` 正式玩家入口在 `Formal Gameplay Summary` 顶部新增 `Control Proof` 卡片，把现有 runtime/viewer 真值聚合为四格：`Player Intent`、`World Consequence`、`Recovery Move`、`Next Move`；`software_safe` 仅作为 compat alias 复核。
- 该卡片只消费既有 `player_gameplay` / feedback 字段，不新增 runtime schema，不把 viewer 派生摘要写成 canonical truth。
- 交互意图：玩家不需要展开 diagnostics 或拼 raw log，也能在同一处回答“我让系统做什么、世界为什么这样、卡住时怎么恢复、下一步做什么”。
- 验收：`software-safe-feedback-contract.test.mjs` 锁定 `controlProof` 派生结构，`main.test.jsx` 锁定 DOM 中 `Control Proof / Player Intent / World Consequence / Recovery Move / Next Move` 可见。

## 2026-06-25 Follow-up: P1/P2 Continuation Surface
- `viewer` 正式玩家入口在 `Formal Gameplay Summary` 中追加 `Agency Moves`、`First Win & Anti-Grind`、`Mature-World Continuation` 与 `Share Replay`，用于把 AI 时代开篇体验的后续落点从“系统替我做事”转成“我能打断、重排、纠偏，并看到世界承接结果”；`software_safe` 仅作为 compat alias 复核。
- 该 surface 继续只读现有 `player_gameplay` 字段，不新增 runtime canonical truth；具体 small-player lane、anti-grind 与 repair/rebuild/pivot 设计真值挂回 `PRD-GAME-015`。
- 验收：`software-safe-feedback-contract.test.mjs` 锁定 P1/P2 派生结构，`main.test.jsx` 锁定 `Agency Moves / First Win & Anti-Grind / Mature-World Continuation / Share Replay` 可见。

## 6. Validation & Decision Record
- 追溯: 对应同名 `.project.md`，保持原文约束语义不变。
