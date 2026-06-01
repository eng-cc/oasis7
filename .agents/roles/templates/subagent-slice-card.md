# Subagent Slice Card

> 用途：在 `tpm` 派工前，先把每个专业角色 subagent slice 固化为单卡，确保 owner / task / worktree / PR 主链一致，并可回写 execution log。

## Required Fields（固定字段）
- owner role:
- slice type:
- input:
- write scope:
- return contract:
- validation command:
- formal sink:
- integration order:

## Disjoint Scope Checklist（并行写入必填）
- [ ] 本 slice 的 write scope 与其他并行 slice 完全不重叠（文件/目录/模块级）。
- [ ] 若存在共享文件，已降级为串行集成，且在 integration order 中标注先后。
- [ ] 已声明本 slice 不得回退或覆盖其他 subagent 已落地改动。
- [ ] 已声明冲突处理策略（rebase / manual merge / owner 手工集成）。
- [ ] 已定义完成信号（patch / findings / evidence / review）与回传路径，避免并行漂移到新真值。

## Example (copy/paste)
- owner role: producer_system_designer
- slice type: implementation
- input: `doc/<module>/project.md` task `<task slug>` + `.pm/tasks/<TASK-UID>.yaml`
- write scope: `crates/foo/**`（disjoint）
- return contract: patch + test evidence
- validation command: `./scripts/cargo-dev.sh test -p foo`
- formal sink: `.pm/tasks/<TASK-UID>.execution.md`
- integration order: 2/3（after runtime slice, before qa slice）

## Notes
- 一个 slice 一张卡；多角色并行时，必须逐张卡校验 disjoint scope checklist。
- slice card 链接/引用应回写 `.pm/tasks/<TASK-UID>.execution.md` 对应条目。
