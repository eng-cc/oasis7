# Runtime 数值安全与原子状态转移项目追踪

- 对应需求文档：`doc/world-runtime/runtime/runtime-numeric-safety.prd.md`
- 对应设计文档：`doc/world-runtime/runtime/runtime-numeric-safety.design.md`
- 当前状态：phase 1–15 与 infinite-sequence rollover 已完成并归档到稳定权威。

## 任务拆解

- [x] numeric-safety-authority-audit (PRD-WORLD_RUNTIME-041) [test_tier_required]: 核对 phase 1–6 的永久语义、当前实现与边界测试。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-authority-absorption (PRD-WORLD_RUNTIME-041) [test_tier_required]: 建立稳定 PRD/design/project 并吸收仍有效语义。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-source-retirement (PRD-WORLD_RUNTIME-041) [test_tier_required]: 删除 18 个阶段源文件并修复活动入口。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-local-verification (PRD-WORLD_RUNTIME-041) [test_tier_required]: 完成本地治理与定向边界测试，并把 frozen-head review、CI 与合入收尾移交 GitHub task evidence。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-phase7-12-authority-audit (PRD-WORLD_RUNTIME-041) [test_tier_required]: 核对 phase 7–12 的永久语义、产品边界、当前实现与边界测试。 Trace: #2665 (task_99bbcd36b8ed43b4a39e86aa2e2d4478)
- [x] numeric-safety-phase7-12-absorption (PRD-WORLD_RUNTIME-041) [test_tier_required]: 将 phase 7–12 吸收到稳定 PRD/design/project，删除 18 个阶段源并修复活动入口。 Trace: #2665 (task_99bbcd36b8ed43b4a39e86aa2e2d4478)
- [x] numeric-safety-phase13-15-rollover-absorption (PRD-WORLD_RUNTIME-041) [test_tier_required]: 将 reconciliation/sync/mempool/federated checked arithmetic 与 persisted sequence rollover 收敛到本 authority，删除完成专题并修复活动入口。 Trace: #2712 (task_f1fdd2293cee427a8c605a62e7df6735)

## 已完成 evidence ledger

| 来源阶段 | 已完成能力 | 当前代码/测试 evidence |
| --- | --- | --- |
| phase 1 | World 资源、rule delta 与高风险事件原子拒绝 | `crates/oasis7/src/simulator/types.rs`；`runtime/world/resources.rs`；`runtime/world/step.rs`；`runtime/state/apply_domain_event_gameplay.rs`；`resource_stock_add_rejects_overflow`、`adjust_resource_balance_rejects_overflow`、`emit_resource_transfer_overflow_keeps_balances_atomic` |
| phase 2 | Node points 结算与 PoS stake/slot 受检推进 | `crates/oasis7/src/runtime/node_points.rs`；`crates/oasis7_consensus/src/node_pos.rs`；`settle_epoch_rejects_*_overflow_without_state_mutation` 及 node_pos overflow tests |
| phase 3 | Node height/slot、gap-fill 与 snapshot 恢复拒绝 | `crates/oasis7_node/src/pos_state_store.rs`；`tests_pos_engine_guardrails.rs`；`tests_hardening.rs` |
| phase 4 | Replication writer epoch/sequence 可失败定位 | `crates/oasis7_node/src/replication.rs`；`replication/tests.rs` 中同 writer、writer 切换、无 guard 三类测试 |
| phase 5 | Sequencer proposal 与 lease term/expiry 原子推进 | `crates/oasis7_consensus/src/sequencer_mainloop.rs`、`lease.rs` 及其 overflow-without-mutation tests |
| phase 6 | Membership schedule expiry 与 Duration 窄化边界 | `membership_reconciliation.rs`、`membership_recovery/stores.rs`、`membership_reconciliation_tests.rs`、`dht.rs`、`crates/oasis7_node/src/runtime_util.rs` |
| phase 7 | PoS 超多数比率与 required-stake 的无溢出三层一致语义 | `crates/oasis7_proto/src/distributed_pos.rs`、`crates/oasis7_consensus/src/pos.rs`、`crates/oasis7_node/src/pos_validation.rs`；`required_supermajority_accepts_extreme_ratio_just_above_half` 及对应 consensus/node 边界测试 |
| phase 8 | Membership retry/backoff 与 adaptive replay 阈值/比率安全 | `membership_recovery/types.rs`、`mod.rs`、`replay.rs`；`with_retry_failure_rejects_attempt_overflow`、`with_retry_failure_rejects_retry_timestamp_overflow` 及 dead-letter replay tests |
| phase 9 | Governance drill schedule 与 audit retention 时间受检语义 | `membership_recovery/replay_archive.rs`；`governance_recovery_drill_schedule_rejects_next_due_overflow_without_mutation`、`governance_audit_archive_prune_rejects_age_overflow_without_mutation` |
| phase 10 | Tiered offload、drill alert 与 rollback audit 的时间/计数原子失败 | `membership_recovery/replay_archive_tiered.rs`、`replay_audit.rs`；tiered-offload、alert age 与 rollback-streak overflow-without-mutation tests |
| phase 11 | Replay interval、policy cooldown 与 rollback cooldown 门控 | `membership_recovery/replay.rs`；`run_replay_schedule_with_state_store_rejects_interval_overflow_without_mutation`、`recommend_guarded_policy_rejects_cooldown_overflow_without_mutation` 及 persisted rollback-cooldown regression |
| phase 12 | Recovery scheduling、alert/dead-letter 容量与计数聚合 | `membership_recovery/mod.rs`；checked count helpers 与 recovery/dead-letter overflow regressions |
| phase 13 | reconciliation schedule/dedup 时间门控与 report 计数 | `crates/oasis7_consensus/src/membership_reconciliation.rs`、`membership_reconciliation_tests.rs`；`checked_usize_*`、schedule/dedup overflow-without-mutation tests |
| phase 14 | membership sync 与 mempool payload 聚合 | `membership_recovery_support.rs`、`mempool.rs`；sync report 与 batch/zone checked-add regressions |
| phase 15 | federated archive 聚合与复合游标 | `membership_recovery/replay_archive_federated.rs`、`replay_archive_federated/tests.rs`；aggregate/cursor overflow regressions |
| rollover | four `next_*_id` era persistence 与 legacy snapshot compatibility | `crates/oasis7/src/runtime/snapshot.rs`、`runtime/world/mod.rs`、`runtime/world/persistence.rs`、`runtime/tests/basic.rs`、`runtime/tests/persistence_recovery_tests.rs` |

## 维护任务

- 已完成：将 phase 1–15 与 sequence rollover 的永久契约吸收到稳定 PRD/design。
- 已完成：将当前实现与测试入口归拢到本 evidence ledger。
- 已完成：分两批删除 phase 1–12 的 36 个阶段源文件并修复活动索引。
- 已完成：保留 phase 13–15 的 `DistributedValidationFailed`/无部分更新语义，以及 era persistence、legacy snapshot default 与 ID-reuse 边界。
- 后续：修改数值策略时，同步更新稳定 authority 与对应边界测试。

## 依赖

- 产品连续性边界：`doc/product/world-infrastructure/world-continuity-governance-and-recovery.prd.md`
- 模块权威入口：`doc/world-runtime/prd.md`
- Runtime/consensus/node 实现与测试：见上方 evidence ledger。

## 验证

- `./scripts/doc-governance-check.sh`
- `./scripts/readme-link-check.sh`
- 对已删除 phase 1–12 文件名执行精确陈旧引用扫描
- 运行与本次 authority map 对应的定向 runtime/consensus/node 测试
- `git diff --check`

## 状态

- 当前状态：phase 1–15 与 sequence rollover 专业权威合并已完成；本地与 frozen-head lifecycle 状态以 GitHub task evidence 为准。
- 阻塞项：无。
- 非声明：本状态不等于集成、长稳或发布就绪。
