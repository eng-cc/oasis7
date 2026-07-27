# Runtime 数值安全与原子状态转移项目追踪

- 对应需求文档：`doc/world-runtime/runtime/runtime-numeric-safety.prd.md`
- 对应设计文档：`doc/world-runtime/runtime/runtime-numeric-safety.design.md`
- 当前状态：phase 1–6 已完成并归档到稳定权威；phase 7–15 未纳入本次吸收。

## 任务拆解

- [x] numeric-safety-authority-audit (PRD-WORLD_RUNTIME-041) [test_tier_required]: 核对 phase 1–6 的永久语义、当前实现与边界测试。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-authority-absorption (PRD-WORLD_RUNTIME-041) [test_tier_required]: 建立稳定 PRD/design/project 并吸收仍有效语义。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-source-retirement (PRD-WORLD_RUNTIME-041) [test_tier_required]: 删除 18 个阶段源文件并修复活动入口。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)
- [x] numeric-safety-local-verification (PRD-WORLD_RUNTIME-041) [test_tier_required]: 完成本地治理与定向边界测试，并把 frozen-head review、CI 与合入收尾移交 GitHub task evidence。 Trace: #2658 (task_530f70d6f4f547c4a6b64aa1da37a4a2)

## 已完成 evidence ledger

| 来源阶段 | 已完成能力 | 当前代码/测试 evidence |
| --- | --- | --- |
| phase 1 | World 资源、rule delta 与高风险事件原子拒绝 | `crates/oasis7/src/simulator/types.rs`；`runtime/world/resources.rs`；`runtime/world/step.rs`；`runtime/state/apply_domain_event_gameplay.rs`；`resource_stock_add_rejects_overflow`、`adjust_resource_balance_rejects_overflow`、`emit_resource_transfer_overflow_keeps_balances_atomic` |
| phase 2 | Node points 结算与 PoS stake/slot 受检推进 | `crates/oasis7/src/runtime/node_points.rs`；`crates/oasis7_consensus/src/node_pos.rs`；`settle_epoch_rejects_*_overflow_without_state_mutation` 及 node_pos overflow tests |
| phase 3 | Node height/slot、gap-fill 与 snapshot 恢复拒绝 | `crates/oasis7_node/src/pos_state_store.rs`；`tests_pos_engine_guardrails.rs`；`tests_hardening.rs` |
| phase 4 | Replication writer epoch/sequence 可失败定位 | `crates/oasis7_node/src/replication.rs`；`replication/tests.rs` 中同 writer、writer 切换、无 guard 三类测试 |
| phase 5 | Sequencer proposal 与 lease term/expiry 原子推进 | `crates/oasis7_consensus/src/sequencer_mainloop.rs`、`lease.rs` 及其 overflow-without-mutation tests |
| phase 6 | Membership schedule expiry 与 Duration 窄化边界 | `membership_reconciliation.rs`、`membership_recovery/stores.rs`、`membership_reconciliation_tests.rs`、`dht.rs`、`crates/oasis7_node/src/runtime_util.rs` |

## 维护任务

- 已完成：将 phase 1–6 的永久契约吸收到稳定 PRD/design。
- 已完成：将当前实现与测试入口归拢到本 evidence ledger。
- 已完成：删除已吸收的 18 个阶段源文件并修复活动索引。
- 已完成：明确 phase 7–15 未被本次合并覆盖。
- 后续：修改数值策略时，同步更新稳定 authority 与对应边界测试。

## 依赖

- 产品连续性边界：`doc/product/world-infrastructure/world-continuity-governance-and-recovery.prd.md`
- 模块权威入口：`doc/world-runtime/prd.md`
- Runtime/consensus/node 实现与测试：见上方 evidence ledger。

## 验证

- `./scripts/doc-governance-check.sh`
- `./scripts/readme-link-check.sh`
- 对已删除 phase 1–6 文件名执行精确陈旧引用扫描
- 运行与本次 authority map 对应的定向 runtime/consensus/node 测试
- `git diff --check`

## 状态

- 当前状态：phase 1–6 专业权威合并与本地验证完成；frozen-head lifecycle 状态以 GitHub task evidence 为准。
- 阻塞项：无。
- 非声明：本状态不等于集成、长稳或发布就绪。
