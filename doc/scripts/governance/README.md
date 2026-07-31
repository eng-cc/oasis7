# 脚本治理专题

这里记录通用脚本治理专题完成“专业权威合并”后的归属。当前规范不再由 dated PRD/design/project 三件套承载。

## 当前权威

| 目标 | 权威入口 |
| --- | --- |
| 模块能力、稳定入口、fallback、worktree 与收口承诺 | `../prd.md` |
| 参数权威边界、harness 隔离和 readiness/smoke 语义 | `../design.md` |
| 已完成任务与历史验证证据 | `GitHub task issue evidence comments` |
| 工程任务生命周期、PR 门禁和终态清理 | `../../engineering/workflow/source-of-truth.md` |
| 当前 CLI 参数与机器可读输出 | 脚本 `--help` 与对应测试 |

历史 dated triplet 的持续语义已回填到上述稳定权威，源文件已删除；需要追溯时使用 Git history、GitHub task issue evidence 和 `GitHub task issue evidence comments` 的 TASK-SCRIPTS 记录。

## 兼容边界

`scripts/land-task-worktree.sh` 仅是 local-only / fallback 兼容工具，永远不是默认最终合流入口。默认路径使用 `scripts/prepare-task-pr.sh` 并服从工程 workflow 真值；完成 worktree 只能通过 canonical post-merge cleanup 回收。

## 已删除的碎片

原角色交接 brief 与本目录下六组 dated triplet 都是已完成任务的历史载体。持续规则现已归入模块稳定权威；为避免同一规则多点漂移，源文件已删除。
