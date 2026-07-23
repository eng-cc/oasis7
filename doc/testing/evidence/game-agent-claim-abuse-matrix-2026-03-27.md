# Game Agent Claim Abuse Matrix (2026-03-27)

审计轮次: 1

## Meta
- Owner Role: `qa_engineer`
- Review Role: `producer_system_designer`
- Scope: `TASK-GAMEPLAY-AGC-004` for `PRD-GAME-011`
- Topic: `doc/game/gameplay/gameplay-agent-claim-economy-contract.project.md`

## 目标
- 把 agent claim 专题里需要 QA 守门的 abuse / accounting coverage 收敛成一份可引用矩阵。
- 明确本专题的 `required/full` 真值：
  - `required`: 以 `runtime::tests::agent_claims` 定向 Rust 回归为真值，冻结并发争抢、tier cap、grace 恢复、release refund、forced reclaim slash/refund 的状态机与账本语义。
  - `full`: 在当前专题仍复用同一套 runtime claim 测试，但要求在 `test_tier_full` 入口下再次通过，确认 feature 组合不会改变 claim 经济与回收行为。

## Evidence Matrix
| 类别 | Required Evidence | Full Evidence | 当前结论 |
| --- | --- | --- | --- |
| concurrent single-owner atomicity | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture`，其中 `concurrent_claim_conflict_charges_only_winner = ok` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture`，同名用例再次 `ok` | `pass`。同 tick 竞争同一 target 时只有 1 个 owner 成功，失败方余额与 treasury 不脏。 |
| tier cap and slot cost scaling | 同一轮 required 命令，其中 `reputation_tier_scales_second_slot_and_enforces_claim_cap = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。`tier-1` 可持有 2 个 claim，`slot-2` 总成本高于 `slot-1`，第 3 个 claim 被 `cap exceeded` 拒绝。 |
| delinquent -> grace -> recovery | 同一轮 required 命令，其中 `grace_claim_recovers_when_owner_refills_before_deadline = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。grace 内补足余额后会结清累计 upkeep 并恢复 `claimed_active`，不会被错误回收。 |
| voluntary release refund | 同一轮 required 命令，其中 `release_refunds_remaining_bond_after_cooldown = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。release cooldown 结束后会退回全部 locked bond，且不会错误写入 slash bucket。 |
| upkeep delinquent reclaim settlement | 同一轮 required 命令，其中 `missed_upkeep_enters_grace_then_forced_reclaim = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。欠费回收会把 arrears 计入 `ecosystem_pool`、penalty 计入 `slash`，并按剩余 bond 退款。 |
| idle reclaim settlement | 同一轮 required 命令，其中 `idle_claim_emits_warning_then_reclaims = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。idle warning 与 forced reclaim 触发后，slash/refund 会按 canonical penalty bps 结算。 |

## 本轮执行命令
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture
```

## 结论
- `TASK-GAMEPLAY-AGC-004` 在当前 PRD 范围内可判定为 `pass`。
- 当前 claim 专题的 QA blocker 已从“系统级 abuse / accounting 空洞”收敛为“后续平衡是否继续维持当前默认参数”，下一步应切回 `producer_system_designer` 执行 `TASK-GAMEPLAY-AGC-005`。
- 补充观察：
  - 直接运行不带 `--lib` 的 `cargo test -p oasis7 --features test_tier_required runtime::tests::agent_claims:: -- --nocapture` 目前会被仓库内不相干的 `crates/oasis7/src/bin/oasis7_hosted_access.rs` 可见性编译错误阻断；该问题不影响本专题 runtime lib claim 真值，但应由对应 owner 另行收口。
