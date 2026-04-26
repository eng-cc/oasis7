# Governance Call Surfaces

This booklet exists to answer one narrow operator/product question: in the current `oasis7` Local Provider path, which governance-adjacent actions are directly callable, which ones are only observable, and where the chain boundary actually begins.

## Product Chain

Read the active product chain in four layers:

1. `oasis7-run.sh play` or bundle `run-game.sh` launches `oasis7_game_launcher`; unless you explicitly pass `--chain-disable`, the default product path also starts `oasis7_chain_runtime`.
2. `oasis7_viewer_live` remains the player-authority control surface. Signed writes enter through `ViewerRequest` messages such as `AuthoritativeRecovery`, `AgentChat`, `PromptControl`, and `GameplayAction`.
3. If runtime live has `chain_status_bind`, accepted `GameplayAction` requests are forwarded to chain runtime `POST /v1/chain/gameplay/submit`; without that bind they stay as local runtime actions.
4. Committed chain state is pulled back into runtime live through chain-linked world sync; the player observes final state through snapshot/event updates, not by trusting the HTTP submit response alone.

## Directly Callable Surfaces Today

### 1. Read-only gameplay/governance state

Use the pure API client to read the player gameplay snapshot:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_pure_api_client -- \
  snapshot --player-gameplay-only
```

This is the current operator-facing source of truth for:

- `player_gameplay.agent_claim`
- `next_claim_quote`
- `restricted_starter_claim_balance`
- `slot_1_eligible_claim_balance`
- recent gameplay feedback / blocker details

### 2. Viewer-authority signed write surfaces

The easiest direct-call path is still `oasis7_pure_api_client`, because it signs `PlayerAuthProof` for you before sending the `ViewerRequest`.

Signed chat:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_pure_api_client -- \
  chat \
  --agent-id agent-0 \
  --player-id player-1 \
  --private-key-hex <player-private-key-hex> \
  --message "status report" \
  --with-snapshot
```

Signed gameplay action:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_pure_api_client -- \
  gameplay-action \
  --action-id build_factory_smelter_mk1 \
  --target-agent-id agent-0 \
  --player-id player-1 \
  --private-key-hex <player-private-key-hex> \
  --with-snapshot
```

Signed prompt control:

```bash
env -u RUSTC_WRAPPER cargo run -p oasis7 --bin oasis7_pure_api_client -- \
  prompt-apply \
  --agent-id agent-0 \
  --player-id player-1 \
  --private-key-hex <player-private-key-hex> \
  --system-prompt "prioritize alloy throughput" \
  --with-snapshot
```

Current write-surface split:

- `AuthoritativeRecovery::RegisterSession`: session / player-agent binding
- `AgentChat`: player chat to an already controlled agent
- `PromptControl::{Preview,Apply,Rollback}`: prompt governance on a controlled agent
- `GameplayAction`: industrial gameplay actions only

### 3. Raw chain submit surface

When runtime live is chain-linked, `ViewerRequest::GameplayAction` is forwarded to:

- `POST /v1/chain/gameplay/submit`

Request body shape:

```json
{
  "action_id": "build_factory_smelter_mk1",
  "target_agent_id": "agent-0",
  "player_id": "player-1",
  "public_key": "<player-public-key-hex>",
  "auth": {
    "scheme": "ed25519",
    "player_id": "player-1",
    "public_key": "<player-public-key-hex>",
    "nonce": 42,
    "signature": "<signature-hex>"
  }
}
```

Current validation boundary:

- `auth` is mandatory
- nonce replay is rejected
- `public_key` in request and proof must match
- only the whitelisted industrial action ids in `crates/oasis7/src/viewer/gameplay_actions.rs` are accepted

This means `POST /v1/chain/gameplay/submit` is not a generic governance endpoint. It is a narrow signed submit surface for current gameplay actions.

## Claim Governance Boundary

`agent_claim` already has a player-facing governance model, but the current active operator surfaces are narrower than that model.

What is observable today:

- claim cap / reputation tier
- owned claim count
- next slot quote
- slot-1 eligibility using `restricted_starter_claim_balance`
- owned-claim lifecycle (`claimed_active`, `release_cooldown`, `release_ready`, `upkeep_grace`, `idle_reclaim_candidate`)

What the runtime rules already enforce:

- slot 1 may use `restricted_starter_claim_balance` together with liquid balance
- slot 2 and slot 3 require liquid balance only
- quote / upkeep / bond / idle reclaim semantics are runtime-owned governance rules

What is not exposed today as a generic direct-call surface:

- a standalone `claim_agent` command in `oasis7_pure_api_client`
- a `ViewerRequest::GameplayAction` variant for `claim_agent`
- a raw chain HTTP `claim_agent` endpoint parallel to `/v1/chain/gameplay/submit`

Current documentation rule:

- treat claim governance as observable product state plus dedicated product-flow territory
- do not document `claim_agent` as if it were already a generic public/operator submit API
- if a future dedicated claim endpoint/helper lands, update this booklet and the world-simulator product docs in the same change

## Session And Key Safety

Do not confuse the two signing domains:

- player auth key: used for `PlayerAuthProof` on chat / prompt / gameplay writes
- node key: used by chain runtime / chain profile and must never be pasted into docs, screenshots, git history, or CI logs

If you need to bind player authority before prompt/chat flows, use the session register path first; if you only need to inspect claim readiness, prefer snapshot reads and avoid ad hoc write experiments.

## Source Anchors

- `crates/oasis7_proto/src/viewer.rs`
- `crates/oasis7/src/bin/oasis7_chain_runtime/gameplay_submit_api.rs`
- `crates/oasis7/src/viewer/gameplay_actions.rs`
- `crates/oasis7/src/viewer/runtime_live/{player_gameplay.rs,chain_link.rs}`
- `crates/oasis7/src/runtime/agent_claims.rs`
- `crates/oasis7/src/simulator/persist.rs`
- `crates/oasis7/src/bin/oasis7_pure_api_client.rs`
