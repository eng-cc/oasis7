# 文档存量维护成本治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] doc-corpus-maintenance-governance (PRD-ENGINEERING-025) [test_tier_required]: 建立专题 `prd/design/project`、新增 `scripts/doc-inventory-report.sh`、回写 engineering 主入口与 `doc-surface-area-governance` handoff，冻结从“阅读面噪音”转向“存量维护成本”的阶段判断。 Trace: .pm/tasks/task_851d3d1452534a2c83355317ae385ade.yaml
- [x] devlog-history-compaction-followup (PRD-ENGINEERING-025/026) [test_tier_required]: 作为第一条 follow-up，建立 `devlog-history-compaction` 专题并新增 `doc/devlog/README.md`，把 `doc/devlog` 从“历史归档声明”收口到按月导航的 canonical archive 入口。 Trace: .pm/tasks/task_caaa7c575ec845dc9c0756c9e92d24f7.yaml

说明:
`doc/devlog` 历史压缩已完成第一批入口收口。后续 `world-simulator / p2p / testing` 路径级治理，以及季度复核是否接入 baseline/门禁扩展，均需各自单独创建新 worktree 与 `.pm` task，不在本专题项目页提前伪造未建项的 Trace。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md`
- `scripts/doc-governance-check.sh`
- `scripts/doc-inventory-report.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: formalize + report + first follow-up closed
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: 入口减重专题 `PRD-ENGINEERING-024` 在默认阅读面层面已完成收口，`doc/devlog` 也已完成第一批历史入口收口。后续应按 `world-simulator -> p2p -> testing -> 季度复核` 的顺序切独立 worktree 执行，不再回到“只改 README/首屏”的处理方式。
