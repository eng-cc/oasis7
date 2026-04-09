# Gameplay 10 分钟留存修复计划（2026-04-09）设计文档

- 对应需求文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- 对应项目管理文档: `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md`

审计轮次: 1

## 设计目标
- 把“偶尔好玩”收成“10 分钟内稳定想继续玩”的产品地板。
- 用最少的新范围改动，优先解决可信度、目标承接和奖励节奏。
- 让跨角色任务按明确先后顺序推进，避免 viewer/runtime/QA 各自优化却无法叠成玩家体验。

## 设计原则
- 先可信，再扩面：最小控制地板不稳定时，不新增主路径功能。
- 先中循环，再宏系统：工业持续能力先于战争/治理大扩面。
- 先主目标，再调试语义：首屏必须服务玩家决策，而不是服务排障。
- 先正式 lane，再 debug lane：active-LLM 样本决定正式门禁，`--no-llm` 只服务定向定位。

## 角色切片
- `producer_system_designer`
  - 冻结 5 条 lane 的优先级、非目标与放行条件。
  - 审核 QA 汇总后给出 continue / hold。
- `viewer_engineer`
  - 负责首屏降噪、玩家身份表达、Mission HUD/summary/toast/chatter 的语义收口。
  - 负责正式入口主路径可见反馈。
- `runtime_engineer`
  - 负责 `play/step/select` 的地板语义。
  - 负责工业状态、阻塞、恢复、分支建议的 canonical 可解释性。
- `qa_engineer`
  - 负责 active-LLM 10 分钟样本与 `software_safe` floor 的统一 verdict。
- `agent_engineer`
  - 只在反馈语义需要把“Agent 回执”改成“玩家指挥结果”时介入，不扩展 AI 新玩法范围。

## 实施顺序
1. `TASK-GAME-061`: 冻结专题、挂载根入口、停止范围蔓延。
2. `TASK-GAME-062`: 修首连、控制 floor、software_safe 最弱入口。
3. `TASK-GAME-064`: 在地板趋稳后收首屏噪音与主目标表达。
4. `TASK-GAME-063`: 把 `PostOnboarding` 后的工业中循环包加厚。
5. `TASK-GAME-065`: 用 active-LLM 10 分钟 gate 做继续/暂停裁决。

## 验证口径
- required:
  - 文档互链、任务映射、执行日志。
  - headed Web/UI 主路径复跑。
  - industrial onboarding 卡组与 10 分钟样本。
- full:
  - 正式 active-LLM 样本趋势与弱入口复核。

## 风险
- 若 `TASK-GAME-062` 未先收口，`TASK-GAME-063/064` 的任何正向结论都可能是脆弱假象。
- 若 `TASK-GAME-065` 不区分 active-LLM 与 debug lane，门禁会再次失真。
