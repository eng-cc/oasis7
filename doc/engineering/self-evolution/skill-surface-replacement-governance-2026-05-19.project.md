# oasis7：Skill Surface 替换治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] skill-replacement-rationalization (PRD-ENGINEERING-032) [test_tier_required]: 冻结当前 `.agents/skills/` inventory 的 keep/replace/retire/defer 矩阵，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 skill surface，同步回写角色卡、活跃文档与 engineering 根入口。 Trace: .pm/tasks/task_e4d000db4c064cfc8a6487c453c41453.yaml
- [x] skill-authoring-surface-tightening (PRD-ENGINEERING-032/PRD-ENGINEERING-032C) [test_tier_required]: 将 upstream `writing-skills` 的可 salvage 部分翻译成 repo-owned skill authoring surface，新增本地 authoring entrypoint、template、checklist 与 bounded borrowing 说明，并同步回写 skill 治理专题、角色卡与 engineering 根项目。 Trace: .pm/tasks/task_ababcdcdc9694fa59acb8b1f0c5116df.yaml
- [x] tdd-skill-boundary-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-AWB-007) [test_tier_required]: 将 `tdd-test-writer` 从“待单独评估”收口为已绑定 root workflow 的 bounded skill：仅用于行为变更且存在稳定自动化 harness 的任务，并通过 `AGENTS.md`、handoff/planning surface 与 borrowing/conflict 文档固定其默认适用边界。 Trace: .pm/tasks/task_64823ec488b648cbb95cc99ed0f4bdfc.yaml
- [x] brainstorming-skill-boundary-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032D/PRD-ENGINEERING-AWB-008) [test_tier_required]: 将 upstream `brainstorming` 从“只保留 visual companion 子模式”的 rejected 状态收口为 repo-owned `bounded-brainstorming` skill：仅用于方向仍模糊、需要 scope decomposition / option framing 或判断是否需要 visual companion 的任务，并通过 `AGENTS.md`、handoff/planning surface 与 borrowing/conflict 文档固定其默认适用边界。 Trace: .pm/tasks/task_3cad85765a3447488acba03d163126d2.yaml
- [x] workflow-router-skill-reconciliation (PRD-ENGINEERING-032/PRD-ENGINEERING-032E/PRD-ENGINEERING-AWB-009) [test_tier_required]: 将 `using-superpowers` 中可借的 process-skill routing order 收口为本地 `repo-owned-workflow-router` skill，并同步回写 `.agents/skills/README.md`、root `AGENTS.md` phase order、skill-surface / borrowing / conflict 文档与 `.pm` trace，明确保留的是 repo-owned workflow entrypoint 而非外部 bootstrap。 Trace: .pm/tasks/task_f305aa614deb4c959d45ffa81599cfb3.yaml

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
  - `.agents/skills/README.md`
  - `.agents/skills/bounded-brainstorming/SKILL.md`
  - `.agents/skills/executing-project-tasks/SKILL.md`
  - `.agents/skills/repo-owned-workflow-router/SKILL.md`
  - `.agents/skills/tdd-test-writer/SKILL.md`
  - `.agents/skills/writing-repo-owned-skills/SKILL.md`
  - `.agents/skills/templates/SKILL.template.md`
  - `.agents/skills/checklists/skill-authoring-checklist.md`
  - `.agents/roles/producer_system_designer.md`
  - `.agents/roles/templates/handoff-brief.md`
  - `.agents/roles/templates/handoff-detailed.md`
  - `.agents/roles/templates/planning-self-checklist.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`
  - `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.project.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.design.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.prd.md`
  - `doc/engineering/self-evolution/agent-workflow-borrowing-governance-2026-05-19.project.md`
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
- 阻塞项:
  - `generic-game-skill-mirror-retirement-followup` 仍需先盘清每个 generic mirror 的实际引用面与是否保留上游同步机制。
- 最新完成:
  - 已建立 skill rationalization 专题三件套，并完成 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 skill surface 的退役与角色卡同步。
  - 已把 `writing-skills` 的可 salvage 部分收口成 repo-owned skill authoring surface：新增 `.agents/skills/README.md`、`writing-repo-owned-skills`、template 与 checklist；其中 authoring 结构、trigger wording 与 supporting-file discipline 已被吸收，但它自身的安装/发布包装与第二套工作流主链仍未采纳。
  - 已将 `tdd-test-writer` 的边界从“待评估”收口为 root workflow 的 bounded behavior-first contract：行为变更且有稳定自动化 harness 的任务默认先走 RED/回归测试或记录 skip reason，但它不升级为 universal TDD gate。
  - 已新增 repo-owned `bounded-brainstorming` skill，并将 upstream `brainstorming` 的可借部分收口为 root workflow 的 bounded ideation contract：只有方向仍模糊、需要拆 scope、做 2-3 方案对比或判断 visual companion 是否值得启用时才进入该 skill；它不升级为 universal brainstorming gate。
  - 已新增 repo-owned `repo-owned-workflow-router` skill，并将 upstream `using-superpowers` 里可借的 process-skill routing order 收口为非 trivial task 的默认 workflow entrypoint；保留的是本地 phase-order 路由，不是外部 bootstrap。
- 下一步:
  - 优先判断 generic game-skill mirror 簇是否应整体改成“上游跟踪清单”，而不是继续本地长期维护。
