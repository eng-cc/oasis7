# Gameplay 小玩家成长线与成熟世界承接（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`

审计轮次: 1

## 任务拆解

- [x] small-player-progression-contract-freeze (PRD-GAME-015) [test_tier_required]: `producer_system_designer` 已新增“小玩家成长线与成熟世界承接”专题 PRD / design / project，并完成 `game` 根 PRD / project、`gameplay-top-level-design` 主文档、索引与当前 task execution log 挂载；正式冻结 `local operator -> regional specialist -> limited-scope regional influence` 这条 mature-world 小玩家主线，明确 `protected first industrial win` 指低爆炸半径、可恢复与 leverage 可见，而不是新手无敌保护。 Trace: .pm/tasks/task_d97dfa29208444a9b6a652f2a12fb65d.yaml

## 后续待建任务

| topic slug | owner role | status | 目标 |
| --- | --- | --- | --- |
| `runtime-small-player-lane-state-contract` | `runtime_engineer` | `planned` | 把 small-player lane 的 entry gate、checkpoint、failure signature 与 repair/rebuild/pivot recovery 下沉到 canonical truth。 |
| `viewer-economic-readability-first-capability-surface` | `viewer_engineer` | `completed` | 已在 headed Web/UI 正式玩家入口新增 `Capability Economics` surface，明确展示 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`，并用 UI/summary 定向测试回归首个持续能力与 small-player lane 的经济可读性。 |
| `viewer-small-player-lane-surface-alignment` | `viewer_engineer` | `planned` | 在 headed Web/UI 与 pure API 明确展示当前 lane、首个胜利、专业化价值、区域影响与恢复路径。 |
| `agent-small-player-specialization-contract` | `agent_engineer` | `planned` | 对齐 specialization / recovery / org-independence 行为合同，避免默认把玩家推向 major-power dependency。 |
| `qa-control-feeling-and-anti-grind-matrix` | `qa_engineer` | `completed` | 已建立跨专题矩阵，用 `player leverage` / `world_activity_only` / `grind_only` / `silent wait` blocker 联审 lane 是否真的成立，并明确 `grind_only` / `forced dependency` 当前仍为 `watch`。 |

## 任务建议标题（给后续 owner 直接开 task 用）

| topic slug | owner role | 建议标题 |
| --- | --- | --- |
| `small-player-progression-contract-freeze` | `producer_system_designer` | Freeze meaningful small-player progression lane in mature world states |
| `runtime-small-player-lane-state-contract` | `runtime_engineer` | Define canonical lane state and recovery contract for mature-world small players |
| `viewer-economic-readability-first-capability-surface` | `viewer_engineer` | Make first-capability economics and next-value legible to players |
| `viewer-small-player-lane-surface-alignment` | `viewer_engineer` | Make the small-player lane explicit in headed Web and pure API surfaces |
| `agent-small-player-specialization-contract` | `agent_engineer` | Align specialization and recovery behavior with the small-player lane contract |
| `qa-control-feeling-and-anti-grind-matrix` | `qa_engineer` | Build a mature-world anti-grind and agency blocker matrix |

## Handoff Matrix

| topic slug | 发起角色 | 接收角色 | 输入 | 期望输出 |
| --- | --- | --- | --- | --- |
| `runtime-small-player-lane-state-contract` | `producer_system_designer` | `runtime_engineer` | `PRD-GAME-015` lane / first-win / recovery / influence 边界、`PostOnboarding` 与 claim canonical truth | canonical lane state、checkpoint 与 recovery truth |
| `viewer-economic-readability-first-capability-surface` | `producer_system_designer` | `viewer_engineer` | `PRD-GAME-012/015` 的经济可读性要求、first capability 样本、现有 headed Web/UI 与 pure API surface | 玩家可见的 `投入/产出/价值/修复/下一步` surface 与文案/状态回归 |
| `viewer-small-player-lane-surface-alignment` | `producer_system_designer` | `viewer_engineer` | lane 阶段语义、player leverage rubric、现有首屏/续玩 surface | 玩家可见的 lane / first win / next-step / recovery surface |
| `agent-small-player-specialization-contract` | `producer_system_designer` | `agent_engineer` | specialization 候选、recovery 语义、org-independence 边界 | agent specialization / recovery / escalation contract 对账 |
| `qa-control-feeling-and-anti-grind-matrix` | `producer_system_designer` | `qa_engineer` | runtime/viewer/agent 对账产物、`player leverage` rubric、mature-world 样本入口 | small-player matrix、pass/block 结论与 blocker 签名 |

## 验收命令（草案）

- `small-player-progression-contract-freeze` / 文档冻结与挂载
  - `rg -n "PRD-GAME-015|small-player|new-player|regional influence|world_activity_only|player leverage" doc/game/prd.md doc/game/project.md doc/game/README.md doc/game/prd.index.md doc/game/gameplay/gameplay-top-level-design.prd.md doc/game/gameplay/gameplay-top-level-design.project.md doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.project.md .pm/tasks/task_d97dfa29208444a9b6a652f2a12fb65d.execution.md`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- `runtime-small-player-lane-state-contract` / runtime lane truth
  - `rg -n "player_gameplay|goal|progress|blocker|recovery|claim|regional|specialization" crates/oasis7/src/viewer/runtime_live crates/oasis7/src`
  - 定向 runtime / snapshot 测试与 canonical 字段对账
- `viewer-small-player-lane-surface-alignment` / 玩家 surface
  - `rg -n "claim|goal|next step|recovery|regional|specialization|player leverage" crates/oasis7_viewer crates/oasis7/src/bin/oasis7_pure_api_client.rs`
  - headed Web/UI 与 pure API 人工复核
- `viewer-economic-readability-first-capability-surface` / 经济可读性 surface
  - `rg -n "cost|input|output|value|repair|next step|first capability|regional" crates/oasis7_viewer crates/oasis7/src/bin/oasis7_pure_api_client.rs`
  - headed Web/UI 与 pure API 人工复核首个持续能力和 mature-world lane 的价值表达
- `agent-small-player-specialization-contract` / agent contract
  - `rg -n "specialization|recovery|override|reprioritize|claim|regional" doc/world-simulator/llm crates/oasis7/src/simulator`
  - `git diff --check`
- `qa-control-feeling-and-anti-grind-matrix` / QA matrix
  - `player leverage` / `world_activity_only` 抽样
  - `grind_only` / `forced major power dependency` 抽样
  - mature-world small-player 样本复核
  - 输出 pass/block 与 blocker 签名

## Done Definition

- `small-player-progression-contract-freeze`
  - [x] 新专题 PRD / design / project 已创建并回挂到 `game` 根入口、`gameplay` 主文档、索引与 task execution log
  - [x] 已冻结至少 1 条 mature-world 小玩家主线
  - [x] 已定义 `protected first industrial win`、limited-scope regional influence 与 recoverable failure
  - [x] 已拆出 runtime / viewer / agent / QA follow-up 任务
- `runtime-small-player-lane-state-contract`
  - [ ] lane entry / checkpoint / failure / recovery truth 已落成 canonical surface
  - [ ] major-power dependency 不再是默认隐性前提
- `viewer-small-player-lane-surface-alignment`
  - [ ] 玩家能读懂当前 lane、首个胜利、局部价值与恢复路径
  - [ ] headed Web/UI 与 pure API 的承接 surface 保持同等级语义
- `viewer-economic-readability-first-capability-surface`
  - [x] 玩家能直接读懂首个持续能力与 mature-world lane 的 `投入 / 产出 / 新用途 / 修复动作 / 下一步价值`
  - [x] 工业成长不再只剩库存/产量上涨，而能回答“为什么现在值得继续”
- `agent-small-player-specialization-contract`
  - [ ] specialization / recovery / org-independence contract 已对齐
  - [ ] agent 不再静默把小玩家推向依附 major power
- `qa-control-feeling-and-anti-grind-matrix`
  - [x] `player leverage != world activity`、`grind_only` 与 `forced dependency` 的 blocker 签名已固化
  - [x] mature-world 小玩家矩阵已建立并给出 pass/watch/block 结论

## 依赖

- `doc/game/prd.md`
- `doc/game/project.md`
- `doc/game/README.md`
- `doc/game/prd.index.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
- `doc/playability_test_result/prd.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-05-17
- 当前状态: in_progress
- 当前 owner: `producer_system_designer`
- 下一任务: `runtime-small-player-lane-state-contract`
- 说明:
  - 本专题当前只完成合同冻结，不等于 runtime / viewer / agent / QA 已全部落地。
  - 本专题不改写当前 `PRD-GAME-012` 的 early-retention 主优先级，也不把 `#165` 当作 stage / preview claim envelope 升级依据。
  - 高风险设计修补后，viewer / QA follow-up 优先新增的是经济可读性与 anti-grind/blocker matrix，而不是继续扩大战争/治理展示面。
