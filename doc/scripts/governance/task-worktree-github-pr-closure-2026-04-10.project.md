# oasis7: task worktree GitHub PR closure 标准入口（2026-04-10）（项目管理）

- 对应设计文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.design.md`
- 对应需求文档: `doc/scripts/governance/task-worktree-github-pr-closure-2026-04-10.prd.md`

审计轮次: 1

## 任务拆解（含 PRD-ID 映射）
- [x] GPR-1 (PRD-SCRIPTS-GHPR-001/002/003) [test_tier_required]: 新增 `scripts/prepare-task-pr.sh`，并同步 `AGENTS.md`、`.pm/README`、scripts 模块文档与旧 landing compatibility 边界。

## 关键契约

### 1. PR 收口策略
| 字段 | 默认值 |
| --- | --- |
| source branch | 当前 branch |
| base branch | `main` |
| remote | `origin` |
| strategy | preflight source cleanliness + compare against `origin/main` or local `main` + optional `gh pr create` |
| cleanup | 只输出命令，不默认自动删除；PR 合入后必须执行 |

### 2. JSON 输出字段
| 字段 | 含义 |
| --- | --- |
| `source_branch` | 准备开 PR 的 task branch |
| `source_worktree` | source branch 对应 worktree |
| `base_branch` | PR base branch，默认 `main` |
| `comparison_ref` | 比较用 ref，优先 `origin/main` |
| `ahead_count` | source 相对 comparison ref 的领先提交数 |
| `behind_count` | source 相对 comparison ref 的落后提交数 |
| `rebase_required` | 是否必须先 rebase |
| `create_command` | 标准 `gh pr create` 命令 |
| `post_merge_commands` | PR 合入后必须执行的本地同步与回收命令全集 |
| `cleanup_commands` | `post_merge_commands` 的兼容别名，保持 agent 侧旧消费方可继续读取 |
| `pr_url` | `--create` 成功后返回的 PR URL |

## 依赖
- `AGENTS.md`
- `.pm/README.md`
- `scripts/prepare-task-pr.sh`
- `scripts/land-task-worktree.sh`
- `doc/scripts/project.md`

## 状态
- 更新日期：2026-04-10
- 当前阶段：已完成
- 阻塞项：无
- 下一步：若后续要补 PR 模板、自动等待 required checks 或 merge queue 辅助，应新开下一轮交付专题；本轮只冻结“默认最终合流经由 GitHub PR，旧 landing 仅保留 compatibility / fallback”的契约。
