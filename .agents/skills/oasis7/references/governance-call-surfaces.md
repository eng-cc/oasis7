# Governance Call Surfaces

This booklet exists to answer one narrow operator/product question: in the current `oasis7` Local Provider path, which governance-adjacent actions already have direct callable chain-runtime endpoints, what the real first-agent-claim approval loop is, and where final chain truth is read back.

## Product Chain

Read the active product chain in five layers:

1. `oasis7-run.sh play` or bundle `run-game.sh` launches `oasis7_game_launcher`; unless you explicitly pass `--chain-disable`, the default product path also starts `oasis7_chain_runtime`.
2. `oasis7_viewer_live` remains the player-facing control plane and snapshot surface. Player UI state should be read back from snapshot, not inferred from a button click.
3. Direct operator/player chain-runtime calls can now hit dedicated agent-claim governance endpoints under `/v1/chain/agent-claim/**`.
4. Each submit endpoint first preflights against the current execution world, then enqueues a consensus action into `oasis7_chain_runtime`.
5. Final truth is the committed world snapshot. The player should confirm approval/claim progress via `player_gameplay.agent_claim` fields after commit, not only via the HTTP `ok=true` response.

## Directly Callable Agent-Claim Governance Surfaces

Current dedicated endpoints:

- `POST /v1/chain/agent-claim/approval-request/submit`
- `GET /v1/chain/agent-claim/approval-requests`
- `POST /v1/chain/agent-claim/approval-request/approve`
- `POST /v1/chain/agent-claim/approval-request/reject`
- `POST /v1/chain/agent-claim/submit`

These are repo-owned control-plane APIs on `oasis7_chain_runtime`.
They are direct callable, but they are not yet documented as public internet-facing auth-hardened APIs.

### 1. Player submits the first-claim approval request

Use when a fresh account wants its slot-1 bootstrap reviewed:

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/submit \
  -H 'Content-Type: application/json' \
  -d '{"claimer_agent_id":"agent-0"}'
```

Current request body:

```json
{
  "claimer_agent_id": "agent-0"
}
```

Current preflight boundary:

- `claimer_agent_id` must exist
- the requester must still be on slot 1
- an active restricted starter claim grant/balance blocks resubmission
- only one `pending` approval request is allowed at a time

On success, runtime records `FirstAgentClaimApprovalRequested` in world state and returns a preview containing the new `request_id`.

### 2. Operator lists pending approvals

Use this as the operator queue:

```bash
curl -sS 'http://127.0.0.1:8765/v1/chain/agent-claim/approval-requests?status=pending'
```

Optional filters:

- `status=pending|approved|rejected`
- `claimer_agent_id=<agent-id>`

Returned items are read from committed world state and include:

- `request_id`
- `claimer_agent_id`
- `status`
- `requested_total_upfront_amount`
- `operator_account_id`
- `approved_amount`
- `expires_at_epoch`
- `rejection_reason`

### 3. Operator approves or rejects

Approve:

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/approve \
  -H 'Content-Type: application/json' \
  -d '{"operator_account_id":"liveops","request_id":1,"expires_at_epoch":10}'
```

Reject:

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/approval-request/reject \
  -H 'Content-Type: application/json' \
  -d '{"operator_account_id":"liveops","request_id":1,"reason":"manual_review_failed"}'
```

Current approval boundary:

- `operator_account_id` must be allowlisted in `restricted_starter_claim_admin_account_ids`
- the request must still be `pending`
- `expires_at_epoch` must be valid
- approval issues the restricted slot-1 bootstrap grant in the same runtime event flow

This means approval is not “docs-only”. It mutates runtime state and treasury-backed restricted balance.

### 4. Player submits the actual slot-1 claim

After approval has committed and snapshot shows the grant/balance, the player can directly call:

```bash
curl -sS http://127.0.0.1:8765/v1/chain/agent-claim/submit \
  -H 'Content-Type: application/json' \
  -d '{"claimer_agent_id":"agent-0","target_agent_id":"agent-1"}'
```

Current request body:

```json
{
  "claimer_agent_id": "agent-0",
  "target_agent_id": "agent-1"
}
```

Runtime rules still apply:

- slot 1 can consume `restricted_starter_claim_balance`
- slot 2/3 still require liquid balance only
- release cooldown, upkeep, idle reclaim, and bond accounting remain runtime-owned rules

## Player-Visible Readback

The player-authority snapshot now exposes the latest first-claim approval request under:

- `player_gameplay.agent_claim.first_agent_claim_approval_request`

Related player-observable fields remain:

- `player_gameplay.agent_claim.next_claim_quote`
- `player_gameplay.agent_claim.restricted_starter_claim_balance`
- `player_gameplay.agent_claim.slot_1_eligible_claim_balance`
- `player_gameplay.agent_claim.owned_claims`

Use this as the real “button clicked, what happened next” readback path:

1. submit approval request
2. wait for commit
3. read snapshot for `first_agent_claim_approval_request.status`
4. if `approved`, confirm restricted balance / eligible balance
5. submit `agent-claim/submit`
6. wait for commit
7. read snapshot for `owned_claims`

## Operational Boundary

These new endpoints make the review loop directly operable, but the trust model is still important:

- the HTTP response only means “accepted into chain-runtime consensus queue after preflight”
- committed snapshot remains the final source of truth
- these endpoints are currently internal/direct-call control surfaces, not yet auth-hardened public APIs

## Source Anchors

- `crates/oasis7/src/bin/oasis7_chain_runtime/agent_claim_api.rs`
- `crates/oasis7/src/viewer/runtime_live/claim_snapshot.rs`
- `crates/oasis7/src/simulator/persist.rs`
- `crates/oasis7/src/runtime/tests/agent_claims.rs`
- `crates/oasis7/src/viewer/runtime_live/tests/snapshot_progress.rs`
