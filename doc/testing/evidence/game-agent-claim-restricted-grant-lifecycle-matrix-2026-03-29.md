# Game Agent Claim Restricted Grant Lifecycle Matrix (2026-03-29)

审计轮次: 1

## Meta
- Owner Role: `qa_engineer`
- Review Role: `producer_system_designer`
- Scope: `TASK-GAME-051` / `TASK-GAMEPLAY-AGC-013` for `PRD-GAME-011`
- Topic: `doc/game/gameplay/gameplay-agent-claim-token-cost-2026-03-27.project.md`
- Upstream Runtime Closure: `TASK-GAME-049`
- Upstream LiveOps Closure: `TASK-GAME-050`

## 目标
- 把 restricted grant 的 lifecycle / audit 守门面收敛成一份可复跑矩阵。
- 验证 runtime 已实现的 `issuance metadata`、`expiry/revoke`、`treasury source-sink`、`terminal refund sink redirect` 与 `transfer non-bypass` 都有 fresh 真值。
- 给出 `PRD-GAME-011` 在 restricted grant 维度上的正式 QA verdict，而不再停留在“仅 bucket 账本正确”的中间状态。

## 本轮判定口径
- `required`:
  - 以 `runtime::tests::agent_claims`、`viewer::runtime_live::tests::compat_snapshot_*` 与 `oasis7_chain_runtime` 的 transfer/explorer 定向用例为真值。
  - 目标是冻结 grant issue metadata、expiry/revoke 行为、source-sink 审计链与公开资产面的 non-bypass。
- `full`:
  - 继续要求 `runtime::tests::agent_claims` 在 `test_tier_full` 入口下通过，确认 feature 组合不会改变 grant lifecycle、refund sink 与 claim 账本语义。

## Evidence Matrix
| 类别 | Required Evidence | Full Evidence | 当前结论 |
| --- | --- | --- | --- |
| issuance metadata persistence | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture`，其中 `restricted_grant_issue_records_metadata_and_moves_treasury_to_restricted_balance = ok` | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture`，同名用例再次 `ok` | `pass`。`issuer_id / issuance_reason / expires_at_epoch / spend_scope` 都会持久化到 grant state，并同步写入 canonical event。 |
| issue source-sink treasury linkage | 同一轮 required 命令，其中 `restricted_grant_issue_records_metadata_and_moves_treasury_to_restricted_balance = ok`；断言 ecosystem treasury 减少、beneficiary restricted bucket 增加、circulating supply 只按 issued amount 增长 | 同一轮 full 命令，同名用例再次 `ok` | `pass`。grant 发放已经进入 `MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL -> beneficiary restricted bucket` 的 canonical source-sink 链。 |
| expiry lifecycle and treasury refund | 同一轮 required 命令，其中 `expired_restricted_grant_returns_remaining_balance_and_redirects_release_refund_to_treasury = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。grant 到达配置的 `expires_at_epoch` 后会进入 `Expired`，剩余 spendable restricted 会回 treasury，之后 release refund 继续定向回 source treasury bucket。 |
| revoke lifecycle and issuer-scope refund | 同一轮 required 命令，其中 `revoked_restricted_grant_returns_spendable_balance_and_redirects_release_refund_to_treasury = ok` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。撤销后 grant 状态变为 `Revoked`，`status_reason` 保留 revoke reason，剩余 spendable restricted 会回 treasury，后续 restricted bond refund 不再返还 beneficiary。 |
| terminal refund sink redirect | 同一轮 required 命令，其中 `expired_restricted_grant_returns_remaining_balance_and_redirects_release_refund_to_treasury = ok`、`revoked_restricted_grant_returns_spendable_balance_and_redirects_release_refund_to_treasury = ok`；断言 `AgentClaimReleased.refunded_bond_restricted_sink = SourceTreasuryBucket` | 同一轮 full 命令，同名用例再次 `ok` | `pass`。grant 终态后的 restricted bond refund 会显式写成 treasury sink，不存在“先 revoke/expire 再通过 release 洗回 beneficiary restricted bucket”的旁路。 |
| viewer / pure API slot eligibility continuity | `env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture`，其中 `compat_snapshot_exposes_player_agent_claim_overview = ok`、`compat_snapshot_flags_restricted_balance_as_ineligible_for_slot_2 = ok` | 当前无单独 full lane；compat snapshot 用例作为 canonical surface 真值 | `pass`。grant lifecycle 补齐后，玩家 surface 仍保持 `restricted/liquid/eligible` 拆分与 `slot-2` blocker，不会因为 grant 引入新旁路。 |
| transfer / explorer non-bypass | `env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_accounts_endpoint_exposes_restricted_balance_separately -- --nocapture`、`env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture` | 当前无单独 full lane；bin 用例直接验证 transfer/explorer surface | `pass`。即使 grant lifecycle 已启用，restricted 余额仍不会被普通转账、transfer accounts 或 explorer 总额语义误当作 transferable liquid。 |
| liveops issuer boundary readiness | `rg -n "issuer_id = liveops|preview_allowlist|qa_seed|liveops_campaign|revoke_reason|Incident Fallback" doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md` | 不适用 | `pass`。v1 issuer/reason/incident 口径已有正式 runbook，可与 runtime 的 `issuer_id`、`issuance_reason` 和 revoke 流程对齐。 |

## 本轮执行命令
```bash
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_required runtime::tests::agent_claims:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib --features test_tier_full runtime::tests::agent_claims:: -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --lib viewer::runtime_live::tests::compat_snapshot_ -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime preflight_transfer_rejects_restricted_only_balance -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime transfer_accounts_endpoint_exposes_restricted_balance_separately -- --nocapture
env -u RUSTC_WRAPPER cargo test -p oasis7 --bin oasis7_chain_runtime explorer_p1_endpoints_return_expected_payloads -- --nocapture
rg -n "issuer_id = liveops|preview_allowlist|qa_seed|liveops_campaign|revoke_reason|Incident Fallback" doc/game/gameplay/gameplay-agent-claim-restricted-grant-liveops-runbook-2026-03-29.md
```

## 结论
- `TASK-GAME-051` / `TASK-GAMEPLAY-AGC-013` 可判定为 `pass`。
- restricted grant 的 lifecycle / audit / non-bypass 现在已经闭环：
  - `issuer_id / issuance_reason / expires_at_epoch` 有持久化状态和 canonical event。
  - issue / expire / revoke 都进入 ecosystem treasury source-sink 审计链。
  - grant 终态后的 restricted bond refund 会显式回 treasury，而不是重回 beneficiary。
  - transfer / explorer / viewer surface 仍保持 restricted 非转账、slot-1 only 的边界。
- `PRD-GAME-011` 的 restricted starter grant blocker 已解除，当前 agent claim 专题整体 QA verdict 从 `block` 升级为 `pass`。
- 历史 blocker 追溯保留在 `doc/testing/evidence/game-agent-claim-restricted-starter-balance-matrix-2026-03-29.md`；本文件是其 grant lifecycle successor evidence。
