# `p2p/node` 热点路径治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/p2p-node-path-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] p2p-node-path-governance (PRD-ENGINEERING-028) [test_tier_required]: 建立专题 `prd/design/project`、新增 `doc/p2p/node/README.md` 作为热点子域 canonical 入口，并回写 `p2p` 与 engineering 上游入口，正式将 `p2p/node` 收口为 `PRD-ENGINEERING-025` 的第三条已执行 follow-up。 Trace: .pm/tasks/task_533ac29c20a84ee8a5e6914839ad0761.yaml

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/p2p/README.md`
- `doc/p2p/prd.index.md`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: p2p node path landing page formalized
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: `p2p/node` 已完成当前批次路径级治理，后续应继续按 `testing -> 季度复核` 的顺序推进，而不是在当前 PR 里同时扩到 `distfs` / `blockchain` 或回到只改模块根入口首屏。
