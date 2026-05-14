# Gameplay 间接控制 control-feeling 合同（项目管理文档）

- 对应设计文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.design.md`
- 对应需求文档: `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`

审计轮次: 1

## 任务拆解

- [x] indirect-control-feeling-contract-freeze (PRD-GAME-014) [test_tier_required]: `producer_system_designer` 已新增“间接控制 control-feeling 合同”专题 PRD / design / project，并完成 `game` 根 PRD / project、`gameplay` 主文档、索引与当前 task execution log 挂载，明确 accepted intent、主因果、重排/打断与续玩恢复四项 guarantees 是正式 agency 地板。 Trace: .pm/tasks/task_89828a4d2c1b4e73987103699c10fa7d.yaml

## 后续待建任务

| topic slug | owner role | status | 目标 |
| --- | --- | --- | --- |
| `runtime-control-feeling-canonical-contract` | `runtime_engineer` | `planned` | 对齐 canonical accepted intent、execution status、blocker/override reason、resume anchor 与 next-step truth，避免只剩 UI 私有语义。 |
| `viewer-control-feeling-surface-alignment` | `viewer_engineer` | `planned` | 收口 headed Web/UI 首屏与续玩面，把 accepted intent、主因果、后果与下一步作为正式玩家 surface。 |
| `agent-control-feeling-reprioritize-contract` | `agent_engineer` | `planned` | 对齐 dual-mode / action contract 的 interrupt、reprioritize、override explanation 与 handoff 语义。 |
| `qa-control-feeling-matrix` | `qa_engineer` | `planned` | 建立 control-feeling matrix，给每条 guarantee 定义 pass/blocker 签名，并接入 trust/capability 样本复核。 |

## 任务建议标题（给后续 owner 直接开 task 用）

| topic slug | owner role | 建议标题 |
| --- | --- | --- |
| `runtime-control-feeling-canonical-contract` | `runtime_engineer` | Align canonical accepted-intent and causality contract for indirect control |
| `viewer-control-feeling-surface-alignment` | `viewer_engineer` | Make player-facing agency surface explicit in headed Web and software_safe |
| `agent-control-feeling-reprioritize-contract` | `agent_engineer` | Align reprioritize and override semantics with indirect-control contract |
| `qa-control-feeling-matrix` | `qa_engineer` | Build control-feeling matrix and blocker signatures for formal gameplay lanes |

## Handoff Matrix

| topic slug | 发起角色 | 接收角色 | 输入 | 期望输出 |
| --- | --- | --- | --- | --- |
| `runtime-control-feeling-canonical-contract` | `producer_system_designer` | `runtime_engineer` | `PRD-GAME-014` guarantees、现有 `player_gameplay`/goal/blocker taxonomy、trust gate blocker 签名 | canonical accepted intent / causality / resume truth 与定向验证 |
| `viewer-control-feeling-surface-alignment` | `producer_system_designer` | `viewer_engineer` | guarantees、首屏噪音治理现状、headed Web/UI 与 `software_safe` 现有 surface | agency surface 对齐稿、UI 语义回归与玩家入口说明 |
| `agent-control-feeling-reprioritize-contract` | `producer_system_designer` | `agent_engineer` | dual-mode/action contract、override 与 prompt-control 现状、future embodied 边界 | reprioritize / override / interruption 语义对账结果 |
| `qa-control-feeling-matrix` | `producer_system_designer` | `qa_engineer` | runtime/viewer/agent 对账产物、active-LLM 正式样本入口、pure API parity 现状 | control-feeling matrix、guarantee-level pass/block 结论与 blocker 归档 |

## 验收命令（草案）

- `indirect-control-feeling-contract-freeze` / 文档冻结与挂载
  - `rg -n "PRD-GAME-014|TASK-GAME-071|TASK-GAME-072|TASK-GAME-073|TASK-GAME-074|TASK-GAME-075|control-feeling|accepted intent|间接控制因果与下一步" doc/game/prd.md doc/game/project.md doc/game/prd.index.md doc/game/README.md doc/game/gameplay/gameplay-top-level-design.prd.md doc/game/gameplay/gameplay-top-level-design.project.md doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md .pm/tasks/task_89828a4d2c1b4e73987103699c10fa7d.execution.md`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- `runtime-control-feeling-canonical-contract` / canonical contract 对齐
  - `rg -n "accepted_intent|execution_status|blocker|override|next_step|resume" crates/oasis7/src/viewer/runtime_live crates/oasis7/src`
  - 定向 runtime / snapshot 测试与 contract 对账
- `viewer-control-feeling-surface-alignment` / surface 对齐
  - `rg -n "accepted intent|next step|blocked|override|resume" crates/oasis7_viewer crates/oasis7/src/bin/oasis7_pure_api_client.rs`
  - headed Web/UI 与 pure API 人工复核 agency surface
- `agent-control-feeling-reprioritize-contract` / agent reprioritize contract
  - `rg -n "override|reprioritize|interrupt|prompt_control|accepted" doc/world-simulator/llm crates/oasis7/src/simulator`
  - `git diff --check`
- `qa-control-feeling-matrix` / QA matrix
  - active-LLM trust/capability 样本复核
  - pure API parity 抽样
  - 输出 guarantee-level blocker 签名

## Done Definition

- `indirect-control-feeling-contract-freeze`
  - [x] 新专题 PRD / design / project 已创建并回挂到 `game` 根入口、主文档、索引与 task execution log
  - [x] 已冻结至少 4 条 control-feeling guarantees
  - [x] 已拆出 runtime / viewer / agent / QA follow-up 任务
- `runtime-control-feeling-canonical-contract`
  - [ ] accepted intent、主因果、blocker/override、resume/next-step truth 已在 canonical surface 上对齐
  - [ ] Viewer / API 不再需要私有拼装才能回答“我现在为什么在这里”
- `viewer-control-feeling-surface-alignment`
  - [ ] headed Web/UI 与 pure API 都能展示同等级 agency surface
  - [ ] 主意图、主因果、后果与下一步不再被噪音淹没
- `agent-control-feeling-reprioritize-contract`
  - [ ] interrupt / reprioritize / override explanation 已具备正式 contract
  - [ ] dual-mode / action contract 不再隐含“AI 自己改道但不解释”的灰区
- `qa-control-feeling-matrix`
  - [ ] QA control-feeling matrix 已建立
  - [ ] 每条 guarantee 的 blocker 签名可稳定复现并回写

## 依赖

- `doc/game/prd.md`
- `doc/game/project.md`
- `doc/game/prd.index.md`
- `doc/game/gameplay/gameplay-top-level-design.prd.md`
- `doc/game/gameplay/gameplay-micro-loop-feedback-visibility-2026-03-05.prd.md`
- `doc/game/gameplay/gameplay-post-onboarding-stage-2026-03-18.prd.md`
- `doc/game/gameplay/gameplay-pure-api-client-parity-2026-03-19.prd.md`
- `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
- `testing-manual.md`

## 状态

- 更新日期: 2026-05-14
- 当前状态: in_progress
- 当前 owner: `producer_system_designer`
- 下一任务: `runtime-control-feeling-canonical-contract`
- 说明:
  - 本专题当前只完成合同冻结与任务挂载，不等于 active-LLM formal lane 的 trust/capability gate 已恢复。
  - 本专题不改动当前“间接控制文明模拟”主路线，也不把 issue #164 解释成 direct-control 立项。
