# Gameplay 间接控制 control-feeling 合同（2026-05-14）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md`

审计轮次: 1

## 设计目标
- 把“间接控制仍然像控制”收成 gameplay 正式合同，而不是零散体验判断。
- 给 runtime / viewer / agent / QA 一套共同 agency 坐标系，避免只在单侧补丁式修复。
- 明确本专题如何承接 `PRD-GAME-012` 的第 4 条 lane“间接控制因果与下一步”。

## 四项 guarantees

### 1. Intent Legibility Before/At Commit
- 玩家必须能知道当前被系统接受的主意图是什么。
- 可以是 commit 前预览，也可以是 commit 后立即确认，但不能完全隐形。
- 一旦新意图取代旧意图，必须显式标记 `replaced`、`reprioritized` 或等价结果。

### 2. Execution Acknowledgement With Causality
- 玩家不能只看到世界有变化。
- 必须看到“这个变化与我刚刚发出的意图是什么关系”。
- canonical 状态至少要区分：
  - `executing`
  - `blocked`
  - `overridden`
  - `completed_no_progress`
  - `completed_with_progress`

### 3. Interrupt / Reprioritize / Recover Hooks
- 玩家必须有方式改变当前方向，而不是只能被动等待。
- 允许不同 surface 的入口不同，但必须在同一语义层里解释“新旧意图如何交接”。
- 重连与回流时必须恢复 agency 面，而不是只恢复原始事件流。

### 4. Bounded Consequence Readability
- 玩家必须看懂 4 件事：
  - 付出了什么
  - 世界改了什么
  - 为什么当前卡住
  - 下一步最值得做什么
- 这不是“全知全能预测器”，而是“足够做下一步决策”的最小后果面。

## 设计原则
- 先 accepted intent，再 debug history：主意图必须优先于原始日志。
- 先主因果，再次级噪音：玩家第一眼看到的必须是“为什么现在这样”。
- 先可重排，再谈更宽动作面：agency 地板优先于功能扩张。
- 先 canonical truth，再多端展示：Viewer 与 pure API 不得各自拼装出不同的控制叙事。

## 与现有专题的边界
- `PRD-GAME-004`
  - 管“反馈可见性”和节奏可靠性。
  - 本专题更窄，专门管“间接控制如何仍然像控制”。
- `PRD-GAME-007`
  - 管 `PostOnboarding` 目标链。
  - 本专题要求该目标链必须被玩家感知为“我在推动”，而不是系统自走。
- `PRD-GAME-008`
  - 管 pure API 与 UI 的正式玩家等价。
  - 本专题要求 parity 不仅是字段有无，还包括 control-feeling guarantees 的等价。
- `PRD-GAME-012`
  - 管 trust gate / capability gate 的当前冲刺与 verdict。
  - 本专题是 lane-4 的正式合同，不替代 gate verdict。

## 角色切片
- `producer_system_designer`
  - 冻结 guarantees、失败签名与 scope boundary。
  - 裁决哪些 future 变更是增强还是削弱 agency。
- `runtime_engineer`
  - 对齐 canonical accepted intent、execution status、blocker/override reason、resume anchor。
- `viewer_engineer`
  - 收口 headed Web/UI 首屏与续玩 surface，确保主意图、主因果、下一步不被噪音淹没。
- `agent_engineer`
  - 对齐 dual-mode / action contract 的 interruption、reprioritization 与 override 语义。
- `qa_engineer`
  - 建立 control-feeling matrix，并把 failed guarantee 作为正式 blocker 分类。

## 实施顺序
1. `TASK-GAME-071`: 冻结专题并回挂根入口、主文档、索引、execution log。
2. `TASK-GAME-072`: 对齐 canonical accepted intent / status / blocker / next-step truth。
3. `TASK-GAME-073`: 对齐 Viewer / pure API control-feeling surface。
4. `TASK-GAME-074`: 对齐 agent/action contract 的 interrupt / reprioritize / override semantics。
5. `TASK-GAME-075`: QA 建立矩阵并接入 trust/capability 样本解释。

## 验证口径
- required:
  - 文档互链与任务映射。
  - runtime/viewer/API/agent contract 对账。
  - QA control-feeling matrix 与 blocker 签名。
- full:
  - fresh bundle active-LLM 正式样本复核。
  - pure API / headed Web agency surface 等价抽样。

## 风险
- 如果只在 Viewer 表面补提示，而 canonical truth 仍不完整，合同会再次回退成文案层错觉。
- 如果只把 issue #164 解释成“增加 direct control”，会偏离当前间接控制主路线并冲淡 retention 主 blocker。
- 如果不把 failed guarantee 写成独立 blocker，后续很容易再次出现“系统能跑但像旁观”的静默退化。
