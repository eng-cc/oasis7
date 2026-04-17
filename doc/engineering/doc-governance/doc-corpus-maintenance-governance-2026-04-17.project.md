# 文档存量维护成本治理（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] doc-corpus-maintenance-governance (PRD-ENGINEERING-025) [test_tier_required]: 建立专题 `prd/design/project`、新增 `scripts/doc-inventory-report.sh`、回写 engineering 主入口与 `doc-surface-area-governance` handoff，冻结从“阅读面噪音”转向“存量维护成本”的阶段判断。 Trace: .pm/tasks/task_851d3d1452534a2c83355317ae385ade.yaml
- [x] devlog-history-compaction-followup (PRD-ENGINEERING-025/026) [test_tier_required]: 作为第一条 follow-up，建立 `devlog-history-compaction` 专题并新增 `doc/devlog/README.md`，把 `doc/devlog` 从“历史归档声明”收口到按月导航的 canonical archive 入口。 Trace: .pm/tasks/task_caaa7c575ec845dc9c0756c9e92d24f7.yaml
- [x] world-simulator-viewer-path-followup (PRD-ENGINEERING-025/027) [test_tier_required]: 作为第二条 follow-up，建立 `world-simulator-viewer-path-governance` 专题并新增 `doc/world-simulator/viewer/README.md`，把 `world-simulator/viewer` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_7d222c2f13454b23889baad383fbdf7e.yaml
- [x] p2p-node-path-followup (PRD-ENGINEERING-025/028) [test_tier_required]: 作为第三条 follow-up，建立 `p2p-node-path-governance` 专题并新增 `doc/p2p/node/README.md`，把 `p2p/node` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_533ac29c20a84ee8a5e6914839ad0761.yaml
- [x] testing-evidence-path-followup (PRD-ENGINEERING-025/029) [test_tier_required]: 作为第四条 follow-up，建立 `testing-evidence-path-governance` 专题并新增 `doc/testing/evidence/README.md`，把 `testing/evidence` 从“热点路径内无首读入口”收口到按问题分流的 canonical 子域入口。 Trace: .pm/tasks/task_38707b4060b54e5e8b8ebcdb8d18a602.yaml

说明:
`doc/devlog` 历史压缩、`world-simulator/viewer`、`p2p/node` 与 `testing/evidence` 路径级治理都已完成首批入口收口。后续若要做季度复核或继续扩 `ci/longrun` 类路径治理，仍需至少各自独立创建 `.pm` task；默认仍建议独立 worktree，除非用户明确要求复用当前 PR/工作树。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-governance/doc-surface-area-governance-2026-04-10.project.md`
- `scripts/doc-governance-check.sh`
- `scripts/doc-inventory-report.sh`

## 状态
- 当前阶段: M2 已完成
- 阶段说明: formalize + report + first four follow-ups closed
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: 入口减重专题 `PRD-ENGINEERING-024` 在默认阅读面层面已完成收口，`doc/devlog`、`world-simulator/viewer`、`p2p/node` 与 `testing/evidence` 也已完成首批 follow-up。当前这一轮后续应转入季度复核，不再回到“只改 README/首屏”的处理方式。
