# `doc/devlog` 历史压缩与入口收口（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] devlog-history-compaction (PRD-ENGINEERING-026) [test_tier_required]: 建立 `devlog-history-compaction` 专题三件套、新增 `doc/devlog/README.md` 作为 canonical archive 入口，并回写 engineering 根入口与 `doc-corpus-maintenance-governance` 上游状态，收口 `doc/devlog` 作为第一条已执行的维护成本 follow-up。 Trace: .pm/tasks/task_caaa7c575ec845dc9c0756c9e92d24f7.yaml

说明:
后续若继续做月度/阶段摘要，或批量清理 active 文档里 lingering 的“回写 devlog”口径，必须再切新的独立 worktree 和 `.pm` task；本批只解决 `doc/devlog` 的 canonical 入口与 archive 边界。

## 依赖
- `doc/devlog/*.md`
- `doc/README.md`
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M1 已完成
- 阶段说明: canonical archive entrypoint + upstream follow-up closure
- 阻塞项: 无
- 最近更新: 2026-04-17
- 后续动作: `doc/devlog` 入口已完成首批收口，下一步不再继续停在历史归档层，而是转入 `world-simulator` 的路径级治理；若后续证明 `doc/devlog` 仍需要更细粒度收口，再另开“月度/阶段摘要”专题，而不是继续堆单日文件说明。
