# 脚本治理专题

这里是 `doc/scripts/governance/` 的唯一专题路由。先按当前要解决的治理问题选择一组权威三件套；不要把已完成的角色交接 brief 当作持续规范。

## 当前规范

| 目标 | 权威入口 |
| --- | --- |
| 高层脚本入口、主路径与 fallback 围栏 | `script-entry-layering-2026-03-11.prd.md` |
| 高频脚本参数、失败语义与 `skip-*` / `dry-run` 边界 | `script-parameter-contracts-2026-03-11.prd.md` |
| 新 task worktree 的 bootstrap | `task-worktree-bootstrap-2026-03-27.prd.md` |
| 隔离 harness 与状态文件契约 | `worktree-isolated-harness-2026-03-27.prd.md` |
| 默认 GitHub PR 收口与 `.pm` rebase 冲突分类 | `task-worktree-github-pr-closure-2026-04-10.prd.md` |

每个专题的 `.design.md` 和 `.project.md` 分别承载设计细节与完成记录；精确的全量三件套索引见上级 `doc/scripts/prd.index.md`。

## 兼容边界

`task-worktree-landing-2026-03-27.*` 仅说明 `scripts/land-task-worktree.sh` 的 local-only / fallback 兼容场景。默认最终合流一律回到 GitHub PR 收口专题，不要从 landing 专题推导默认流程。

## 已删除的碎片

原 `runtime-to-qa-task-scripts-002-entry-layering-2026-03-11.md` 与 `runtime-to-qa-task-scripts-003-parameter-contracts-2026-03-11.md` 是 2026-03 已完成任务的角色交接 brief；其持续有效的规则已完整落入对应 PRD/design/project 三件套，且无活跃调用方。为避免同一规则在 brief 和规范间漂移，已删除 brief；历史任务追溯保留在 Git history 与 `doc/scripts/project.md`。
