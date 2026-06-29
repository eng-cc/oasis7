# oasis7: 文档总入口公开阅读路径同步（2026-03-11）（项目管理）

- 对应设计文档: `doc/core/doc-readme-public-entry-sync-2026-03-11.design.md`
- 对应需求文档: `doc/core/doc-readme-public-entry-sync-2026-03-11.prd.md`

审计轮次: 4

## 任务拆解（含 PRD-ID 映射）
- [x] DOC-HUB-1 (PRD-CORE-007/008) [test_tier_required]: 定义 `doc/README.md` 的新阅读路径与时间戳同步范围。
- [x] DOC-HUB-2 (PRD-CORE-007/008) [test_tier_required]: 回写 `doc/README.md` 的快速阅读路径与更新时间。
- [x] DOC-HUB-3 (PRD-CORE-008) [test_tier_required]: 回写 core 主项目与 devlog。

## 依赖
- `doc/README.md`
- `README.md`
- `site/index.html`
- `doc/core/project.md`
- 历史 devlog 回写语义已退役；当前追溯以本项目页与 `.pm/tasks/task_<32hex>.execution.md` 为准。

## 状态
- 更新日期: 2026-03-11
- 当前阶段: 已完成
- 阻塞项: 无
- 下一步: 后续如公开入口继续变化，再按同一专题模式增量回写。
