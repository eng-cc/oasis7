# task_b14f4ccce06d4f179ed1de6c79b5b08e Execution Log

- task_uid: task_b14f4ccce06d4f179ed1de6c79b5b08e
- title: tighten builtin wasm integer-centimeter boundary
- owner_role: wasm_platform_engineer
- worktree_hint: /home/scc/worktrees/oasis7-world-runtime-builtin-wasm-integer-centimeter-contract

<!-- Append entries using:
Example:
  ## YYYY-MM-DD HH:MM:SS CST / role_name
  - 完成内容: ...
  - 遗留事项: ...
-->
## 2026-05-06 22:31:28 CST / wasm_platform_engineer
- 完成内容: 将 builtin wasm 的厘米边界收口到整数合同。`oasis7_wasm_sdk` 新增 `wire::GeoPosCm` 与整数厘米 JSON 解析入口，兼容读取旧的整值浮点 state，但对动作/事件/观测 JSON 显式拒绝 fractional cm；`m1.rule.move`、`m1.rule.visibility`、`m1.rule.transfer`、`m1.sensor.basic`、`m1.mobility.basic`、`m1.power.radiation_harvest`、`m1.power.storage` 已统一改为整数厘米主表示，`m1_rule_move` observability sample 与 runtime 断言同步改为整数值。
- 完成内容: 收口 builtin wasm 工件真值与验证链路。重跑 canonical m1 构建并用 `./scripts/sync-m1-builtin-wasm-artifacts.sh` 回写 `m1_builtin_modules.sha256` / `m1_builtin_modules.identity.json`，修复 runtime full regression 暴露的 builtin artifact hash mismatch；同时将 `tools/wasm_module_observe` 的冷启动执行预算提高到 10s，消除首次 compile+execute 被默认 `max_call_ms=2000` 误判为 timeout 的测试不稳定性。
- 完成内容: 回写 `doc/world-runtime/prd.md`、`doc/world-runtime/project.md`、`doc/world-runtime/wasm/wasm-interface.md`，补充 builtin wasm 整数厘米合同、旧 state 兼容边界、真实验收命令与产物列表。
- 验证: `env -u RUSTC_WRAPPER cargo test -p oasis7_wasm_sdk --features wire -- --nocapture`
- 验证: `env -u RUSTC_WRAPPER cargo test -p oasis7 --features test_tier_full scenario_modules_with_transfer_and_body_keep_wasm_closed_loop_consistent -- --nocapture`
- 验证: `env -u RUSTC_WRAPPER cargo test --manifest-path tools/wasm_module_observe/Cargo.toml observe_runner_executes_m1_rule_move_fixture -- --nocapture`
- 验证: `env -u RUSTC_WRAPPER cargo run --manifest-path tools/wasm_module_observe/Cargo.toml -- observe --spec crates/oasis7_builtin_wasm_modules/m1_rule_move/observability/module_observe.json --out-dir .tmp/wasm_module_observe_m1_integer_cm_after_timeout_fix`
- 验证: `./scripts/doc-governance-check.sh`
- 验证: `git diff --check`
- 遗留事项: 无；当前任务范围内的 builtin wasm 整数厘米边界、受跟踪工件与 observability 验证已闭环。
