# task_21dfffe808a24221a70fa5fe3fa895aa Execution Log

- task_uid: task_21dfffe808a24221a70fa5fe3fa895aa
- title: gate hosted_public_join to player-session lane and block local chain runtime
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-p2p-hosted-public-join-player-session-gate

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-04-24 13:33:00 CST / runtime_engineer
- 完成内容: 已在独立 worktree 内收口 `hosted_public_join` 的 launcher 真值边界。当前代码改动覆盖 `oasis7_client_launcher`、`oasis7_web_launcher`、`oasis7_game_launcher` 三条入口：`hosted_public_join` 下不再允许拉起本地 `oasis7_chain_runtime`，并把 viewer/public snapshot 的 hosted contract 补充为 `local_chain_runtime=blocked_for_public_player_plane` 与 `node_admission=operator_managed_node_onboarding_only`，避免继续把 public join 误建模成 shared-devnet 节点 bootstrap。
- 完成内容: 已同步回写 `doc/p2p/project.md` 与 `doc/p2p/blockchain/p2p-hosted-world-player-access-and-session-auth-2026-03-25.project.md`，将本次 slice 追踪到 `.pm/tasks/task_21dfffe808a24221a70fa5fe3fa895aa.yaml`。
- 遗留事项: 定向验证仍在运行：`cargo test -p oasis7_client_launcher`、`cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture`、`cargo test -p oasis7 --bin oasis7_game_launcher -- --nocapture` 还未全部返回，需要待构建完成后根据实际签名继续修复或确认通过。

## 2026-04-24 13:54:30 CST / runtime_engineer
- 完成内容: 定向验证已通过：`env -u RUSTC_WRAPPER cargo test -p oasis7_client_launcher`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_web_launcher -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher -- --nocapture`、`./scripts/doc-governance-check.sh` 与 `git diff --check` 全部通过。
- 完成内容: `oasis7_web_launcher` 新增了 `start_chain_process_rejects_hosted_public_join_local_runtime` 回归；`oasis7_client_launcher` / `oasis7_web_launcher` / `oasis7_game_launcher` 的 URL/contract 测试也已同步校验新的 `local_chain_runtime` 与 `node_admission` 提示字段。
- 遗留事项: 本轮未处理 production custody / external signer / operator-managed node admission 的真正后端实现；当前完成的是 public join 入口的强门禁与 truth-surface 收口，不代表整体已升级为 `mainnet-grade`。
