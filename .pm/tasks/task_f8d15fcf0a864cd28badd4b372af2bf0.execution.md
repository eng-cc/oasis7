# task_f8d15fcf0a864cd28badd4b372af2bf0 Execution Log

- task_uid: task_f8d15fcf0a864cd28badd4b372af2bf0
- title: Fix software_safe prompt apply ack regression
- owner_role: viewer_engineer
- worktree_hint: world-simulator-software-safe-prompt-apply-ack-fix

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-03 11:10:00 CST / viewer_engineer
- 完成内容: 在独立 worktree 内复现 `software_safe` chat 回归，确认 `legacy_viewer_auth_bootstrap` 路径下 `authoritative_recovery_ack/error` 没有正确收口 `register_session` waiter，导致 `prompt apply` 卡在 queued/registering 之间且无法到达 `apply_ack`。
- 遗留事项: 继续修复 legacy preview recovery ack 合并逻辑，并复验 chat / rollback / message-flow 全链路。

## 2026-04-03 11:36:00 CST / viewer_engineer
- 完成内容: 修复 `crates/oasis7_viewer/software_safe_src/legacy_core.js` 中 legacy preview recovery ack 处理，允许 `session_registered` 正确更新 bound agent / runtime status，并避免无 `agent_id` 的后续 ack 清空既有绑定；同步重建 `crates/oasis7_viewer/software_safe.js`。
- 遗留事项: `prompt apply/rollback` 已恢复，但 `agent_chat` 暴露新的 `llm_init_failed` 阻断，需要继续收口 QA echo 的无 LLM 配置回归路径。

## 2026-04-03 11:44:00 CST / viewer_engineer
- 完成内容: 在 `crates/oasis7/src/viewer/runtime_live/control_plane.rs` 与 `crates/oasis7/src/viewer/runtime_live.rs` 增加 QA `agent_chat` echo fallback，允许 `OASIS7_RUNTIME_AGENT_CHAT_ECHO=1` 时在 `llm_init_failed` 场景下仍返回 chat ack、注入 `AgentSpoke` 虚拟事件并立即冲刷到订阅会话；新增 `runtime_agent_chat_echo_env_accepts_chat_without_llm_runner_config` 定向测试锁定行为。
- 遗留事项: 继续执行 step/chat browser 回归、文档治理与 PM lint，完成收口后进入 review / close / commit。

## 2026-04-03 11:47:00 CST / viewer_engineer
- 完成内容: 已通过独立 subagent review，未发现新的阻断缺陷；定向 Rust 测试、`software_safe` step/chat browser 回归、`doc-governance-check`、`pm-lint` 与 `git diff --check` 全部通过。
- 遗留事项: review 提醒当前仍缺一个更窄的 socket-level Rust 测试来锁定 `agent_chat` 后即时冲刷虚拟事件的分支；本次先以 browser 回归 + 单元测试覆盖收口，后续若继续演进 runtime_live 事件发送层，应补上协议级集成测试。
