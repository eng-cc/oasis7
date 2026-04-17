# Rust 1200 行根治治理（2026-03-29）项目管理

- 对应设计文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- 对应需求文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-051 (PRD-ENGINEERING-R1200-001/002/005) [test_tier_required]: 产出 Rust 1200 行根治治理专题 `prd/design/project`，并同步回写 engineering 模块入口、索引与 devlog。
- [x] TASK-ENGINEERING-052 (PRD-ENGINEERING-R1200-001/003) [test_tier_required]: 新增 Rust 文件体量检查脚本、冻结当前超限基线，并接入 `scripts/ci-tests.sh required`。
- [x] TASK-ENGINEERING-053 (PRD-ENGINEERING-R1200-002/003) [test_tier_required]: 在门禁中增加 `touch-and-shrink` 与 `split_part/include!` 完成态阻断规则。
- [x] TASK-ENGINEERING-054 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `chain_runtime` 首批目录模块化治理，优先处理 `oasis7_chain_runtime.rs` 与 `execution_bridge.rs`。
- [x] TASK-ENGINEERING-055 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `viewer/runtime_live` 首批目录模块化治理，并补齐对应 live / Web 验证链路。
- [x] TASK-ENGINEERING-056 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `oasis7_viewer` 首批治理，并继续清理 runtime / launcher 余量；本轮门禁已将生产 Rust 超限文件清零。
- [x] TASK-ENGINEERING-057 (PRD-ENGINEERING-R1200-004/005) [test_tier_required]: 收口超限测试文件治理策略，冻结残余 13 个测试尾债并建立下一批 burn-down 清单。
- [x] TASK-ENGINEERING-104 (PRD-ENGINEERING-R1200-001/003/005) [test_tier_required]: 修复 `rust-oversized-file-baseline.tsv` 被误清空后的 required gate 阻断，按当前仓库实况重写 frozen baseline，并把专项状态从“已收口”拉回到继续 burn-down 的真实阶段。
- [x] oasis7-node-types-burn-down (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required]: 抽离 `crates/oasis7_node/src/types.rs` 的 main-token controller binding 类型与校验 helper 到 `crates/oasis7_node/src/types/controller_binding.rs`，将根文件从 1277 行压到 1069 行，并同步退休 frozen oversized baseline 项。 Trace: .pm/tasks/task_b46f0ea2302d4b289fe2b6977a0d3041.yaml

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.prd.md`
- `doc/engineering/rust-governance/oversized-rust-file-splitting-2026-02-23.project.md`
- `testing-manual.md`
- `scripts/ci-tests.sh`
- `scripts/doc-governance-check.sh`
- `doc/.governance/rust-oversized-file-baseline.tsv`
- `doc/.governance/rust-structural-slicing-baseline.tsv`
- `scripts/check-rust-file-size.sh`
- `AGENTS.md`
- Batch A (`TASK-ENGINEERING-052/053`) 依赖 `TASK-ENGINEERING-051` 完成并冻结规则口径。
- Batch B/C/D (`TASK-ENGINEERING-054~057`) 依赖 `TASK-ENGINEERING-052/053` 先把门禁、基线与完成态建立起来。

## 状态
- 更新日期: 2026-04-17
- 当前阶段: active
- 当前任务: `oasis7-node-types-burn-down` 已完成，`crates/oasis7_node/src/types.rs` 已通过目录子模块拆分退役 frozen baseline；当前 frozen baseline 为 4 个生产文件 / 6 个测试文件，专项继续处于持续 burn-down 状态。
- 阻塞项: 无新的脚本阻断；当前真实约束是仓库仍存在 4 个生产 Rust 超限文件与 6 个测试 Rust 超限文件，后续治理不得再宣称 `oversized code files=0, test files=0` 已达成。
- 最新完成:
  - `TASK-ENGINEERING-104`：确认 `doc/.governance/rust-oversized-file-baseline.tsv` 在 `2026-03-30` 被提交为只剩注释的空壳，导致 `scripts/check-rust-file-size.sh` 在 required gate 中把全部存量超限项误判为“相对 HEAD 新增”；现已按当前仓库实况重写 baseline，恢复 `scripts/ci-tests.sh required` 的 frozen baseline 语义，并把专项状态改回继续 burn-down。
  - `oasis7-node-types-burn-down`：`crates/oasis7_node/src/types.rs` 已将 main-token controller binding 抽到 `types/controller_binding.rs`，根文件从 1277 行降到 1069 行；`types::tests::` 8 项与 `tests_action_payload::consensus_auth_tests::` 19 项通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - `TASK-ENGINEERING-054`：`oasis7_chain_runtime.rs` 拆出 `cli.rs`，`execution_bridge.rs` 迁移为目录模块，门禁基线已退休旧入口超限项。
  - `TASK-ENGINEERING-055`：`viewer/runtime_live.rs` 与测试集拆为目录模块；同时把 `runtime/events.rs`、`state.rs`、`apply_domain_event_*`、`world/event_processing.rs` 压回 1200 行内，清零本批新增 runtime 超限。
  - `TASK-ENGINEERING-056`：`oasis7_viewer` 首批治理与本轮 runtime / launcher 收尾已完成，退役了 `egui_right_panel_player_guide.rs`、`web_test_api.rs`、`viewer_automation.rs` 以及 `oasis7_provider_parity_bench.rs`、`oasis7_pure_api_client.rs`、`oasis7_provider_local_bridge.rs`、`oasis7_web_launcher/control_plane.rs`、`runtime/world/persistence.rs`、`viewer/live_split_part1.rs`、`runtime/world/governance.rs`、`runtime/world/module_actions_impl_part2.rs` 的生产超限基线。
  - `TASK-ENGINEERING-057`：`rust-oversized-file-baseline.tsv` 已刷新为只保留 13 个测试尾债；`module_actions_impl_part2` 新增 `release_support.rs`，`governance_internal.rs` 已放宽必要 helper 可见性，并在对齐最新 `main` 时同步把 `crates/oasis7_node/src/tests_action_payload.rs` 的冻结行数刷新到 1622。
  - 后续补修：`crates/oasis7_viewer` 已补齐 `player_experience_hud -> player_experience` 的薄包装 helper，并更新 `tests_selection_details.rs` 的 `AgentClaimState` 新字段；`cargo test -p oasis7_viewer --tests --no-run` 与 `cargo test -p oasis7_viewer` 已恢复通过。
  - 测试尾债 burn-down：`crates/oasis7_viewer/src/egui_right_panel_tests.rs` 已进一步拆出 `egui_right_panel_observe_tests.rs`，根文件降到 979 行；`cargo test -p oasis7_viewer egui_right_panel -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/viewer/live/tests.rs` 已拆出 `crates/oasis7/src/viewer/live/tests_auth.rs`，根文件降到 879 行；`cargo test -p oasis7 viewer::live::tests -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/module_action_loop_split_part1.rs` 已拆出 `crates/oasis7/src/runtime/tests/module_action_loop_market_tests.rs`，根文件降到 1007 行；`cargo test -p oasis7 runtime::tests::module_action_loop:: -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/persistence.rs` 已拆出 `crates/oasis7/src/runtime/tests/persistence_recovery_tests.rs`，根文件降到 1111 行；`cargo test -p oasis7 runtime::tests::persistence:: -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/gameplay_protocol_split_part1.rs` 已拆出 `crates/oasis7/src/runtime/tests/gameplay_protocol_policy_tests.rs`，根文件降到 1107 行；`cargo test -p oasis7 runtime::tests::gameplay_protocol::policy_tests:: -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。补充说明：`cargo test -p oasis7 runtime::tests::gameplay_protocol:: -- --nocapture` 仍有 2 个 builtin-wasm 相关用例因 Docker Hub `debian:bookworm-slim` metadata timeout 失败，本轮未改动其行为。
  - 测试尾债 burn-down：`crates/oasis7_node/src/tests_split_part1.rs` 已拆出 `crates/oasis7_node/src/tests_commit_execution_hashes.rs`，根文件降到 1130 行；`cargo test -p oasis7_node commit_signature_covers_execution_hashes -- --nocapture` 与 `cargo test -p oasis7_node pos_engine_ingests_commit_execution_hashes -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7_node/src/tests_split_part2.rs` 已拆出 `crates/oasis7_node/src/tests_network_gap_sync.rs`，根文件降到 1050 行；`cargo test -p oasis7_node runtime_network_replication_gap_sync_fetches_missing_commits -- --nocapture` 与 `cargo test -p oasis7_node runtime_network_replication_gap_sync_not_found_is_non_fatal -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7_client_launcher/src/main_tests.rs` 已拆出 `crates/oasis7_client_launcher/src/main_tests_onboarding.rs`，根文件降到 1033 行；`cargo test -p oasis7_client_launcher onboarding_auto_open_happens_only_once_per_session -- --nocapture` 与 `cargo test -p oasis7_client_launcher start_demo_mode_one_click_applies_safe_defaults -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7_node/src/tests_action_payload.rs` 已拆出 `crates/oasis7_node/src/tests_action_payload_consensus_auth.rs`，根文件降到 983 行；`cargo test -p oasis7_node submit_consensus_action_payload_accepts_signed_main_token_transfer_action -- --nocapture` 与 `cargo test -p oasis7_node submit_consensus_action_payload_accepts_signed_restricted_grant_admin_registry_action -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_tests.rs` 已拆出 `crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_explorer_tests.rs`，根文件降到 947 行；`cargo test -p oasis7 --bin oasis7_chain_runtime explorer_transactions_reject_invalid_status_filter -- --nocapture` 与 `cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_address_returns_not_found_for_unknown_account -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/economy.rs` 已拆出 `crates/oasis7/src/runtime/tests/economy_module_validation_tests.rs`，根文件降到 883 行；`cargo test -p oasis7 runtime::tests::economy::module_validation_tests::validate_product_with_module_uses_module_decision -- --nocapture` 与 `cargo test -p oasis7 runtime::tests::economy::module_validation_tests::schedule_recipe_marks_factory_blocked_and_resumes_after_inputs_recover -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/economy_priority_logistics.rs` 已拆出 `crates/oasis7/src/runtime/tests/economy_priority_governance_tests.rs`，根文件降到 1174 行；`cargo test -p oasis7 runtime::tests::economy_priority_logistics::governance_tests::govern_profile_actions_emit_events_and_update_profile_state -- --nocapture` 与 `cargo test -p oasis7 runtime::tests::economy_priority_logistics::governance_tests::industry_stage_progresses_from_bootstrap_to_scale_out_and_governance -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除该冻结项。
  - 测试尾债 burn-down：`crates/oasis7/src/runtime/tests/module_action_loop_split_part3.rs` 已拆出 `crates/oasis7/src/runtime/tests/module_action_loop_release_controls_tests.rs`，根文件降到 1147 行；`cargo test -p oasis7 runtime::tests::module_action_loop::release_controls_tests::module_release_shadow_rejects_missing_artifact_identity -- --nocapture`、`cargo test -p oasis7 runtime::tests::module_action_loop::release_controls_tests::rollback_module_instance_reverts_to_historical_version_and_emits_audit -- --nocapture` 与 `cargo test -p oasis7 runtime::tests::module_action_loop::release_controls_tests::module_release_apply_with_finality_succeeds_in_production_policy -- --nocapture` 通过，`rust-oversized-file-baseline.tsv` 已同步移除最后一条冻结测试基线。
- 下一步:
  - 继续按既有门禁链路执行 `./scripts/check-rust-file-size.sh`、`./scripts/doc-governance-check.sh` 与 `git diff --check`，确保当前 4 个生产文件 / 6 个测试文件之外不再新增超限项。
  - 重新拆解下一批 Rust 1200 行 burn-down 任务，优先处理 `viewer/runtime_live/llm_sidecar.rs`、`crates/oasis7/src/bin/oasis7_web_launcher/control_plane.rs`、`crates/oasis7_client_launcher/src/launcher_core.rs`、`crates/oasis7_node/src/replication.rs` 与对应高体量测试文件。
