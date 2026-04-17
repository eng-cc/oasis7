# `testing/evidence` 热点路径治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/testing-evidence-path-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] testing-evidence-path-governance (PRD-ENGINEERING-029) [test_tier_required]: 建立专题 `prd/design/project`、新增 `doc/testing/evidence/README.md` 作为热点子域 canonical 入口，并回写 `testing` 与 engineering 上游入口，正式将 `testing/evidence` 收口为 `PRD-ENGINEERING-025` 的第四条已执行 follow-up。 Trace: .pm/tasks/task_38707b4060b54e5e8b8ebcdb8d18a602.yaml

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/testing/README.md`
- `doc/testing/prd.index.md`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: testing evidence path landing page formalized
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: `testing/evidence` 已完成当前批次路径级治理；本轮后续应转入季度复核，而不是在当前 PR 里继续并行扩到 `ci` / `longrun` / `templates`。
