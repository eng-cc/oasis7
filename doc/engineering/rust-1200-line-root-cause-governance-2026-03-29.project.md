# Rust 1200 行根治治理（2026-03-29）项目管理

- 对应设计文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.design.md`
- 对应需求文档: `doc/engineering/rust-1200-line-root-cause-governance-2026-03-29.prd.md`

审计轮次: 6

## 任务拆解（含 PRD-ID 映射）
- [x] TASK-ENGINEERING-051 (PRD-ENGINEERING-R1200-001/002/005) [test_tier_required]: 产出 Rust 1200 行根治治理专题 `prd/design/project`，并同步回写 engineering 模块入口、索引与 devlog。
- [x] TASK-ENGINEERING-052 (PRD-ENGINEERING-R1200-001/003) [test_tier_required]: 新增 Rust 文件体量检查脚本、冻结当前超限基线，并接入 `scripts/ci-tests.sh required`。
- [x] TASK-ENGINEERING-053 (PRD-ENGINEERING-R1200-002/003) [test_tier_required]: 在门禁中增加 `touch-and-shrink` 与 `split_part/include!` 完成态阻断规则。
- [x] TASK-ENGINEERING-054 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `chain_runtime` 首批目录模块化治理，优先处理 `oasis7_chain_runtime.rs` 与 `execution_bridge.rs`。
- [x] TASK-ENGINEERING-055 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `viewer/runtime_live` 首批目录模块化治理，并补齐对应 live / Web 验证链路。
- [x] TASK-ENGINEERING-056 (PRD-ENGINEERING-R1200-002/004/005) [test_tier_required] + [test_tier_full]: 完成 `oasis7_viewer` 首批治理，并继续清理 runtime / launcher 余量；本轮门禁已将生产 Rust 超限文件清零。
- [x] TASK-ENGINEERING-057 (PRD-ENGINEERING-R1200-004/005) [test_tier_required]: 收口超限测试文件治理策略，冻结残余 13 个测试尾债并建立下一批 burn-down 清单。

## 依赖
- `doc/engineering/prd.md`
- `doc/engineering/project.md`
- `doc/engineering/prd.index.md`
- `doc/engineering/README.md`
- `doc/engineering/oversized-rust-file-splitting-2026-02-23.prd.md`
- `doc/engineering/oversized-rust-file-splitting-2026-02-23.project.md`
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
- 更新日期: 2026-03-30
- 当前阶段: verified
- 当前任务: `TASK-ENGINEERING-056` / `TASK-ENGINEERING-057` 已完成；`scripts/check-rust-file-size.sh` 当前结果为 `oversized code files=0, test files=13`。
- 阻塞项:
  - `crates/oasis7_viewer` 的 `--tests` 编译链路仍有独立测试债：`tests_selection_details.rs` 中 `AgentClaimState` 初始化缺少新字段；该问题不属于本轮 1200 行生产文件治理阻断项。
- 最新完成:
  - `TASK-ENGINEERING-054`：`oasis7_chain_runtime.rs` 拆出 `cli.rs`，`execution_bridge.rs` 迁移为目录模块，门禁基线已退休旧入口超限项。
  - `TASK-ENGINEERING-055`：`viewer/runtime_live.rs` 与测试集拆为目录模块；同时把 `runtime/events.rs`、`state.rs`、`apply_domain_event_*`、`world/event_processing.rs` 压回 1200 行内，清零本批新增 runtime 超限。
  - `TASK-ENGINEERING-056`：`oasis7_viewer` 首批治理与本轮 runtime / launcher 收尾已完成，退役了 `egui_right_panel_player_guide.rs`、`web_test_api.rs`、`viewer_automation.rs` 以及 `oasis7_openclaw_parity_bench.rs`、`oasis7_pure_api_client.rs`、`oasis7_openclaw_local_bridge.rs`、`oasis7_web_launcher/control_plane.rs`、`runtime/world/persistence.rs`、`viewer/live_split_part1.rs`、`runtime/world/governance.rs`、`runtime/world/module_actions_impl_part2.rs` 的生产超限基线。
  - `TASK-ENGINEERING-057`：`rust-oversized-file-baseline.tsv` 已刷新为只保留 13 个测试尾债；`module_actions_impl_part2` 新增 `release_support.rs`，`governance_internal.rs` 已放宽必要 helper 可见性，并在对齐最新 `main` 时同步把 `crates/oasis7_node/src/tests_action_payload.rs` 的冻结行数刷新到 1622。
- 下一步:
  - 下一批 burn-down 以冻结测试尾债为唯一输入，优先顺序如下：`crates/oasis7/src/bin/oasis7_chain_runtime/transfer_submit_api_tests.rs`、`crates/oasis7/src/runtime/tests/economy.rs`、`crates/oasis7/src/runtime/tests/economy_priority_logistics.rs`、`crates/oasis7/src/runtime/tests/gameplay_protocol_split_part1.rs`、`crates/oasis7/src/runtime/tests/module_action_loop_split_part1.rs`、`crates/oasis7/src/runtime/tests/module_action_loop_split_part3.rs`、`crates/oasis7/src/runtime/tests/persistence.rs`、`crates/oasis7/src/viewer/live/tests.rs`、`crates/oasis7_client_launcher/src/main_tests.rs`、`crates/oasis7_node/src/tests_action_payload.rs`、`crates/oasis7_node/src/tests_split_part1.rs`、`crates/oasis7_node/src/tests_split_part2.rs`、`crates/oasis7_viewer/src/egui_right_panel_tests.rs`。
