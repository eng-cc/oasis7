# p2p public testnet faucet service evidence (2026-05-19)

## Summary
- Verdict: `pass` for the task scope of replacing placeholder `faucet_ref` with a real guarded public-testnet faucet on existing ECS infrastructure.
- Live faucet endpoint: `http://39.104.204.172:6681/`
- Backing RPC endpoint: `http://39.104.204.172:6631/v1/chain/status`
- Explorer endpoint used for verification: `http://39.104.205.67:6632/v1/chain/explorer/overview`
- Faucet policy deployed on the live host:
  - `amount = 1000000`
  - `cooldown_secs = 3600`
  - target account format must be `oc:pk:<64-hex>`

## Topology
- `39.104.204.172`
  - `oasis7-testnet-sequencer.service`
  - `oasis7-testnet-faucet.service`
  - public status `:6631`
  - public faucet `:6681`
- `39.104.205.67`
  - `oasis7-testnet-storage.service`
  - public status `:6632`

## Root Cause Closed
- Chain liveness unblock:
  - world finality registry signer bindings were imported as `<slot_id>.<signer_id>`, while local validator ids were plain node ids; `governance_registry.rs` now strips the slot prefix before mapping world finality bindings back to validator ids.
- Faucet transfer unblock:
  - `PosNodeEngine::propose_next_head()` was draining `pending_consensus_actions` before checking whether the local node was the proposer for that slot.
  - Result before fix: a transfer submitted during a non-local proposer slot was silently dropped and later surfaced as faucet transfer `timeout`.
  - Fix: only drain the queue after confirming the local node is the expected proposer.

## Deployment Steps
1. Built and deployed patched `oasis7_chain_runtime` and `oasis7_testnet_faucet`.
2. Installed sequencer-side systemd unit `oasis7-testnet-faucet.service` on `39.104.204.172`.
3. Performed a coordinated cold reset of both ECS nodes because the tier is explicitly `resettable public_testnet`.
4. Re-imported `/opt/oasis7/p2p-testnet/config/governance-public-manifest-public-testnet-2validator.json` into both fresh `execution-world` directories with `oasis7_governance_registry_import`.
5. Re-submitted main-token genesis bucket `public_testnet_faucet_genesis`.
6. Claimed the faucet vesting bucket into the faucet hot wallet.
7. Restarted faucet service and verified an external claim end-to-end.

## External Verification
### Faucet info
`GET http://39.104.204.172:6681/`

Observed payload:

```json
{
  "ok": true,
  "faucet_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "amount": 1000000,
  "cooldown_secs": 3600,
  "claim_path": "/claim"
}
```

### Health check
`GET http://39.104.204.172:6681/healthz`

Observed payload:

```json
{
  "ok": true
}
```

### Real claim
- Request:
  - `POST http://39.104.204.172:6681/claim`
  - body: `{"account_id":"oc:pk:2222222222222222222222222222222222222222222222222222222222222222"}`
- Immediate faucet response:

```json
{
  "ok": true,
  "faucet_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "amount": 1000000,
  "cooldown_secs": 3600,
  "action_id": 1
}
```

- Confirmed on-chain afterward:
  - faucet account `oc:pk:14699e...fa7d` liquid balance: `9999000000`
  - target account `oc:pk:2222...2222` liquid balance: `1000000`
  - explorer address record status: `confirmed`

Observed `transfer/accounts` snapshot:

```json
{
  "accounts": [
    {
      "account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
      "liquid_balance": 9999000000,
      "last_transfer_nonce": 1,
      "next_nonce_hint": 2
    },
    {
      "account_id": "oc:pk:2222222222222222222222222222222222222222222222222222222222222222",
      "liquid_balance": 1000000,
      "next_nonce_hint": 1
    }
  ]
}
```

## Operational Notes
- `/v1/chain/balances` is not a faucet-wallet truth surface here because it only reports `node_main_token_account` bindings for the local node id; the faucet hot wallet is not exposed through that binding.
- Use these surfaces for faucet truth:
  - `/v1/chain/transfer/accounts`
  - `/v1/chain/explorer/address`
  - `data/execution-world/snapshot.json`
- Backup directories created during this rollout include:
  - sequencer: `/opt/oasis7/p2p-testnet/backups/coordinated-reset-20260519-172636`
  - storage: `/opt/oasis7/p2p-testnet/backups/coordinated-reset-20260519-172637`

## Acceptance Mapping
- Real `faucet_ref` backed by working service: `pass`
- Canonical liveops grant entrypoint reused with abuse-control boundary (`amount/cooldown/account-format`): `pass`
- Runtime evidence and readiness verdict updated after external verification: `pass`
