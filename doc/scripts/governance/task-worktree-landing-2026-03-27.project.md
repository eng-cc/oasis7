# oasis7: task worktree landing 标准入口（2026-03-27）（项目管理）

- 对应设计文档: `doc/scripts/governance/task-worktree-landing-2026-03-27.design.md`
- 对应需求文档: `doc/scripts/governance/task-worktree-landing-2026-03-27.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] WTL-1 (PRD-SCRIPTS-WTL-001/002/003) [test_tier_required]: 新增 `scripts/land-task-worktree.sh`，并同步 `AGENTS.md`、scripts 模块 PRD/project、索引与治理专题。

## 关键契约

### 1. landing 策略
| 字段 | 默认值 |
| --- | --- |
| source branch | 当前 branch |
| target branch | 本地 `main` |
| strategy | `rebase <target>` -> `merge --ff-only <source>` |
| cleanup | 只输出命令，不默认自动删除；landing 成功后必须执行 |

### 2. JSON 输出字段
| 字段 | 含义 |
| --- | --- |
| `source_branch` | 被合入的 task branch |
| `source_worktree` | source branch 对应 worktree |
| `target_branch` | 目标 branch，默认本地 `main` |
| `target_worktree` | target branch 对应 worktree |
| `source_head_before` | landing 前 source head |
| `source_head_after` | landing 后 source head |
| `target_head_before` | landing 前 target head |
| `target_head_after` | landing 后 target head |
| `rebase_status` | `already_up_to_date` / `rebased` / `would_rebase` |
| `result` | `dry_run` / `landed` / `already_landed` |
| `cleanup_commands` | landing 成功后必须执行的 source worktree / branch 回收命令 |

## 依赖
- `AGENTS.md`
- `scripts/land-task-worktree.sh`
- `scripts/new-task-worktree.sh`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-04-10
- 当前阶段：已完成
- 阻塞项：无
- 下一步：默认最终合流已迁到 `task-worktree-github-pr-closure-2026-04-10`；本专题仅保留给 local-only / fallback 兼容场景，继续冻结“本地 landing 后必须 cleanup”的契约。
