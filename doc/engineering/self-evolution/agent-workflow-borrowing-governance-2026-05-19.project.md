# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 冲突 / 互借参考: `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] agent-workflow-borrowing-governance (PRD-ENGINEERING-031) [test_tier_required]: 建立专题 `prd/design/project`，首批冻结 `superpowers` 的 adopted / rejected / deferred 边界，并吸收后续 borrowing doc truth refresh 留痕，作为该主题的持续聚合 Trace。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] completion-claim-verification-helper (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 落地 repo-owned `scripts/pm/claim-ready.sh`，把“fresh verification before completion claims”收成可执行 helper，并同步接入 `prepare-task-pr` 推荐输出、PM close checklist、README 与 shell regression。 Trace: .pm/tasks/task_32a955cb401e4a269f72113db4fa0371.yaml
- [x] superpowers-skill-localization (PRD-ENGINEERING-031) [test_tier_required]: 将 `verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch` 四个已裁定可借鉴项本地化为 repo-owned skills，并同步回写 borrowing / skill inventory 文档真值。 Trace: .pm/tasks/task_6a10c37fc1fe4528a1b3cda4a43721c6.yaml
- [x] superpowers-conflict-reconciliation-doc (PRD-ENGINEERING-031) [test_tier_required]: 新增冲突/互借参考文档，明确 `rejected` / `deferred` skill 的冲突类型、可 salvage 子模式与 reopen 条件，并吸收后续 conflict doc / table truth refresh 留痕，避免后续 reopen 时重新从零梳理。 Trace: .pm/tasks/task_b7b2e89a1bec4fd0a38615773ce91af3.yaml
- [x] workflow-planning-surface-tightening (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-005) [test_tier_required]: 将 `writing-plans` 的可 salvage 部分翻译成 repo-owned planning surface，补 `project.md` 的 `File Structure / Affected Paths` 规则、handoff 原子步骤模板和 lightweight planning self-checklist，并同步回写 workflow-borrowing 专题与 engineering 根项目。 Trace: .pm/tasks/task_9bb4396c9add4868897fbf4dbfea61d9.yaml
- [x] workflow-execution-surface-tightening (PRD-ENGINEERING-031) [test_tier_required]: 将 upstream `executing-plans` 的可 salvage 部分翻译成 repo-owned execution surface，新增 `.agents/skills/executing-project-tasks`，并在 `AGENTS.md` 固化 execution gap review、逐步验证与 blocker handling 规则，同时同步回写 workflow-borrowing 专题与冲突文档真值。 Trace: .pm/tasks/task_2538f5756ad44d6ea7d1c890852389c6.yaml
- [x] default-role-subagent-orchestration (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 将 `dispatching-parallel-agents` 从 deferred 改判为 adopted（bounded），把它翻译成 repo-owned 默认 `producer_system_designer` orchestrator + role subagents 规则，并回写 `AGENTS.md`、角色卡、borrowing 专题与冲突文档。 Trace: .pm/tasks/task_9a5b1adca6a945c9ba1d48e19b77bf83.yaml
- [x] role-subagent-local-validation (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 在独立 task worktree 内并发派生 `runtime_engineer`、`viewer_engineer`、`qa_engineer` 三个只读角色 subagent，验证默认 `producer_system_designer` orchestrator + role subagents 流程至少能完成一次受控本地手工试跑，并把 residual risk 收口回 topic/root project 与 `.pm` trace。 Trace: .pm/tasks/task_141528270e2c421cb6377d6fa4eea680.yaml
- [x] subagent-driven-default-reconciliation (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 将 `subagent-driven-development` 从 rejected 改判为 adopted（bounded），把“默认子代理驱动实施”收口为同一 owner / `.pm` task / worktree / PR 真值内的分析、实现、验证与补充 review 切片，并继续拒绝 fresh subagent-per-task + local two-stage review ritual。 Trace: .pm/tasks/task_4da9c2f4ee1e431f99003056cb10522e.yaml
- [x] subagent-driven-default-workflow-rollout (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 将默认 subagent-driven execution 从原则层推进到 root workflow contract，补齐 `AGENTS.md` 的步骤化主链、handoff 模板的 `slice type / write scope / return contract / integration order` 字段，以及 planning checklist 的对应自检项。 Trace: .pm/tasks/task_a15cb9cb2832431b952c2fc9b400388d.yaml
- [x] bounded-tdd-workflow-rollout (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-007/PRD-ENGINEERING-032) [test_tier_required]: 将 `test-driven-development` 从 rejected 改判为 adopted（bounded），把 behavior-first / regression-first contract 接回 root workflow、handoff/planning surface、`.agents/skills/README.md` 与 `tdd-test-writer` skill，同时收口 skill-surface replacement 中关于 `tdd-test-writer` 的待评估边界。 Trace: .pm/tasks/task_64823ec488b648cbb95cc99ed0f4bdfc.yaml
- [x] bounded-brainstorming-workflow-rollout (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-003/PRD-ENGINEERING-AWB-008/PRD-ENGINEERING-032/PRD-ENGINEERING-032D) [test_tier_required]: 将 `brainstorming` 从 rejected 改判为 adopted（bounded），把 scope decomposition / option framing / optional visual companion 接回 root workflow、handoff/planning surface、`.agents/skills/README.md` 与本地 `bounded-brainstorming` skill，同时保留 universal gate / 逐段审批 / 强制转入 `writing-plans` 为 rejected。 Trace: .pm/tasks/task_3cad85765a3447488acba03d163126d2.yaml
- [x] repo-owned-workflow-router (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-009/PRD-ENGINEERING-032/PRD-ENGINEERING-032E) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 翻译成 repo-owned workflow router，新增 `.agents/skills/repo-owned-workflow-router`，并把默认 phase order 接回 root `AGENTS.md`、skill README 与 borrowing/conflict/skill-surface 文档真值，同时继续拒绝外部 bootstrap。 Trace: .pm/tasks/task_f305aa614deb4c959d45ffa81599cfb3.yaml

## Planned Follow-ups
- `workflow-behavior-eval-harness-followup` (`PRD-ENGINEERING-AWB-001/006`, target `test_tier_required + test_tier_full`): 为 `new-task-worktree -> workflow-report -> producer orchestrate / role subagent dispatch -> task-closeout -> prepare-task-pr -> review-thread-closeout` 建立 repo-owned agent behavior eval harness，验证默认角色 subagent 主链在真实 agent 回合中被遵守。启动时需创建独立 `.pm` task 与 worktree。
- `viewer-visual-companion-pilot-followup` (`PRD-ENGINEERING-AWB-003/PRD-WORLD_SIMULATOR-046`, target `test_tier_required`): 在 Viewer Web 下一轮结构/视觉专题中试点 browser-based visual companion，先产出 IA/wireframe/layout compare 再切实现 task，同时保持 `agent-browser` / repo-owned UI regression 仍是正式验证面。启动时需创建独立 `.pm` task 与 worktree。
- `multi-harness-workflow-packaging-deferred` (`PRD-ENGINEERING-AWB-004`, target `test_tier_required`): 在 repo-owned workflow helpers 与 evals 稳定后，再评估是否需要为 Codex/OpenCode 等 harness 做 workflow packaging；未到该阶段前保持 deferred。若重开，必须先新建专题 task。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.prd.md`
- `doc/engineering/self-evolution/memory-inspired-self-evolution-reinforcement-2026-03-31.prd.md`
- `doc/world-simulator/project.md`
- `doc/world-simulator/viewer/viewer-web-entry-visual-redesign-2026-05-12.project.md`
- `AGENTS.md`
- `.agents/roles/producer_system_designer.md`
- `.agents/skills/README.md`
- `.agents/skills/bounded-brainstorming/SKILL.md`
- `.agents/skills/executing-project-tasks/SKILL.md`
- `.agents/skills/tdd-test-writer/SKILL.md`
- `.agents/roles/*.md`
- `.agents/roles/templates/handoff-brief.md`
- `.agents/roles/templates/handoff-detailed.md`
- `.agents/roles/templates/planning-self-checklist.md`
- `testing-manual.md`

## File Structure / Affected Paths
- 改动路径:
  - `AGENTS.md`
  - `.agents/roles/producer_system_designer.md`
  - `.agents/roles/templates/{handoff-brief,handoff-detailed,planning-self-checklist}.md`
  - `.agents/skills/README.md`
  - `.agents/skills/{bounded-brainstorming,executing-project-tasks,repo-owned-workflow-router,tdd-test-writer}/SKILL.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.{prd,design,project}.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.{prd,design,project}.md`
  - `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`
  - `doc/engineering/project.md`
- 只读依赖:
  - `.agents/skills/prd/SKILL.md`
  - `.agents/skills/prd/check.md`
- 验证入口:
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- 正式回写面:
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`
  - `doc/engineering/project.md`

## 状态
- 更新日期: 2026-05-22
- 当前阶段: planned
- 当前任务: `workflow-behavior-eval-harness-followup`
- 阅读面说明: 本页只保留任务拆解、下一步与里程碑摘要；逐 slice 过程证据统一回到对应 `Trace: .pm/tasks/task_<32hex>.yaml` 与 `.execution.md`
- 阻塞项:
  - `workflow-behavior-eval-harness-followup` 仍需决定 fixture 形态，以及如何把本轮已跑通的人工多角色试跑进一步收口成可重复采样的 orchestrator/subagents 主链行为。
  - `viewer-visual-companion-pilot-followup` 必须等下一轮明确的 Viewer Web 设计任务创建后再绑定独立 `.pm` task。
- 最新完成:
  - 已完成 borrowing 基线冻结与 claim/verification 主链落地：专题三件套建立完成，`scripts/pm/claim-ready.sh` 已把“完成前 fresh verification”收口为 repo-owned helper，并接回 `prepare-task-pr`、PM close checklist 与回归测试。
  - 已完成 workflow surface 本地化：`verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch`、`executing-project-tasks`、`bounded-brainstorming`、`tdd-test-writer`、`repo-owned-workflow-router` 已落为 repo-owned skill / entrypoint，并接回 `AGENTS.md` 与 `.agents/skills/README.md`。
  - 已完成 planning / execution / authoring 真值收口：`writing-plans`、`executing-plans`、`writing-skills` 中可借的部分已经分别翻译成 planning surface、execution surface 与 skill authoring surface；对应的 future-state / deferred 漂移也已从文档中清掉。
  - 已完成多角色默认协作收口：`dispatching-parallel-agents` 与 `subagent-driven-development` 的可借部分已翻译成 `producer_system_designer` orchestrator + role subagents、bounded subagent-driven execution，以及 handoff / planning checklist 的 write-scope / return-contract 约束；本地只读试跑已证明主链可跑通，但 repo-owned eval 仍待 follow-up。
  - 已完成冲突说明与 skill-surface 对齐：`superpowers-conflict-reconciliation-2026-05-20.md`、borrowing topic 与 skill-surface topic 现在都只保留当前 adopted / deferred / rejected 真值，不再重复展开每次 truth-refresh 的过程叙述。
- 下一步:
  - 优先推进 `workflow-behavior-eval-harness-followup`，把本轮已完成的人工多角色试跑收口成可重复的 fixture、采样面与 failure signature，验证默认 `producer orchestrator + role subagents` 主链是否能被 agent 稳定执行出来。
  - Viewer 方向仅在下一轮明确结构/视觉题时，按需启动 `viewer-visual-companion-pilot-followup`。
