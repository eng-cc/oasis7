# oasis7：外部 Agent Workflow 借鉴治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
- 冲突 / 互借参考: `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] agent-workflow-borrowing-governance (PRD-ENGINEERING-031) [test_tier_required]: 建立专题 `prd/design/project`，首批冻结 `superpowers` 的 adopted / rejected / deferred 边界，并回写 engineering 根入口、主项目、索引与 Viewer Web 后续参考口径。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] completion-claim-verification-helper (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-002) [test_tier_required]: 落地 repo-owned `scripts/pm/claim-ready.sh`，把“fresh verification before completion claims”收成可执行 helper，并同步接入 `prepare-task-pr` 推荐输出、PM close checklist、README 与 shell regression。 Trace: .pm/tasks/task_32a955cb401e4a269f72113db4fa0371.yaml
- [x] superpowers-skill-localization (PRD-ENGINEERING-031) [test_tier_required]: 将 `verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch` 四个已裁定可借鉴项本地化为 repo-owned skills，并同步回写 borrowing / skill inventory 文档真值。 Trace: .pm/tasks/task_6a10c37fc1fe4528a1b3cda4a43721c6.yaml
- [x] superpowers-conflict-reconciliation-doc (PRD-ENGINEERING-031) [test_tier_required]: 新增冲突/互借参考文档，明确 `rejected` / `deferred` skill 的冲突类型、可 salvage 子模式与 reopen 条件，避免后续 reopen 时重新从零梳理。 Trace: .pm/tasks/task_b7b2e89a1bec4fd0a38615773ce91af3.yaml
- [x] workflow-planning-surface-tightening (PRD-ENGINEERING-031/PRD-ENGINEERING-AWB-005) [test_tier_required]: 将 `writing-plans` 的可 salvage 部分翻译成 repo-owned planning surface，补 `project.md` 的 `File Structure / Affected Paths` 规则、handoff 原子步骤模板和 lightweight planning self-checklist，并同步回写 workflow-borrowing 专题与 engineering 根项目。 Trace: .pm/tasks/task_9bb4396c9add4868897fbf4dbfea61d9.yaml
- [x] workflow-execution-surface-tightening (PRD-ENGINEERING-031) [test_tier_required]: 将 upstream `executing-plans` 的可 salvage 部分翻译成 repo-owned execution surface，新增 `.agents/skills/executing-project-tasks`，并在 `AGENTS.md` 固化 execution gap review、逐步验证与 blocker handling 规则，同时同步回写 workflow-borrowing 专题与冲突文档真值。 Trace: .pm/tasks/task_2538f5756ad44d6ea7d1c890852389c6.yaml
- [x] workflow-borrowing-doc-truth-refresh (PRD-ENGINEERING-031) [test_tier_required]: 更新当前 PR 中已过时的 borrowing 文档表述，使 planning / execution / skill-authoring 已落地事实与 remaining deferred 边界一致，并回写 topic/root project 与 `.pm` trace。 Trace: .pm/tasks/task_a8a7260e7ed74f288f29d8609e8f75e1.yaml

## Planned Follow-ups
- `workflow-behavior-eval-harness-followup` (`PRD-ENGINEERING-AWB-001`, target `test_tier_required + test_tier_full`): 为 `new-task-worktree -> workflow-report -> task-closeout -> prepare-task-pr -> review-thread-closeout` 建立 repo-owned agent behavior eval harness，验证主链规则在真实 agent 回合中被遵守。启动时需创建独立 `.pm` task 与 worktree。
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
- `.agents/skills/README.md`
- `.agents/skills/executing-project-tasks/SKILL.md`
- `.agents/roles/*.md`
- `.agents/roles/templates/handoff-brief.md`
- `.agents/roles/templates/handoff-detailed.md`
- `.agents/roles/templates/planning-self-checklist.md`
- `testing-manual.md`

## File Structure / Affected Paths
- 改动路径:
  - `AGENTS.md`
  - `.agents/skills/README.md`
  - `.agents/skills/executing-project-tasks/SKILL.md`
  - `.agents/roles/templates/handoff-brief.md`
  - `.agents/roles/templates/handoff-detailed.md`
  - `.agents/roles/templates/planning-self-checklist.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`
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
- 更新日期: 2026-05-21
- 当前阶段: planned
- 当前任务: `workflow-behavior-eval-harness-followup`
- 阻塞项:
  - `workflow-behavior-eval-harness-followup` 仍需决定 fixture 形态与“真实 agent 行为”采样面。
  - `viewer-visual-companion-pilot-followup` 必须等下一轮明确的 Viewer Web 设计任务创建后再绑定独立 `.pm` task。
- 最新完成:
  - 已建立专题三件套，冻结 `superpowers` 的 adopted / rejected / deferred 边界，并将 adopted 项与已决定吸收的 bounded borrowing 收口为 repo-owned follow-up / local surfaces。
  - 已落地 `completion-claim-verification-helper`，新增 `scripts/pm/claim-ready.sh`，并把 helper 接入 `prepare-task-pr` 推荐输出、PM close checklist、README 与 shell regression。
  - 已新增五个 repo-owned workflow skills：`verification-before-completion`、`systematic-debugging`、`receiving-code-review`、`finishing-a-development-branch`、`executing-project-tasks`，并把它们接回 borrowing / skill inventory 文档真值。
  - 已新增 `superpowers-conflict-reconciliation-2026-05-20.md`，把“为什么冲突”和“未来怎样局部互借”明确落成 explanation/reference 文档。
  - 已把 `writing-plans` 的可 salvage 部分收口成 repo-owned planning surface：`project.md` 的 `File Structure / Affected Paths`、handoff 原子步骤模板与 lightweight planning self-checklist。
  - 已把 `executing-plans` 的可 salvage 部分收口成 repo-owned execution surface：新增 `.agents/skills/executing-project-tasks`，并把 execution gap review、逐步验证与 blocker handling 接回 `AGENTS.md` 主链。
  - 已把 `writing-skills` 的 authoring surface 收口成 repo-owned 入口：`.agents/skills/README.md`、`writing-repo-owned-skills`、template 与 checklist；但 upstream 的 TDD/subagent gate 与分发部署部分仍保持 deferred。
  - 已更新当前 PR 中过时的 borrowing 文档表述，消除“未来态 roadmap / 四个 skill / 仅三条 follow-up”等与仓库现状不符的描述。
- 下一步:
  - 优先推进 `workflow-behavior-eval-harness-followup`，先验证当前 engineering 主链是否能被 agent 稳定执行出来。
  - Viewer 方向仅在下一轮明确结构/视觉题时，按需启动 `viewer-visual-companion-pilot-followup`。
