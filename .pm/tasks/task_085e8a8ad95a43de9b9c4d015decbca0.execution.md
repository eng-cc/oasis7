# task_085e8a8ad95a43de9b9c4d015decbca0 Execution Log

- task_uid: task_085e8a8ad95a43de9b9c4d015decbca0
- title: rename software_safe canonical ui mode to viewer
- owner_role: producer_system_designer
- worktree_hint: null

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-11 02:18:00 CST / producer_system_designer
- 完成内容: 将 `viewer` 收口为唯一正式 UI / Web 入口，`software_safe` 降为兼容 alias；同步更新 viewer runtime 状态命名、launcher URL 默认值、Web regression 口径、发布产物复制策略、当前真值 PRD / manual / README / site 手册镜像。
- 完成内容: 重建 `crates/oasis7_viewer/software_safe.js`，并通过 `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs` 与 `./scripts/doc-governance-check.sh`。
- 遗留事项: Rust 定向测试受到本机磁盘空间约束，`cargo test -p oasis7 ...` 级验证会触发 `No space left on device (os error 28)`；已改为准备执行更窄的 `--bin` 级验证以继续收口。
