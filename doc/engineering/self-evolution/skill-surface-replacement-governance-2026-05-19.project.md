# oasis7：Skill Surface 替换治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`

审计轮次: 1

> 注：当前 superpowers workflow PR 的 `.pm` canonical trace 已并档到单个 aggregate task `task_de7dbd97ffdb485eb4a869cc8ac0673a`。本页 rows 继续保留 skill-surface 维度的边界拆解，但不再各自占用独立 `.pm` task。

## 任务拆解（含 PRD-ID 映射）
- [x] skill-replacement-rationalization (PRD-ENGINEERING-032) [test_tier_required]: 冻结当前 `.agents/skills/` inventory 的 keep/replace/retire/defer 矩阵，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 skill surface，同步回写角色卡、活跃文档与 engineering 根入口。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] skill-authoring-surface-tightening (PRD-ENGINEERING-032/PRD-ENGINEERING-032C) [test_tier_required]: 将 upstream `writing-skills` 的可 salvage 部分翻译成 repo-owned skill authoring surface，新增本地 authoring entrypoint、template、checklist 与 bounded borrowing 说明，并同步回写 skill 治理专题、角色卡与 engineering 根项目。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] tdd-skill-boundary-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-007) [test_tier_required]: 将 `tdd-test-writer` 从“待单独评估”收口为已绑定 root workflow 的 bounded skill：仅用于行为变更且存在稳定自动化 harness 的任务，并通过 `AGENTS.md`、handoff/planning surface 与 borrowing/conflict 文档固定其默认适用边界。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] brainstorming-skill-boundary-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032D/PRD-ENGINEERING-AWB-008) [test_tier_required]: 将 upstream `brainstorming` 从“只保留 visual companion 子模式”的 rejected 状态收口为 repo-owned `bounded-brainstorming` skill：仅用于方向仍模糊、需要 scope decomposition / option framing 或判断是否需要 visual companion 的任务，并通过 `AGENTS.md`、handoff/planning surface 与 borrowing/conflict 文档固定其默认适用边界。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml
- [x] workflow-router-skill-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032E/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 收口为本地 `repo-owned-workflow-router` skill，并同步回写 `.agents/skills/README.md`、root `AGENTS.md` phase order、skill-surface / borrowing / conflict 文档与 `.pm` trace，明确保留的是 repo-owned workflow entrypoint 而非外部 bootstrap。 Trace: .pm/tasks/task_de7dbd97ffdb485eb4a869cc8ac0673a.yaml

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
- 改动路径:
  - `AGENTS.md`
  - `.agents/roles/producer_system_designer.md`
  - `.agents/roles/templates/{handoff-brief,handoff-detailed,planning-self-checklist}.md`
  - `.agents/skills/README.md`
  - `.agents/skills/{bounded-brainstorming,executing-project-tasks,repo-owned-workflow-router,tdd-test-writer,writing-repo-owned-skills}/SKILL.md`
  - `.agents/skills/templates/SKILL.template.md`
  - `.agents/skills/checklists/skill-authoring-checklist.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.{prd,design,project}.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.{prd,design,project}.md`
  - `doc/engineering/self-evolution/superpowers-conflict-reconciliation-2026-05-20.md`
  - `doc/engineering/project.md`
- 只读依赖:
  - `skills-lock.json`
  - `.agents/skills/verification-before-completion/SKILL.md`
  - `.agents/skills/systematic-debugging/SKILL.md`
- 验证入口:
  - `./scripts/pm/lint.sh`
  - `./scripts/doc-governance-check.sh`
  - `git diff --check`
- 正式回写面:
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`
  - `doc/engineering/project.md`

## 状态
- 更新日期: 2026-05-22
- 当前阶段: planned
- 当前任务: `generic-game-skill-mirror-retirement-followup`
- 阅读面说明: 本页只保留 inventory 治理任务、保留/替换边界与下一步；逐 slice 过程证据统一回到对应 `Trace: .pm/tasks/task_<32hex>.yaml` 与 `.execution.md`
- 阻塞项:
  - `generic-game-skill-mirror-retirement-followup` 仍需先盘清每个 generic mirror 的实际引用面与是否保留上游同步机制。
- 最新完成:
  - 已完成 skill inventory 基线冻结，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 surface；角色卡与工程入口已同步清理悬空引用。
  - 已完成 skill authoring 与 borrowed workflow surface 的本地化：`writing-repo-owned-skills`、template、checklist、`bounded-brainstorming`、`tdd-test-writer` 与 `repo-owned-workflow-router` 现已作为 repo-owned skill / entrypoint 保留在 inventory 内。
  - 已完成与 borrowing/conflict 主题的真值对齐：`writing-skills`、`brainstorming`、`test-driven-development`、`using-superpowers` 的可借部分已全部翻译到本地 surface，剩余 bootstrap / universal gate / distribution 语义继续保持 deferred 或 rejected。
- 下一步:
  - 优先判断 generic game-skill mirror 簇是否应整体改成“上游跟踪清单”，而不是继续本地长期维护。
