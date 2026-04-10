# oasis7 task worktree landing Design

审计轮次: 1

## 1. Context
- 上游前提：每个需求默认新开独立 task worktree，并通过 `scripts/new-task-worktree.sh` 建立统一 branch/path。
- 当前定位：该专题现仅保留 local-only / fallback 兼容路径；默认最终合流已经迁到 GitHub PR 收口。

## 2. Architecture
- branch 解析层：识别 source branch（默认当前 branch）和 target branch（默认本地 `main`）。
- worktree 映射层：通过 `git worktree list --porcelain` 找出 source/target 对应 worktree 路径。
- 围栏层：检查 source / target worktree 是否干净，阻断 detached HEAD、source=target、缺少 target worktree 等错误。
- 执行层：在 source worktree 上做 `git rebase <target>`；在 target worktree 上做 `git merge --ff-only <source>`。
- 输出层：打印人类摘要与必做 cleanup 命令；`--json` 时只输出结构化结果。

## 3. Interface
- 主入口：`scripts/land-task-worktree.sh`
- 参数契约：
  - `[source-branch]`: 可选；默认当前 branch。
  - `--target <branch>`: 默认本地 `main`。
  - `--dry-run`: 只解析 worktree、检查 clean 状态和预估 landing，不实际变更 git 历史。
  - `--json`: 输出单个 JSON 对象。
- 关键输出字段：
  - `source_branch`
  - `source_worktree`
  - `target_branch`
  - `target_worktree`
  - `source_head_before`
  - `source_head_after`
  - `target_head_after`
  - `result`
  - `cleanup_commands`

## 4. State Machine
- `input -> source_resolved`
- `source_resolved -> target_resolved`
- `target_resolved -> guarded`
- `guarded -> rebased`
- `rebased -> landed`
- `landed -> ready`

## 5. Failure Semantics
- `source_branch_not_checked_out`: 指定 source branch 没有对应 worktree。
- `target_branch_not_checked_out`: target branch 没有对应 worktree。
- `source_dirty` / `target_dirty`: 对应 worktree `git status --short` 非空。
- `rebase_failed`: `git rebase <target>` 返回非零。
- `fast_forward_failed`: `git merge --ff-only <source>` 返回非零。
- `already_landed`: target 已经包含 source，返回 no-op。

## 6. Rationale
- 该脚本保留是为了覆盖用户显式要求的 local-only 合流、离线演练或 PR 路径暂不可用的情况，而不是继续承担默认最终合流职责。
- 默认不自动 cleanup，是为了避免脚本在 source worktree 内自删当前目录，也保留 landing 后即时复核的余地；但 cleanup 仍被视为完成 fallback 流程后的必做步骤，而非可选建议。
