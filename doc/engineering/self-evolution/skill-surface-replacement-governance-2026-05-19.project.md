# oasis7：Skill Surface 替换治理（2026-05-19）项目管理

- 对应需求文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.prd.md`
- 对应设计文档: `doc/engineering/self-evolution/skill-surface-replacement-governance-2026-05-19.design.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] skill-replacement-rationalization (PRD-ENGINEERING-032) [test_tier_required]: 冻结当前 `.agents/skills/` inventory 的 keep/replace/retire/defer 矩阵，并退役 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 skill surface，同步回写角色卡、活跃文档与 engineering 根入口。 Trace: .pm/tasks/task_e4d000db4c064cfc8a6487c453c41453.yaml

## Planned Follow-ups
- `generic-game-skill-mirror-retirement-followup` (`PRD-ENGINEERING-032`, target `test_tier_required`): 继续评估 `asset-optimization` 到 `synchronization-algorithms` 这一组 generic game-skill mirror 是否转成“上游跟踪清单”而非本地长期维护。
- `tdd-skill-boundary-followup` (`PRD-ENGINEERING-032`, target `test_tier_required`): 单独评估 `tdd-test-writer` 的边界，明确“按需 skill”与“默认工作流门禁”之间的关系，避免与当前 evidence-first/GitHub PR 主链冲突。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `.agents/roles/*.md`
- `.agents/skills/*/SKILL.md`

## 状态
- 更新日期: 2026-05-19
- 当前阶段: planned
- 当前任务: `generic-game-skill-mirror-retirement-followup`
- 阻塞项:
  - `generic-game-skill-mirror-retirement-followup` 仍需先盘清每个 generic mirror 的实际引用面与是否保留上游同步机制。
- 最新完成:
  - 已建立 skill rationalization 专题三件套，并完成 `documentation-writer`、`frontend-ui-ux`、`game-changing-features` 三个低耦合 skill surface 的退役与角色卡同步。
- 下一步:
  - 优先判断 generic game-skill mirror 簇是否应整体改成“上游跟踪清单”，而不是继续本地长期维护。
  - 单独评估 `tdd-test-writer` 的按需 skill 边界，避免它被误读成默认工程门禁。
