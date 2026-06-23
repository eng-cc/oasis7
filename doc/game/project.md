# game PRD Project

审计轮次: 18

## 入口定位
- 本文件是 game 模块当前/近期执行入口，回答“现在推进什么、谁在阻断、下一步做什么、历史 trace 去哪里找”。
- 完整专题清单看 `doc/game/prd.index.md`。
- 当前玩家侧目标态和 PRD-ID baseline 看 `doc/game/prd.md`。
- gameplay 子域首读分流看 `doc/game/gameplay/README.md`。
- 历史执行流水、已完成子任务和证据明细保留在专题 `*.project.md`、`.pm/tasks/*.execution.md`、`doc/testing/evidence/` 与 `doc/playability_test_result/`，不再在本文件平铺成长账。

## 当前执行看板
| 轨道 | 当前口径 | 下一步 / owner | Trace |
| --- | --- | --- | --- |
| 阶段判断 | 当前阶段保持 `internal_playable_alpha_late`；当前对外 claim envelope 为 `limited playable technical preview`。 | `producer_system_designer` 仅在真实 preview 回流、QA verdict 与 claim envelope 均收口后重新评估。 | `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.project.md`, `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md` |
| Limited preview 执行 | round-1 主线程已切到 GitHub issue `eng-cc/oasis7#48`；当前重点是首批 `Blocking / Opportunity / Idea` 信号回流和 claim drift 纠偏。 | `liveops_community` 执行 controlled builder-facing callout；`qa_engineer` 输出 event/weekly verdict；`producer_system_designer` 决定 continue / hold / reassess。 | `TASK-GAME-036/037/038`, `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md` |
| 10-minute trust / first capability | 2026-04-15 的 `trust gate = hold / capability gate = not_run` 只保留为 historical baseline；当前 fresh formal truth 已更新为 `trust gate = pass`、`first capability gate = pass`。 | 后续若再次回退，必须以新样本单独重开，不复用旧 blocker 文案。 | `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.project.md`, `doc/testing/evidence/issue-160-first-capability-closeout-2026-05-17.md` |
| Indirect control agency | control-feeling 合同已冻结为 accepted intent、主因果、打断/重排、续玩恢复和 fallback。 | 变更 headed Web/UI、pure API、agent reprioritize 或 runtime feedback 时，先回到专题合同和 QA matrix。 | `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.project.md`, `doc/testing/evidence/gameplay-control-feeling-and-anti-grind-matrix-2026-05-23.md` |
| Mature-world small-player lane | 小玩家主线冻结为 `local operator -> regional specialist -> limited-scope regional influence`；`protected first industrial win` 是低爆炸半径、可恢复、leverage 可见，不是无敌保护。 | 新增 progression / economy / regional influence 变更时，先证明 `player leverage` 而不是 `world_activity_only` 或纯 grind。 | `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.project.md` |
| Physical scale / action grain | `1cm` 是世界物理真值；当前正式玩家主路线仍是间接控制文明模拟，不承诺 Minecraft 式逐块直接操作。 | runtime/viewer/agent 任何尺度表达变化都要保持四层合同：厘米真值、coarse-grained 子系统、玩家动作粒度、表现层夸张。 | `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.project.md` |
| Agent claim economy | 首个 claim 也非免费；slot-1 可在 runtime 允许时使用 restricted starter funding，claim/upkeep/reclaim/audit 以专题合同为准。 | claim quote、restricted grant、upkeep、reclaim 或 liveops pool 变更进入 agent claim 专题，不在根 project 展开。 | `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.project.md`, `doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md` |

## 当前开放任务
| 任务 | PRD | Owner | 当前状态 | 下一步 | Trace |
| --- | --- | --- | --- | --- | --- |
| TASK-GAME-036 | PRD-GAME-010 | `liveops_community` | in progress / waiting for real signal | 继续归档 controlled preview 的 `Blocking / Opportunity / Idea` 信号，并在 claim drift 出现时当轮纠偏。 | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`, `doc/game/gameplay/producer-to-liveops-task-game-036-limited-preview-execution-2026-03-22.md` |
| TASK-GAME-037 | PRD-GAME-010 | `qa_engineer` | pending preview signal | 输出 `QA Weekly / Event Verdict`，确认 unified gate 是否仍为 `pass`，或给出 continue / conditional go / no-go 建议。 | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`, `doc/game/gameplay/producer-to-qa-task-game-037-limited-preview-gate-watch-2026-03-22.md` |
| TASK-GAME-038 | PRD-GAME-010 | `producer_system_designer` | pending TASK-GAME-036/037 | 基于真实执行样本决定继续维持、收紧节奏，或触发下一轮阶段评审。 | `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`, `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.project.md`, future producer sink |

## 最近收口但仍影响当前判断
| 主题 | 当前可复用结论 | Trace |
| --- | --- | --- |
| `issue-160-first-capability-closeout` | active-LLM formal sample 已把 trust gate 和 first capability gate 刷新为 pass；旧 `hold/not_run` 不再是当前 blocker。 | `doc/testing/evidence/issue-160-first-capability-closeout-2026-05-17.md`, `.pm/tasks/task_4261de9e42ac422c9ecc63525740fbb9.yaml` |
| `gameplay-high-risk-design-hardening` | bounded-response、anti-passive fallback、economic readability 与 mature-world anti-grind/anti-forced-dependency 约束已回写。 | `.pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml` |
| `viewer-economic-readability-first-capability-surface` | `software_safe` 正式玩家 surface 已显式展示投入、产出、新用途、修复动作和下一步价值。 | `.pm/tasks/task_b23cd4919b4c481490777293b556cc70.yaml` |
| `agent-claim-slot-1-auto-starter-grant` | slot-1 启动金可由专用池自动补足并原子认领，仍保持首个 claim 非免费。 | `.pm/tasks/task_313368c409c54cc2bcf8ef4f47919b65.yaml` |
| `local-standalone-submit-flow` / `chain-side-manifest-delta-runtime-readiness` | PR #547 / `a39a8d224` 后，cold-start 已形成 `claim_first_agent -> claim_starter_oc -> first agent chat` 的资源与动作链路；该进展支持 PRD-GAME-002/011 的可验证语义和 chain resource readiness，但不改变当前阶段或 claim envelope。 | `.pm/tasks/task_212396995cf3409eb34c8e9bec563ca3.yaml`, `.pm/tasks/task_a0e15f2d5d0547a3a13c93caab49b611.yaml` |
| `game-design-goal-refresh-audit` | 本轮只做根 PRD / project / gameplay README / claim 专题的冷启动链路文档刷新，明确 starter OC 与 restricted starter claim balance 的边界，不升级阶段或 claim envelope。 | Trace: .pm/tasks/task_c35014dff0ba4411a54a2a8f4fb65040.yaml |
| `game-small-player-lane-runtime-truth` | small-player lane / anti-grind runtime truth 已落到 canonical snapshot、runtime_live 派生和正向回归；这只证明 runtime/sample truth 可测，不直接升级 mature-world QA lane verdict。 | Trace: .pm/tasks/task_96b6823495f44ef39c80f3c8b1a74421.yaml |
| `viewer-immersive-blue-gradient-line` / `immersive-command-panel-spacing` | 近期 player-facing viewer polish 已收口，不改变 game 根 PRD 的玩法承诺。 | `.pm/tasks/task_834078a49c334891a3193e4f303f939a.yaml`, `.pm/tasks/task_b5440afc520648ffa963803c93da43f2.yaml` |
| `game-content-doc-trim-audit` | 根 PRD / project 已瘦身为 active gameplay baseline 与当前执行看板；专题细节、历史证据和执行流水改由专题 project、evidence 与 `.pm` trace 承接。 | Trace: .pm/tasks/task_07cf7b41bab74286b2d4573da613779d.yaml |

## 追踪与归档规则
- 本文件只保留当前开放任务、近期仍影响判断的收口项、当前阶段和下一步。
- 已完成任务的长表不在本文件继续平铺；按以下路径追踪：
  - 完整专题文件清单: `doc/game/prd.index.md`
  - 玩法骨架与历史分期: `doc/game/gameplay/gameplay-top-level-design.project.md`
  - preview / beta / retention / claim / agency / small-player / scale 的具体执行: 对应 `doc/game/gameplay/*.project.md`
  - 任务级实际执行证据: `.pm/tasks/*.execution.md`
  - QA / playability / release evidence: `doc/testing/evidence/`, `doc/playability_test_result/`
- historical baseline 可以保留，但必须标明时间和 evidence；不得把历史 blocker 复用为当前 blocker。
- 新增当前开放任务时，在本文件加一行；任务完成后，只保留“最近收口但仍影响当前判断”的短摘要或完全移到专题 project。

## 依赖
- 模块设计总览: `doc/game/design.md`
- 模块 PRD baseline: `doc/game/prd.md`
- 文件级索引: `doc/game/prd.index.md`
- gameplay 子域入口: `doc/game/gameplay/README.md`
- 核心玩法骨架: `doc/game/gameplay/gameplay-top-level-design.prd.md`
- 当前高频专题:
  - `doc/game/gameplay/gameplay-ten-minute-retention-recovery-2026-04-09.prd.md`
  - `doc/game/gameplay/gameplay-indirect-control-feeling-contract-2026-05-14.prd.md`
  - `doc/game/gameplay/gameplay-small-player-progression-lane-2026-05-17.prd.md`
  - `doc/game/gameplay/gameplay-physical-scale-indirect-control-2026-05-07.prd.md`
  - `doc/game/gameplay/gameplay-limited-preview-execution-2026-03-22.prd.md`
  - `doc/game/gameplay/gameplay-closed-beta-readiness-2026-03-21.prd.md`
  - `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.prd.md`
- Test / evidence references: `testing-manual.md`, `doc/testing/evidence/`, `doc/playability_test_result/`

## 状态
- 更新日期: 2026-06-21
- 当前状态: in_progress
- 当前阶段判断: `internal_playable_alpha_late`
- 当前 claim envelope: `limited playable technical preview`
- 当前执行重点: controlled builder-facing limited preview signal 回流、QA verdict、producer continue / hold / reassess 决策。
- 当前阻断条件:
  - 若 unified release gate 回退为 `block`，不得扩大 preview 节奏。
  - 若真实 preview signal 暴露 claim drift、agency confusion、first capability regression 或 small-player leverage 误报，必须回到对应专题 owner slice。
  - 若未完成 TASK-GAME-036/037/038，不得仅凭历史 topic pass 将阶段升级为 `closed_beta_candidate`。
- 说明: 本文件不再维护已完成任务长表；更早轮次进展以专题 project、evidence 与 `.pm` task trace 为准。

## 验证入口
- 文档治理: `./scripts/doc-governance-check.sh`
- 空白/格式检查: `git diff --check`
- 路由检查:
  - `rg -n "TASK-GAME-036|TASK-GAME-037|TASK-GAME-038|limited playable technical preview|internal_playable_alpha_late" doc/game/project.md`
  - `rg -n "PRD-GAME-012|PRD-GAME-014|PRD-GAME-015|PRD-GAME-010" doc/game/prd.md doc/game/project.md doc/game/prd.index.md doc/game/gameplay/README.md`
