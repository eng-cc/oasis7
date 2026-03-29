# Game Agent Claim Restricted Starter Balance Matrix (2026-03-29)

审计轮次: 1

## Meta
- Owner Role: `qa_engineer`
- Review Role: `producer_system_designer`
- Scope: `TASK-GAME-047` / `TASK-GAMEPLAY-AGC-009` for `PRD-GAME-011`
- Topic: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.project.md`

## 目标
- 把 `restricted starter claim balance` 的 QA 守门面收敛成一份可复跑矩阵。
- 明确当前 restricted starter 方案里哪些项已经有自动化真值，哪些项仍然是 release blocker。

## 本轮判定口径
- `required`:
  - 以 `runtime::tests::agent_claims`、`viewer::runtime_live::tests::compat_snapshot_*` 和 `oasis7_chain_runtime` 的 transfer/explorer 定向用例为真值。
  - 目标是冻结 `slot-1` restricted 专用消费、`slot-2/3` guard、refund provenance、transfer-only guard 和公开展示口径。
- `full`:
  - 继续要求 `runtime::tests::agent_claims` 在 `test_tier_full` 入口下通过，确认 feature 组合不会改变 claim 账本与回收语义。

## Evidence Matrix
| 类别 | Required Evidence | Full Evidence | 当前结论 |
| --- | --- | --- | --- |
| QA seed / manual restricted bootstrap | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture`，其中 `slot_1_claim_can_spend_restricted_balance = ok`；当前通过 `set_main_token_account_balance_with_restricted` 手动注入 QA seed，验证 restricted bucket 能启动首个 claim | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture`，同名用例再次 `ok` | `pass`，但仅限“手动 QA seed 注入”路径。当前还没有正式 `issuance_reason / issuer_id / expires_at_epoch` 发放链路。 |
| slot-1 upfront / upkeep restricted-only spend | 同一轮 required 命令，其中 `slot_1_claim_can_spend_restricted_balance = ok`、`slot_1_upkeep_uses_restricted_balance_before_liquid = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。`slot-1` 可先花 restricted，再补 liquid；restricted-only 余额可覆盖首个 claim 与后续 upkeep。 |
| mixed funding refund provenance | 同一轮 required 命令，其中 `mixed_slot_1_claim_tracks_bond_provenance_and_refunds_back_to_source_buckets = ok`、`forced_reclaim_preserves_refund_provenance_after_mixed_funding = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。release / forced reclaim 的 bond refund 会按 restricted/liquid 原来源拆回原 bucket，不会洗成 transferable liquid。 |
| slot-2 restricted guard | 同一轮 required 命令，其中 `slot_2_claim_cannot_spend_restricted_balance = ok`；`env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture`，其中 `compat_snapshot_flags_restricted_balance_as_ineligible_for_slot_2 = ok` | `test_tier_full` 入口下同名 runtime 用例再次 `ok` | `pass`。runtime 会拒绝 `slot-2` 消费 restricted；pure API canonical blocker 现明确返回 `restricted_balance_not_eligible_for_slot`。 |
| transfer guard and public balance surfaces | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_accounts_endpoint_exposes_restricted_balance_separately -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture` | 暂无单独 full lane；当前 bin 用例直接验证 transfer/explorer surface | `pass`。restricted-only 账户会被普通转账拒绝；transfer accounts / explorer address-assets 会展示 restricted 字段，但 `total_balance` 仍只按 `liquid + vested` 计算。 |
| viewer / pure API canonical quote split | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture`，其中 `compat_snapshot_exposes_player_agent_claim_overview = ok`、`compat_snapshot_flags_restricted_balance_as_ineligible_for_slot_2 = ok` | 暂无单独 full lane；当前以 compat snapshot 真值为准 | `pass`。claim snapshot 已对齐 `transferable_liquid_balance / restricted_starter_claim_balance / eligible_claim_balance / funding mix`。 |
| restricted grant issuance metadata and expiry / revoke lifecycle | 无。当前 runtime 只有 `restricted_starter_claim_balance` 数值 bucket，没有 `issuance_reason / issuer_id / expires_at_epoch` 状态、没有 grant issue/expire/revoke 事件、也没有对应自动化验证入口 | 无 | `block`。`PRD-GAME-011` 要求的 allowlist / QA seed 发放、过期、回收与 issuer audit 还未实现，QA 无法给出放行结论。 |
| restricted grant economic audit linkage | 无。当前 claim activation / upkeep / refund 已进入审计链，但 restricted grant 的发放/过期/回收本身没有事件或报表字段 | 无 | `block`。还不能证明 restricted grant source/sink 已 100% 进入 main token 源汇审计链。 |

## 本轮执行命令
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_accounts_endpoint_exposes_restricted_balance_separately -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture
```

## 结论
- `TASK-GAME-047` / `TASK-GAMEPLAY-AGC-009` 的 QA 矩阵已经建立完成。
- 当前 restricted starter claim 闭环的 claim/upkeep/refund/transfer guard/viewer parity 可判定为 `pass`。
- 但专题整体结论仍然是 `block`：
  - 还没有正式 `allowlist / qa_seed / liveops_campaign` 发放模型。
  - 还没有 `issuance_reason / issuer_id / expires_at_epoch` 持久化状态与事件。
  - 还没有 restricted grant 的过期/回收和经济审计证据。
- 下一步应回到 `producer_system_designer` 执行 `TASK-GAME-048`，决定是收敛 PRD 范围，还是重新打开 runtime/liveops 任务补齐 grant lifecycle。
- 追记（2026-03-29，同日后续）:
  - 上述 blocker 已在 `TASK-GAME-049/050/051` 中被逐步补齐；正式 successor evidence 见 `doc/testing/evidence/game-agent-claim-restricted-grant-lifecycle-matrix-2026-03-29.md`。
