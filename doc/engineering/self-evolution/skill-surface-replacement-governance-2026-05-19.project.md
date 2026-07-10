# oasis7：Skill Surface 替换治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`

审计轮次: 1

> Current status: completed historical project summary. This file preserves the
> 2026-05/06 skill-surface rationalization outcome; current task truth and
> execution evidence live in GitHub task issue evidence comments plus
> `.pm/github-project-sync/task-archive.jsonl`. Current skill reachability is
> governed by `doc/engineering/workflow/source-of-truth.md#12-specialist-skill-reachability`.
> `.agents/skills/` is the default-loadable workflow entrypoint surface; root
> `skills/` is the non-default specialist library surface unless source-of-truth
> promotes a wrapper back into `.agents/skills/`.

## 任务拆解（含 PRD-ID 映射）
- [x] skill-inventory-rationalization (PRD-ENGINEERING-032) [test_tier_required]: 冻结当时本地 skill inventory 的 keep/replace/retire/defer 矩阵，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 surface；当前 trace 以 GitHub task issue evidence comments 与 archive record 为准。 Trace: #1473 (task_de7dbd97ffdb485eb4a869cc8ac0673a)
- [x] repo-owned-skill-surfaces (PRD-ENGINEERING-032/PRD-ENGINEERING-032C/PRD-ENGINEERING-AWB-007/008) [test_tier_required]: 将 `writing-skills`、`test-driven-development`、`brainstorming` 的可 salvage 部分翻译成 skill authoring surface、`tdd-test-writer` 与 `bounded-brainstorming` 的 repo-owned 边界；当前 trace 以 GitHub task issue evidence comments 与 archive record 为准。 Trace: #1473 (task_de7dbd97ffdb485eb4a869cc8ac0673a)
- [x] workflow-router-skill-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032E/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 收口为本地 `repo-owned-workflow-router` skill，并同步回写 skill inventory 与 root workflow phase order；当前 trace 以 GitHub task issue evidence comments 与 archive record 为准。 Trace: #1473 (task_de7dbd97ffdb485eb4a869cc8ac0673a)

## Completed Follow-ups
- [x] generic-game-skill-mirror-retirement-followup (PRD-ENGINEERING-032) [test_tier_required]: 2026-06-20 governance decision completed. `asset-optimization`、`audio-systems`、`monetization-systems` retired locally to upstream tracking; `skills/game-design-theory` and `skills/synchronization-algorithms` kept with narrowed domain-triggered entrypoints; `skills/level-design` and `skills/particle-systems` preserved as domain-triggered non-default library surfaces with unreferenced supporting files retired. Trace: #954 (task_382d3fe8d9cc4e2fa60e0425072cf644)

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `.agents/roles/*.md`
- `.agents/skills/*/SKILL.md` for default workflow entrypoints
- `skills/*/SKILL.md` for non-default specialist library material
- `.agents/skills/README.md`
- `.agents/skills/templates/SKILL.template.md`
- `.agents/skills/checklists/skill-authoring-checklist.md`

## File Structure / Affected Paths
- Historical changed paths: `.agents/skills/{README.md,templates,checklists,bounded-brainstorming,repo-owned-workflow-router,tdd-test-writer,writing-repo-owned-skills}`、`skills/{game-design-theory,level-design,particle-systems,synchronization-algorithms}`、`AGENTS.md`、`doc/engineering/self-evolution/*superpowers*`、`doc/engineering/project.md`、`doc/engineering/workflow/source-of-truth.md`
- Current read-only dependencies: `skills-lock.json`、`.agents/skills/{verification-before-completion,systematic-debugging}/SKILL.md`、`skills/README.md`
- 验证入口: `./scripts/pm/lint.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`
- Historical writeback surfaces: `skill-surface-replacement-governance-2026-05-19.{prd,design,project}.md`、`doc/engineering/project.md`、`doc/engineering/workflow/source-of-truth.md`

## 状态
- 当前阶段: completed
- 当前任务: completed; no active follow-up remains in this project file
- 关键结论: watch/defer 桶已拆分为 retired-to-upstream-tracking、maintain-trigger-narrowing、preserve-domain-triggered-non-default，不再作为长期观察状态。
- 下一步: 仅在后续具体产品/系统需求重新出现时，由对应专业角色提出新的 repo-owned skill 或恢复申请；不要直接恢复已退役 generic mirror。
