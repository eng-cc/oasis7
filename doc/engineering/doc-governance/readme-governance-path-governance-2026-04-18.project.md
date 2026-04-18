# `readme/governance` 热点路径治理（2026-04-18）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.design.md`
- 对应需求文档: `doc/engineering/doc-governance/readme-governance-path-governance-2026-04-18.prd.md`

审计轮次: 1

## 任务拆解
- [x] readme-governance-path-governance (PRD-ENGINEERING-030) [test_tier_required]: 建立专题 `prd/design/project`、新增 `doc/readme/governance/README.md` 作为热点子域 canonical 入口，并回写 `readme` 与 engineering 上游入口，正式将 `governance/` 收口为 `PRD-ENGINEERING-025` 的第五条已执行 follow-up。 Trace: .pm/tasks/task_d37f636846fa44449988240af8630454.yaml

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/readme/README.md`
- `doc/readme/prd.index.md`
- `doc/readme/project.md`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: readme governance path landing page formalized
- 阻塞项: 无
- 最近更新: 2026-04-18
- 后续动作: `readme/governance` 已完成第一批路径级治理，后续应正式转入季度复核；若未来 `governance/` 内某个主题簇再次膨胀，再单独开“簇内治理/归档”专题，而不是继续在当前 PR 里横向扩展到 `gap/production`。
