# `doc/devlog` 历史压缩与入口收口（2026-04-17）项目管理文档

- 对应设计文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.design.md`
- 对应需求文档: `doc/engineering/doc-governance/devlog-history-compaction-2026-04-17.prd.md`

审计轮次: 1

## 任务拆解
- [x] devlog-history-compaction (PRD-ENGINEERING-026) [test_tier_required]: 建立 `devlog-history-compaction` 专题三件套、新增 `doc/devlog/README.md` 作为 canonical archive 入口，并回写 engineering 根入口与 `doc-corpus-maintenance-governance` 上游状态，收口 `doc/devlog` 作为第一条已执行的维护成本 follow-up。 Trace: .pm/tasks/task_caaa7c575ec845dc9c0756c9e92d24f7.yaml
- [x] devlog-summary-retire (PRD-ENGINEERING-026) [test_tier_required]: 将 `doc/devlog/README.md` 从逐日导航升级为 compact archive summary，删除已摘要的 57 份 `doc/devlog/2026-*.md` 日文件，并把仓库内具体日文件引用收敛到 `doc/devlog/README.md`。 Trace: .pm/tasks/task_eebbcc11039940dcbc99c540a15ece03.yaml

说明:
`doc/devlog` 已完成摘要替代和日文件退役；后续若批量清理 active 文档里 lingering 的“回写 devlog”口径，必须再切新的独立 worktree 和 `.pm` task。

## 依赖
- `doc/devlog/README.md`
- `doc/README.md`
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/README.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/doc-governance/doc-corpus-maintenance-governance-2026-04-17.project.md`
- `scripts/doc-governance-check.sh`

## 状态
- 当前阶段: M2 已完成
- 阶段说明: canonical compact archive summary + retired daily files
- 阻塞项: 无
- 最近更新: 2026-05-27
- 后续动作: `doc/devlog` 只保留 compact archive summary，不再新增日文件；当前执行证据继续写入 `.pm/tasks/*.execution.md`。
