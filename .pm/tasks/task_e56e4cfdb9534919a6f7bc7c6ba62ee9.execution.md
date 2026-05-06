# task_e56e4cfdb9534919a6f7bc7c6ba62ee9 Execution Log

- task_uid: task_e56e4cfdb9534919a6f7bc7c6ba62ee9
- title: implement bridge-service binding and route contract
- owner_role: runtime_engineer
- worktree_hint: /home/scc/worktrees/oasis7-p2p-oc-newapi-bridge-proposal

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-05-06 10:49:34 CST / runtime_engineer
- 完成内容: 新增独立二进制 `oasis7_newapi_bridge_service`，实现最小 CLI、HTTP 服务、repo-owned `bridge-state.json` 持久化、`/v1/bridge/bind`、`/v1/bridge/deposit-route`、活跃 binding 冲突校验、活跃 route 复用与过期重发逻辑。
- 完成内容: 补齐 7 个定向测试，覆盖 binding 幂等/冲突、route 复用/过期、HTTP contract、状态 reload 与 CLI 校验；执行 `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_newapi_bridge_service -- --nocapture` 通过。
- 完成内容: 回写 `doc/p2p/token/mainchain-token-newapi-quota-bridge-2026-05-06.project.md` 与 `doc/p2p/project.md`，将 `bridge-binding-and-route-contract` 标记为已完成，并明确当前仅完成 binding/route slice，尚未接入 watcher / `bridge_ledger` / `New API` adapter。
- 遗留事项: 下一任务应实现 `bridge-ledger-and-confirmation-engine`，把链上 deposit truth、确认窗口、异常态与幂等账本接入当前服务。

## 2026-05-06 11:27:02 CST / runtime_engineer
- 完成内容: 处理 PR #180 review comments，给已知 bridge API 路径补 `405 method_not_allowed` 响应，避免错误 HTTP method 落到泛化 `404`。
- 完成内容: 为 `route_ttl_seconds` 增加启动期和运行期双重溢出保护，避免毫秒换算在超大 CLI 配置下溢出；补齐 405 与 TTL overflow 定向测试。
- 完成内容: 对齐 bridge 设计文档中的 endpoint 路径与 binding 字段命名，修正为 `/v1/bridge/bind`、`/v1/bridge/deposit-route` 与 `oasis_sender_account_id`。
- 遗留事项: 待 push 后 resolve 当前 5 个 review threads，并继续观察 GitHub required checks / review 状态。
