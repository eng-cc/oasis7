# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 冲突 / 互借参考: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`

审计轮次: 1

> Historical note: 该 2026-05 rollout 当时的本地 `.pm` trace 已收口为单个 aggregate task `task_de7dbd97ffdb485eb4a869cc8ac0673a`。当前工程 workflow 的 canonical task truth 已迁移为 GitHub-backed task issue + Project fields；本地 `.pm` trace 在本文只作为历史追溯/脚本桥接语义。本文保留 adopted / rejected / deferred 裁决历史，不再作为当前 workflow backlog 入口；当前默认入口以 `doc/engineering/workflow/source-of-truth.md`、`default-workflow-bootstrap`、`repo-owned-workflow-router` 与 `requesting-repo-owned-review` 为准。

## 任务拆解（含 PRD-ID 映射）
- [x] borrowing-governance-baseline (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 冻结 `superpowers` adopted / rejected / deferred 矩阵，补齐 `claim-ready.sh` 完成前 fresh verification gate，本地化四个已裁定可借鉴 skill，并补一份 conflict / reopen reference。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] planning-execution-authoring-surfaces (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-005) [test_tier_required]: 将 `writing-plans`、`executing-plans`、`writing-skills` 的可 salvage 部分翻译成 repo-owned planning surface、execution surface 与 skill authoring surface，并同步回写 root workflow 与 topic docs。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] default-role-subagent-rollout (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-006) [test_tier_required]: 将默认协作收口为 `producer_system_designer` orchestrator + role subagents，并把 bounded subagent-driven execution、local validation 和 handoff `write scope / return contract / integration order` 约束接回 root workflow。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] bounded-testing-and-brainstorming (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-003/007/008) [test_tier_required]: 将 `test-driven-development` 和 `brainstorming` 分别收口为 bounded TDD 与 bounded brainstorming，只保留 behavior-first / option-framing / optional visual companion 等 repo-owned 可承接部分。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] repo-owned-workflow-router (PRD-ENGINEERING-031/PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 翻译成本地 workflow router，并把默认 phase order 接回 `AGENTS.md`、`.agents/skills/README.md` 与相关 topic docs。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] workflow-behavior-eval-and-closeout-hardening (PRD-ENGINEERING-AWB-001/002/006) [test_tier_required]: 将 `task-closeout.sh` 收紧为 `done` closeout 前必须 fresh verify，并新增 `scripts/pm/workflow-behavior-eval.sh`，把 task-worktree bootstrap、subagent contract surface、PM closeout/claim gate、PR preflight 与 review-thread closeout 收口成 repo-owned eval。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] pm-step-evidence-lint-defaulting (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 将 execution surface 的逐步验证纪律继续上收到 `pm lint`，只对 `2026-05-23` 起按当前 execution-log 模板启动的 task 强制每条 entry 补齐 `Action / Validation Command / Expected Result / Actual Result / Blocker / Next Action`，并补一条缺失 `Actual Result` 的 smoke 负例。 Trace: .pm/tasks/task_4c9d1a0350034138bbc49f8b93cf321c.yaml
- [x] default-workflow-bootstrap (PRD-ENGINEERING-021/031/PRD-ENGINEERING-AWB-009A) [test_tier_required]: 新增 repo-owned `default-workflow-bootstrap` skill，把新 non-trivial task 的 trivial/non-trivial 判定、隔离 task worktree / `.pm` task 真值检查与 formal doc 入口统一前置到 router 之前，并扩展 `workflow-behavior-eval.sh` 覆盖该 surface。 Trace: .pm/tasks/task_aafdb461ebbc4d9bbe882c58e8553e67.yaml
- [x] superpowers-further-absorption-followup (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-003/005/010) [test_tier_required]: superseded / absorbed by current workflow; review-request is now owned by `requesting-repo-owned-review` and pre-PR local role review evidence, visual companion is optional evidence, and step evidence / blocker escalation are owned by `executing-project-tasks` plus the source-of-truth phase map. Legacy trace: `.pm/tasks/task_e737a3094e824f43a8c4e24e6564ea2a.yaml`. Trace: #2106 (task_5f92b597c8ae44728045398d024f0bee)

> Current convergence note (2026-07-06 / #2106): 上述旧 `.pm` trace 只保留历史追溯，不再作为当前 backlog 真值。`default-role-subagent-rollout` 的现行语义已升级为 `tpm` workflow coordinator / integrator only + bounded professional role slices；`default-workflow-bootstrap` 已升级为所有用户请求均先 bootstrap，旧 trivial/non-trivial 前置分流由 post-bootstrap friction controls 取代；review-request surface 已由 `requesting-repo-owned-review` 与 pre-PR local role review evidence packet 承接，visual companion 已收敛为可选 evidence，execution step evidence 与 blocker escalation 由 `executing-project-tasks` 和 source-of-truth phase map 承接。当前规则入口见 `doc/engineering/workflow/source-of-truth.md` 与根 `AGENTS.md`。
- [x] workflow-enforcement-audit-followup (PRD-ENGINEERING-AWB-002) [test_tier_required]: 将 fresh verification 结果写入 `.pm/tasks/*.yaml` 真值，并把 `move-task --to-status done` 收紧为必须具备 `task_complete` claim evidence，避免通过低层 `workflow-report --phase close` + `move-task` 绕过 closeout helper。 Trace: .pm/tasks/task_8b863d2d58e240398e9f2f723944ef2d.yaml

## Historical Follow-up Disposition
- `workflow-enforcement-audit-followup`、`default-workflow-bootstrap` 与 `repo-owned-review-request-followup` 已被当前 workflow source-of-truth / GitHub-backed task truth / `requesting-repo-owned-review` 吸收，不再作为本专题 open backlog。
- `viewer-visual-companion-pilot-followup` 仅保留为未来 Viewer 结构/视觉专题可选方法；不得从本文直接启动 active task truth。
- `multi-harness-workflow-packaging-deferred` 继续保持 deferred；若重开，必须新建 GitHub-backed task/worktree，并先对齐 `doc/engineering/workflow/source-of-truth.md`。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/self-evolution/file-based-self-evolution-management-2026-03-30.design.md`
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
- 主要改动路径: `AGENTS.md`、`.agents/roles/templates/*.md`、`.agents/skills/{default-workflow-bootstrap,bounded-brainstorming,executing-project-tasks,repo-owned-workflow-router,requesting-repo-owned-review,tdd-test-writer,README.md}`、`scripts/pm/{workflow-behavior-eval.sh,pm_store.py,pm_store_task_lint.py,required-tier-smoke.sh,memory-regression-smoke.sh,codex-working-memory-smoke.sh}`、`doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.{prd,design,project}.md`、`doc/engineering/project.md`、`doc/world-simulator/{project.md,viewer/viewer-web-entry-visual-redesign-2026-05-12.project.md}`
- 只读依赖: `.agents/skills/prd/SKILL.md`、`.agents/skills/prd/check.md`
- 验证入口: `./scripts/pm/lint.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`
- 正式回写面: `agent-workflow-borrowing-governance-2026-05-19.{prd,project}.md`、`doc/engineering/project.md`

## 状态
- 更新日期: 2026-07-06
- 当前阶段: completed / historical snapshot
- 当前任务: 无本专题未完成 workflow backlog；后续执行入口已迁移到 GitHub-backed task issue evidence、workflow source-of-truth 与对应 repo-owned skills。
- 关键缺口: 无本专题阻塞项；`viewer-visual-companion-pilot-followup` 仅作为未来 Viewer 结构/视觉专题可选方法，`multi-harness-workflow-packaging-deferred` 继续保持 deferred，均不得从本文直接当作 active task truth 启动。
- 下一步: 若需要重开外部 workflow 借鉴，先新建 GitHub-backed task/worktree，并以 `doc/engineering/workflow/source-of-truth.md` 为当前规则入口；本文只提供 adopted / rejected / deferred 历史裁决背景。
