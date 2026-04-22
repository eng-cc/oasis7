# task_d0aac5fb3b664514ae8a85a3aabf2b2d Execution Log

- task_uid: task_d0aac5fb3b664514ae8a85a3aabf2b2d
- title: fix release build-web-dist failure
- owner_role: viewer_engineer
- worktree_hint: /home/scc/worktrees/oasis7-site-release-build-web-dist-fix

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-22 21:45:30 CST / viewer_engineer
- 完成内容: 复盘 `Release Packages` run `24769333021`，确认唯一阻断为 `build-web-dist` 构建 `wasm32` launcher 时 `WebStateSnapshot` 缺失 `chain_replication_status` 字段；已在 `crates/oasis7_client_launcher/src/web_api_support.rs` 补回该字段并对齐 `main_chain_status::WebChainReplicationStatus` 真值。
- 完成内容: 回写 `doc/site/github-pages/github-pages-release-download-pipeline-2026-03-01.project.md` 的 `T3Z build-web-dist Web snapshot schema 漂移热修`，记录 run id、失败签名、本地回归命令和当前下一步。
- 完成内容: 本地回归通过 `env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher apply_web_snapshot_tracks_chain_p2p_status_payload -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher connected_peer_detail_rows_follow_connected_peer_order -- --nocapture`、`cd crates/oasis7_client_launcher && env -u NO_COLOR trunk build --release --dist ../../output/release/web-launcher-dist`、`cd crates/oasis7_viewer && env -u NO_COLOR trunk build --release --dist ../../output/release/web-dist`；`build-web-dist` 两段入口均成功收口。
- 遗留事项: 仍需提交修复、走 PR/合流，并在远端重触发 `Release Packages` 验证 `build-web-dist` 不再失败。
