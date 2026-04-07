# task_a2a5c83cb80f4a6f9deb3dfcb5ca8377 Execution Log

- task_uid: task_a2a5c83cb80f4a6f9deb3dfcb5ca8377
- title: Make software_safe the default formal Web entry
- owner_role: viewer_engineer
- worktree_hint: /home/scc/worktrees/oasis7-world-simulator-software-safe-primary-web-default

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-07 21:11:32 CST / viewer_engineer
- 完成内容: 将 `crates/oasis7_viewer/index.html` 默认/`auto` Web 入口改为直接跳转 `software_safe`，仅 `render_mode=standard` 保留显式 visual QA 入口；同步把 `crates/oasis7/src/bin/oasis7_game_launcher.rs` 默认 URL 改为携带 `render_mode=software_safe`，并补齐 launcher URL 单测。
- 完成内容: 在 `crates/oasis7_viewer/software_safe_src/{legacy_core.js,main.jsx}` 为正式主入口补 canonical gameplay summary、blocked/handoff surface、available gameplay actions 与显式“Asset / Governance Lane 不暴露 main token transfer form”口径；`__AW_TEST__.getState()` 新增 `gameplaySummary` 观测字段，并通过 `crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs` 锁定 contract。
- 完成内容: 重新执行 `npm ci` 与 `npm run build:software-safe` 生成 fresh `crates/oasis7_viewer/software_safe.js`，并验证 `node crates/oasis7_viewer/scripts/software-safe-feedback-contract.test.mjs`、`env -u RUSTC_WRAPPER cargo test -p oasis7 build_game_url -- --nocapture`、`git diff --check` 通过。
- 遗留事项: `T21`/`TASK-WORLD_SIMULATOR-304` 的 formal gameplay vs `standard` visual QA 证据尚未重跑；README / current-entry / release claim 文案需等证据完成后再回写。

## 2026-04-07 21:11:32 CST / viewer_engineer
- 完成内容: 提交前独立 review agent 完成当前 diff 审查，结论为 no findings；确认默认入口改向、launcher URL、`software_safe` gameplay summary 与 handoff surface 没有发现明确回归或 contract 违背。
- 遗留事项: review 仅指出三项后续覆盖缺口仍待后续任务承接：`index.html` 的 `/` 与 `?render_mode=auto` browser-level redirect regression、`software_safe` gameplay summary 的 DOM-level 回归、以及 `TASK-WORLD_SIMULATOR-304` / `T21` 的 formal Web vs visual QA 证据重跑。
