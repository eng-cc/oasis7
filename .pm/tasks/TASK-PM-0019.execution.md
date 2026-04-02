# TASK-PM-0019 Execution Log

- task_id: TASK-PM-0019
- title: Make workflow close working-memory guidance task-scoped and bootstrap-friendly
- owner_role: producer_system_designer
- worktree_hint: engineering-task-scoped-working-memory-checklist

## 2026-04-02 10:20:30 CST / producer_system_designer
- 完成内容: 复核 `workflow-report` / `working-memory-report` / `codex-working-memory` 的契约后确认，close checklist 之前错误地把角色级 working_memory 汇总当成当前 task 汇总，导致第一次抽取前要么看到误导性的全局条目数，要么没有明确的 bootstrap/extract 入口。已将 `workflow-report --phase close --task-id` 改为按当前 task 统计 working_memory；当当前 task 仍为 0 条时，checklist 现在明确提示先执行 `./scripts/pm/codex-working-memory.sh --task-id <TASK-ID> --role <role>`，而不是直接暴露 review/autoflow。
- 遗留事项: 待补跑 `./scripts/pm/required-tier-smoke.sh`、`./scripts/pm/lint.sh`、`git diff --check`，再做 close-phase、独立 subagent review 与 commit/landing 收口。

## 2026-04-02 10:24:20 CST / producer_system_designer
- 完成内容: 已验证 `workflow-report --phase close --task-id TASK-PM-0019` 现在显示 `working_memory_entries: 0`，并把 close checklist 切换为 bootstrap `codex-working-memory`，不再把角色全局 17 条 working_memory 误投到当前 task。同步补齐 `required-tier-smoke` 断言：零条目 task 必须看到 bootstrap-working-memory，且不能看到 review/autoflow。排查中还发现 smoke 初始化会继承仓库已有 `signals.jsonl`，导致“isolated temp root”被真实 triaged signal 污染；现已在 smoke 初始化阶段清空 signal inbox，恢复隔离性。`./scripts/pm/required-tier-smoke.sh`、`./scripts/pm/lint.sh`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 均已通过。
- 遗留事项: 待按仓库规则完成 close-phase 收口、独立 subagent review、commit 与 landing。

## 2026-04-02 10:27:10 CST / producer_system_designer
- 完成内容: 已处理独立 review 提出的唯一 finding：此前 smoke 只覆盖“零条目 => bootstrap”分支，未覆盖“已有 working_memory => review/autoflow”分支。现已在 `required-tier-smoke` 中追加 seeded working_memory 场景，断言同一 task 在 `entry_count=1` 时必须出现 `review-working-memory` 和 `autoflow-working-memory`，且不能再出现 bootstrap 提示。回归后 `./scripts/pm/required-tier-smoke.sh`、`./scripts/pm/lint.sh` 与 `git diff --check` 再次通过。
- 遗留事项: 待将 `TASK-PM-0019` 迁到 `done`，并完成 commit 与标准 landing。
