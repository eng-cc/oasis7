# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 冲突 / 互借参考: `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`

审计轮次: 1

> 当前 PR 的 `.pm` canonical trace 已收口为单个 aggregate task `task_de7dbd97ffdb485eb4a869cc8ac0673a`。

## 任务拆解（含 PRD-ID 映射）
- [x] borrowing-governance-baseline (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 冻结 `superpowers` adopted / rejected / deferred 矩阵，补齐 `claim-ready.sh` 完成前 fresh verification gate，本地化四个已裁定可借鉴 skill，并补一份 conflict / reopen reference。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] planning-execution-authoring-surfaces (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-005) [test_tier_required]: 将 `writing-plans`、`executing-plans`、`writing-skills` 的可 salvage 部分翻译成 repo-owned planning surface、execution surface 与 skill authoring surface，并同步回写 root workflow 与 topic docs。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] default-role-subagent-rollout (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 将默认协作收口为 `producer_system_designer` orchestrator + role subagents，并把 bounded subagent-driven execution、local validation 和 handoff `write scope / return contract / integration order` 约束接回 root workflow。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] bounded-testing-and-brainstorming (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-003/007/008) [test_tier_required]: 将 `test-driven-development` 和 `brainstorming` 分别收口为 bounded TDD 与 bounded brainstorming，只保留 behavior-first / option-framing / optional visual companion 等 repo-owned 可承接部分。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] repo-owned-workflow-router (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 翻译成本地 workflow router，并把默认 phase order 接回 `AGENTS.md`、`.agents/skills/README.md` 与相关 topic docs。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] workflow-behavior-eval-and-closeout-hardening (PRD-ENGINEERING-AWB-001/002/006) [test_tier_required]: 将 `task-closeout.sh` 收紧为 `done` closeout 前必须 fresh verify，并新增 `scripts/pm/workflow-behavior-eval.sh`，把 task-worktree bootstrap、subagent contract surface、PM closeout/claim gate、PR preflight 与 review-thread closeout 收口成 repo-owned eval。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml

## Planned Follow-ups
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
- 主要改动路径: `AGENTS.md`、`.agents/roles/templates/*.md`、`.agents/skills/{bounded-brainstorming,executing-project-tasks,repo-owned-workflow-router,tdd-test-writer,README.md}`、`doc/engineering/self-evolution/*superpowers*`、`doc/engineering/project.md`
- 只读依赖: `.agents/skills/prd/SKILL.md`、`.agents/skills/prd/check.md`
- 验证入口: `./scripts/pm/lint.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`
- 正式回写面: `agent-workflow-borrowing-governance-2026-05-19.{prd,project}.md`、`doc/engineering/project.md`

## 状态
- 更新日期: 2026-05-22
- 当前阶段: planned
- 当前任务: `viewer-visual-companion-pilot-followup`
- 关键缺口: workflow behavior eval 与 closeout fresh verification gate 已落为 repo-owned helper / eval，但 `viewer-visual-companion-pilot-followup` 仍需等待下一轮明确的 Viewer 设计题。
- 下一步: 仅在下一轮结构/视觉专题中按需启动 `viewer-visual-companion-pilot-followup`；multi-harness packaging 继续保持 deferred。
