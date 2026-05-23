# oasis7：Skill Surface 替换治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`

审计轮次: 1

> 当前 PR 的 `.pm` canonical trace 已并档到单个 aggregate task `task_de7dbd97ffdb485eb4a869cc8ac0673a`。

## 任务拆解（含 PRD-ID 映射）
- [x] skill-inventory-rationalization (PRD-ENGINEERING-032) [test_tier_required]: 冻结当前 `.agents/skills/` inventory 的 keep/replace/retire/defer 矩阵，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 surface。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] repo-owned-skill-surfaces (PRD-ENGINEERING-032/PRD-ENGINEERING-032C/PRD-ENGINEERING-AWB-007/008) [test_tier_required]: 将 `writing-skills`、`test-driven-development`、`brainstorming` 的可 salvage 部分翻译成 skill authoring surface、`tdd-test-writer` 与 `bounded-brainstorming` 的 repo-owned 边界。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] workflow-router-skill-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032E/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 收口为本地 `repo-owned-workflow-router` skill，并同步回写 skill inventory 与 root workflow phase order。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml

## Planned Follow-ups
- `generic-game-skill-mirror-retirement-followup` (`PRD-ENGINEERING-032`, target `test_tier_required`): 继续评估 `asset-optimization` 到 `synchronization-algorithms` 这一组 generic game-skill mirror 是否转成“上游跟踪清单”而非本地长期维护。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `.agents/roles/*.md`
- `.agents/skills/*/SKILL.md`
- `.agents/skills/README.md`
- `.agents/skills/templates/SKILL.template.md`
- `.agents/skills/checklists/skill-authoring-checklist.md`

## File Structure / Affected Paths
- 主要改动路径: `.agents/skills/{README.md,templates,checklists,bounded-brainstorming,repo-owned-workflow-router,tdd-test-writer,writing-repo-owned-skills}`、`AGENTS.md`、`doc/engineering/self-evolution/*superpowers*`、`doc/engineering/project.md`
- 只读依赖: `skills-lock.json`、`.agents/skills/{verification-before-completion,systematic-debugging}/SKILL.md`
- 验证入口: `./scripts/pm/lint.sh`、`./scripts/doc-governance-check.sh`、`git diff --check`
- 正式回写面: `skill-surface-replacement-governance-2026-05-19.{prd,project}.md`、`doc/engineering/project.md`

## 状态
- 更新日期: 2026-05-22
- 当前阶段: planned
- 当前任务: `generic-game-skill-mirror-retirement-followup`
- 关键缺口: `generic-game-skill-mirror-retirement-followup` 仍需先盘清 generic mirror 簇的真实引用面和上游同步需求。
- 下一步: 优先判断这组 generic mirror 是否应整体降为“上游跟踪清单”，而不是继续本地长期维护。
