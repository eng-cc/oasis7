# task_dfe0d5fdffcf4e0cb5b28a28cda917c8 Execution Log

- task_uid: task_dfe0d5fdffcf4e0cb5b28a28cda917c8
- title: Fix software_safe step controls timing out without progress
- owner_role: runtime_engineer
- worktree_hint: world-simulator-software-safe-step-timeout

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->

## 2026-04-07 10:13:44 CST / runtime_engineer
- 完成内容: 在 `crates/oasis7/src/viewer/runtime_live.rs` 补齐 runtime live `step/play` 路径的 `llm_sidecar.request_decision()`，避免 live runtime 在未 priming mailbox 时直接消费空决策并返回 `TimeoutNoProgress`；同步把 `request_decision()` 可见性放宽到 `crate::viewer::runtime_live`。
- 完成内容: 新增 `crates/oasis7/src/viewer/runtime_live/tests/auth_actions.rs` 回归用例，使用本地 mock `openclaw_local_http` provider 验证 `ViewerControl::Step { count: 1 }` 会触发 provider 决策请求并返回 `ControlCompletionStatus::Advanced`。
- 完成内容: 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 runtime_step_control_ -- --nocapture`，结果 3/3 通过。
- 完成内容: 补齐当前 task worktree 的 `config.toml` `[llm]` 段，使其与主工作区保持一致，避免 browser/harness 复现时误落到“当前 worktree 未配置 LLM”的环境偏差。
- 完成内容: 修正 `scripts/viewer-software-safe-step-regression.sh` 的门禁，`completed_timeout` / 无世界进度不再被 summary 误判为 `ok: true`。
- 遗留事项: browser 真链路仍未完全收口。未同步 `[llm]` 配置时，`software_safe` step 会快速落成 `completed_timeout`；同步配置后，step 请求可发出但在 60s/210s 窗口内都未收到 terminal feedback，表现为真实 provider/LLM 调用长时间挂起或未回包，需继续排查 live runtime 与实际 provider 的交互耗时/阻塞点。

## 2026-04-07 11:24:41 CST / runtime_engineer
- 完成内容: 在 `crates/oasis7/src/viewer/runtime_live.rs` 为 live runtime 控制失败补齐 `Blocked` ack 回包路径，`world.step()` / authoritative batch 注册与 finality 推进失败不再直接中断 socket，而是回写结构化 `error_code` / `error_message`，并同步修正 `crates/oasis7/src/viewer/runtime_live/gameplay_snapshot.rs`、`crates/oasis7_viewer/src/web_test_api/{mod,wasm}.rs` 的 blocked hint，避免把 runtime 故障误标成 LLM lane 问题。
- 完成内容: 在 `crates/oasis7/src/simulator/llm_agent.rs` 移除了 OpenAI client 的隐式长超时 retry；当请求超时后直接按配置 timeout 返回 `LlmClientError::Http`，并在 `crates/oasis7/src/simulator/llm_agent/tests_split_part1.rs` 新增慢响应本地 HTTP 行为测试，验证请求会在配置 timeout 附近直接失败，不会再退化到隐藏长重试。
- 完成内容: 在 `crates/oasis7/src/bin/oasis7_game_launcher.rs` 增加 builtin LLM live 交互默认 timeout 注入：当父进程未显式提供 `OASIS7_LLM_TIMEOUT_MS` 且使用 builtin provider 时，launcher 为 `oasis7_viewer_live` 注入 `10000ms` 默认值；显式父环境变量与 openclaw provider 保持原有优先级不变。同步在 `crates/oasis7/src/bin/oasis7_game_launcher/oasis7_game_launcher_tests.rs` 新增 helper 级与实际 spawn command 级回归，覆盖默认 timeout、显式覆盖、openclaw 隔离与 `--llm` 启动接线。
- 完成内容: 运行 `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_game_launcher -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib runtime_step_control_ -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 openai_client_respects_configured_timeout_without_hidden_retry -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7_viewer web_test_api -- --nocapture`，结果全部通过；review agent 提出的“timeout 行为测试过弱”和“spawn path 缺少自动化覆盖”两项结论已消化。
- 完成内容: 真实 live 复验两组链路：
  1. 默认配置 `./scripts/run-game-test.sh --viewer-port 5073 --live-bind 127.0.0.1:6083 --web-bind 127.0.0.1:6071 --chain-status-bind 127.0.0.1:6181 --run-id stepfix-20260407-default --viewer-static-dir output/playwright/playability/startup-20260407-110530/web-dist --json-ready`
  2. 判定实验 `OASIS7_LLM_TIMEOUT_MS=10000 ./scripts/run-game-test.sh --viewer-port 4973 --live-bind 127.0.0.1:5983 --web-bind 127.0.0.1:5971 --chain-status-bind 127.0.0.1:6081 --run-id stepfix-20260407-timeout10s --viewer-static-dir output/playwright/playability/startup-20260407-110530/web-dist --json-ready`
- 完成内容: 真实复验结果表明 root cause 为 builtin LLM 默认 `180000ms` timeout 不适合 `software_safe` 的交互式 step。修复后在默认配置下，`target/debug/oasis7_pure_api_client --addr 127.0.0.1:6083 --timeout-ms 30000 step --count 1 --events --metrics` 会在约 `10007ms` 返回 `control_completion_ack.status=blocked`，并给出 `error_code=llm_init_failed`、`error_message=http request failed: request timed out after 10000ms ...`，不再表现为用户侧“连接关闭/长期挂死”。
- 遗留事项: 当前 letai provider 在 `10000ms` 窗口内仍未产出可执行决策，因此 step 现在是 fail-fast blocked，而不是 `Advanced`；这已满足“非 timeout 挂死”的产品门槛，但若要进一步提高可玩性，需要后续继续优化 provider 延迟、prompt 体积或切换到更稳定的 agent provider lane。
