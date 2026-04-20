# Rust 1200 行根治治理（2026-03-29）项目管理

- 对应设计文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- 对应需求文档: `doc/engineering/rust-governance/rust-1200-line-root-cause-governance-2026-03-29.prd.md`

审计轮次: 7

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-051 (PRD-ENGINEERING-R1200-001/002/005) [test_tier_required]: 产出 Rust 1200 行根治治理专题 `prd/design/project`，并同步回写 engineering 模块入口、索引与 devlog。
- [x] TASK-ENGINEERING-052 (PRD-ENGINEERING-R1200-001/003) [test_tier_required]: 新增 Rust 文件体量检查脚本、冻结当前超限基线，并接入 `scripts/ci-tests.sh required`。
- [x] TASK-ENGINEERING-053 (PRD-ENGINEERING-R1200-002/003) [test_tier_required]: 在门禁中增加 `touch-and-shrink` 与 `split_part/include!` 完成态阻断规则。
- [x] TASK-ENGINEERING-054 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `chain_runtime` 首批目录模块化治理，优先处理 `oasis7_chain_runtime.rs` 与 `execution_bridge.rs`。
- [x] TASK-ENGINEERING-055 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `viewer/runtime_live` 首批目录模块化治理，并补齐对应 live / Web 验证链路。
- [x] TASK-ENGINEERING-056 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `oasis7_viewer` 首批治理，并继续清理 runtime / launcher 余量；本轮门禁已将生产 Rust 超限文件清零。
- [x] TASK-ENGINEERING-057 (PRD-ENGINEERING-R1200-004/005) [test_tier_required]: 收口超限测试文件治理策略，冻结残余 13 个测试尾债并建立下一批 burn-down 清单。
- [x] TASK-ENGINEERING-104 (PRD-ENGINEERING-R1200-001/003/005) [test_tier_required]: 修复 `rust-oversized-file-baseline.tsv` 被误清空后的 required gate 阻断，按当前仓库实况重写 frozen baseline，并把专项状态从“已收口”拉回到继续 burn-down 的真实阶段。
- [x] rust-size-gate-recovery-and-mid-burn-down (PRD-ENGINEERING-R1200-001/002/003/004/005) [test_tier_required]: 合并记录 Rust-size 治理中段收口，包括 oversized gate 真值修复，以及 `runtime/world/module_actions`、`oasis7_node/src/lib.rs`、`oasis7_net/src/libp2p_net.rs`、`oasis7_node/src/types.rs` 四个热点文件的语义化拆分/热点抽离。 Trace: .pm/tasks/task_19e73f36db1040ccbf5eb579ade2e310.yaml
- [x] clear-rust-size-baselines (PRD-ENGINEERING-R1200-001/002/003/005) [test_tier_required]: 作为最终收口任务，在同一治理任务内退役 `rust-structural-slicing-baseline.tsv` 与 `rust-oversized-file-baseline.tsv`，把全部 `split_part/impl_part` 存量债和最后 7 个超限 Rust 文件改成语义化模块，并将 `check-rust-file-size` 收口为 oversized/structural 双零扫描门禁。 Trace: .pm/tasks/task_d2e428f00e5047e581061c8cb75963ef.yaml
- [x] shrink-near-limit-rust-hotspots (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required]: 在零扫描门禁恢复后，对 5 个逼近 1200 行阈值的热点文件执行预防性职责拆分，避免下一轮正常开发把 `llm_sidecar`、`oasis7_node::lib`、`oasis7_provider_parity_bench`、`oasis7_wasm_router::lib`、`self_guided` 再次推回超限。 Trace: .pm/tasks/task_fd49238273e0447d8189df40519a51b0.yaml

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
- `scripts/check-rust-file-size.sh`
- `AGENTS.md`
- Batch A (`TASK-ENGINEERING-052/053`) 依赖 `TASK-ENGINEERING-051` 完成并冻结规则口径。
- Batch B/C/D (`TASK-ENGINEERING-054~057`) 依赖 `TASK-ENGINEERING-052/053` 先把门禁、基线与完成态建立起来。

## 状态
- 更新日期: 2026-04-20
- 当前阶段: active
- 当前任务: `shrink-near-limit-rust-hotspots` 已完成，在 `clear-rust-size-baselines` 退役所有 baseline 之后，继续对 5 个 near-limit 热点做预防性职责拆分；当前真实约束仍为 `oversized code files=0, test files=0, structural slice files=0, include targets=0` 必须长期保持。
- 阻塞项: 无；后续若再次出现任一超限 Rust 文件或结构切片命名，required gate 直接阻断。
- 最新完成:
  - `shrink-near-limit-rust-hotspots`：`crates/oasis7/src/viewer/runtime_live/llm_sidecar.rs` 抽出 `llm_sidecar_runtime_support.rs`，`crates/oasis7_node/src/lib.rs` 抽出 `consensus_support.rs` 与 `replica_maintenance_support.rs`，`crates/oasis7/src/bin/oasis7_provider_parity_bench.rs` 抽出 `src/bin/oasis7_provider_parity_bench/io_support.rs`，`crates/oasis7_wasm_router/src/lib.rs` 抽出 `filtering.rs`，`crates/oasis7_client_launcher/src/self_guided.rs` 抽出 `self_guided_storage.rs`；5 个热点主文件现已降到 `858 / 841 / 981 / 860 / 1131` 行。验证已通过 `env -u RUSTC_WRAPPER cargo check -p oasis7_wasm_router -p oasis7_client_launcher -p oasis7_node -p oasis7 --bin oasis7_provider_parity_bench`、`cargo test -p oasis7_wasm_router --lib`、`cargo test -p oasis7 --lib bind_agent_player_emits_unbind_before_rebind_for_same_agent -- --nocapture`、`cargo test -p oasis7_client_launcher self_guided -- --nocapture`、`cargo test -p oasis7_node --lib --no-run`、`./scripts/check-rust-file-size.sh` 与 `./scripts/doc-governance-check.sh`。
  - `rust-size-gate-recovery-and-mid-burn-down`：中段治理已压缩为一组聚合记录。先通过 `TASK-ENGINEERING-104` 修复 `rust-oversized-file-baseline.tsv` 空壳导致的 required gate 误报，再连续完成四个热点文件收缩：`runtime/world/module_actions` 改为目录模块、`crates/oasis7_node/src/lib.rs` 迁移为 `node_engine_{core,network,storage_challenge}.rs`、`crates/oasis7_net/src/libp2p_net.rs` 抽出 `constructor_support.rs`、`crates/oasis7_node/src/types.rs` 抽出 `types/main_token_controller_binding.rs`。对应 targeted `cargo check/test` 与 `./scripts/check-rust-file-size.sh` 均已通过；`cargo test -p oasis7_node --lib -- --nocapture` 的 3 条既有 replication/network 失败签名仅作留痕，未在本轮扩大。
  - `TASK-ENGINEERING-054`：`oasis7_chain_runtime.rs` 拆出 `cli.rs`，`execution_bridge.rs` 迁移为目录模块，门禁基线已退休旧入口超限项。
  - `TASK-ENGINEERING-055`：`viewer/runtime_live.rs` 与测试集拆为目录模块；同时把 `runtime/events.rs`、`state.rs`、`apply_domain_event_*`、`world/event_processing.rs` 压回 1200 行内，清零本批新增 runtime 超限。
  - `TASK-ENGINEERING-056`：`oasis7_viewer` 首批治理与本轮 runtime / launcher 收尾已完成，退役了 `egui_right_panel_player_guide.rs`、`web_test_api.rs`、`viewer_automation.rs` 以及 `oasis7_provider_parity_bench.rs`、`oasis7_pure_api_client.rs`、`oasis7_provider_local_bridge.rs`、`oasis7_web_launcher/control_plane.rs`、`runtime/world/persistence.rs`、`viewer/live_split_part1.rs`、`runtime/world/governance.rs`、`runtime/world/module_actions_impl_part2.rs` 的生产超限基线。
  - `TASK-ENGINEERING-057`：历史上曾以 `rust-oversized-file-baseline.tsv` 承接测试尾债过渡；该过渡基线现已随最后一批 burn-down 一并退役，不再作为当前治理真值。
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
  - `clear-rust-size-baselines`：`crates/oasis7` / `crates/oasis7_consensus` / `crates/oasis7_node` 内全部 `split_part` / `impl_part` 存量文件已批量改成语义化文件名；与此同时，`control_plane.rs`、`llm_sidecar.rs`、`launcher_core.rs`、`replication.rs`、`auth_actions.rs`、`main_tests.rs`、`oasis7_net/src/tests.rs` 全部压回 `<= 1200` 行，`doc/.governance/rust-structural-slicing-baseline.tsv` 与 `doc/.governance/rust-oversized-file-baseline.tsv` 已一并退役删除，当前扫描结果为 `oversized code files=0, test files=0, structural slice files=0, include targets=0`。
- 下一步:
  - 持续按既有门禁链路执行 `./scripts/check-rust-file-size.sh`、`./scripts/doc-governance-check.sh` 与 `git diff --check`，确保 `0 oversized / 0 structural slicing` 不回弹。
  - 后续若新功能逼近 1200 行阈值，优先提前拆职责模块，而不是重新引入 baseline 或 `split_part`。
