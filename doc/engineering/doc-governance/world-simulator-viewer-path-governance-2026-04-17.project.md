# `world-simulator/viewer` 热点路径治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/world-simulator-viewer-path-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] world-simulator-viewer-path-governance (PRD-ENGINEERING-027) [test_tier_required]: 建立专题 `prd/design/project`、新增 `doc/world-simulator/viewer/README.md` 作为热点子域 canonical 入口，并回写 `world-simulator` 与 engineering 上游入口，正式将 `viewer/` 收口为 `PRD-ENGINEERING-025` 的第二条已执行 follow-up。 Trace: .pm/tasks/task_7d222c2f13454b23889baad383fbdf7e.yaml

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/world-simulator/README.md`
- `doc/world-simulator/prd.index.md`
- `doc/world-simulator/viewer/viewer-manual.manual.md`
- `scripts/doc-inventory-report.sh`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: viewer path landing page formalized
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: `world-simulator/viewer` 已完成第一批路径级治理，后续应继续按 `p2p -> testing -> 季度复核` 的顺序推进，而不是在当前 PR 里继续扩展到 `launcher/llm`。
