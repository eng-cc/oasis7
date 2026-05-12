# task_9c8babbd74814b0ea7b88c2dccf783e6 Execution Log

- task_uid: task_9c8babbd74814b0ea7b88c2dccf783e6
- title: align triad docs and snapshot help with three-equal-validator truth
- owner_role: producer_system_designer
- worktree_hint: /home/scc/worktrees/oasis7-p2p-triad-doc-truth-alignment

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-05-12 21:54:41 CST / producer_system_designer
- 完成内容: 对齐 triad source-of-truth 文档与 snapshot help；将 `doc/p2p/prd.md`、`doc/p2p/project.md`、triad observability 专题 PRD/project/design、`testing-manual.md`、`scripts/p2p-real-env-triad-snapshot.sh` 统一到“当前 real-env triad 物理上为本机 + 2 ECS，但 live runtime 已是 three_equal_validator，legacy service label 仅作兼容别名”的口径。
- 完成内容: 同步修正 testing manual 中三节点等权 validator baseline 的 claim 门槛，改为要求 `summary.json.claim_status == pass_candidate`，不再沿用旧的 `partial_with_local_validator_blocker` 文档说法。
- 验证: `bash -n scripts/p2p-real-env-triad-snapshot.sh`；`./scripts/doc-governance-check.sh`；`git diff --check`
- 遗留事项: 本轮未改历史 evidence / devlog 归档；这些文件继续保留其发生当时的拓扑事实。
