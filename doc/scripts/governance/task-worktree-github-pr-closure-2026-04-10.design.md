# oasis7 task worktree GitHub PR closure Design

审计轮次: 1

## 1. Context
- 上游前提：每个需求默认新开独立 task worktree，并通过 `scripts/new-task-worktree.sh` 建立统一 branch/path。
- 当前缺口：本地 `land-task-worktree.sh` 只能约束单机线性历史，无法表达 GitHub PR 的服务端保护边界。

## 2. Architecture
- branch 解析层：识别 source branch（默认当前 branch）、base branch（默认 `main`）和 remote（默认 `origin`）。
- worktree 围栏层：确认 source branch 已被某个 worktree 检出，且 source worktree 干净。
- comparison 层：优先读取 `refs/remotes/origin/main`，缺失时退回 `refs/heads/main`，计算 ahead/behind 与是否需要 rebase。
- PR create 层：`--create` 时先 push source，再执行 `gh pr create`。
- 输出层：打印人类摘要或单个 JSON 对象；同时输出 PR 合入后的本地同步/cleanup 命令。

## 3. Interface
- 主入口：`scripts/prepare-task-pr.sh`
- 参数契约：
  - `[source-branch]`: 可选；默认当前 branch。
  - `--base <branch>`: 默认 `main`。
  - `--remote <name>`: 默认 `origin`。
  - `--create`: 实际 push 并调用 `gh pr create`。
  - `--draft`: 仅对 `--create` 有效。
  - `--title <text>` / `--body-file <path>`: 覆盖默认 `--fill`。
  - `--json`: 输出单个 JSON 对象。
- 关键输出字段：
  - `source_branch`
  - `source_worktree`
  - `base_branch`
  - `comparison_ref`
  - `ahead_count`
  - `behind_count`
  - `rebase_required`
  - `create_command`
  - `post_merge_commands`
  - `cleanup_commands`
  - `pr_url`

## 4. State Machine
- `input -> source_resolved`
- `source_resolved -> guarded`
- `guarded -> compared`
- `compared -> pr_ready`
- `pr_ready -> pr_opened`
- `pr_opened -> merged -> cleaned_up`

## 5. Failure Semantics
- `source_branch_not_checked_out`: source branch 没有对应 worktree。
- `source_dirty`: source worktree `git status --short` 非空。
- `comparison_ref_missing`: `origin/main` 与本地 `main` 都不存在。
- `source_behind_base`: `--create` 时 source 落后于 comparison ref。
- `gh_missing`: `--create` 时 `gh` 不在 PATH。
- `pr_create_failed`: push 或 `gh pr create` 返回非零。

## 6. Rationale
- 选择把默认最终合流切到 GitHub PR，而不是继续强化本地 landing，是因为默认保护边界必须落在 required checks 与 review/approval，而不是单机历史整理。
- 保留 `land-task-worktree.sh` 作为兼容/应急工具，而不是立即删除，是为了保留离线、本地演练或用户显式要求的 fallback 路径。
