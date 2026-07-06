# Public Testnet Faucet Guard Ready Evidence (2026-07-05)

## Meta
- Task: GitHub issue #1964 / `task_643302fe9bf7404fb9bd9dcf7f8985ed`
- Lane: `faucet_guard_ready`
- Owner role: `liveops_community`
- Integration role: `tpm`
- Endpoint: `http://39.104.204.172:6681/`
- Verdict: `pass`

This evidence restores the governed `public_testnet` guarded faucet lane after
the 2026-07-05 run142 clean validator rebuild. It does not promote
`public_testnet` to `ready_for_live_candidate`.

## Deployment
- Host: `39.104.204.172`
- Service: `oasis7-public-testnet-faucet.service`
- Service state: `active` / `enabled`
- Release: `/opt/oasis7/public-testnet-faucet/releases/20260705T160633Z-run142-faucet`
- Binary SHA-256: `b7dcf3958ef1e9183b8e8088df04629826185437d6f13a9aa0af6935871a464e`
- Faucet account: `oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d`
- Upstream: `http://127.0.0.1:6631`
- Amount: `1000000`
- Cooldown: `3600` seconds

Secret boundary:
- Public key file mode: `0640 oasis7-testnet:oasis7-testnet`
- Private key file mode: `0600 oasis7-testnet:oasis7-testnet`
- Private key path and contents are intentionally not recorded.
- SSH credentials, signer private keys, and raw signed payloads are not part of
  this evidence packet.

## Token Funding
The run142 validator rebuild left `main_token` uninitialized, so the faucet
hot wallet first had to be funded under the governed public-testnet controller
policy.

Actions:
- Submitted `InitializeMainTokenGenesis` for bucket
  `public_testnet_faucet_genesis` with `threshold=2` using the frozen
  `msig.genesis.v1` testnet governance signer set.
- Recipient: `oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d`
- Ratio: `10000` bps.
- Submitted `ClaimMainTokenVesting` for the same bucket and beneficiary.

Post-funding account state:

```json
{
  "account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "liquid_balance": 10000000000,
  "vested_balance": 0,
  "restricted_starter_claim_balance": 0,
  "next_nonce_hint": 1
}
```

## Public Endpoint Evidence
`GET http://39.104.204.172:6681/`

```json
{
  "ok": true,
  "faucet_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "amount": 1000000,
  "cooldown_secs": 3600,
  "claim_path": "/claim"
}
```

`GET http://39.104.204.172:6681/healthz`

```json
{
  "ok": true
}
```

## Guarded Claim Evidence
Fresh claim request:

```json
{
  "account_id": "oc:pk:3333333333333333333333333333333333333333333333333333333333333333"
}
```

Immediate faucet response:

```json
{
  "ok": true,
  "faucet_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "amount": 1000000,
  "cooldown_secs": 3600,
  "action_id": 1
}
```

Repeated claim response:

```json
{
  "ok": false,
  "faucet_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
  "amount": 1000000,
  "cooldown_secs": 3600,
  "error_code": "cooldown_active"
}
```

## Chain Confirmation
`GET /v1/chain/transfer/status?action_id=1`

```json
{
  "ok": true,
  "action_id": 1,
  "status": {
    "from_account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
    "to_account_id": "oc:pk:3333333333333333333333333333333333333333333333333333333333333333",
    "amount": 1000000,
    "nonce": 1,
    "status": "confirmed"
  }
}
```

`GET /v1/chain/transfer/accounts`

```json
{
  "accounts": [
    {
      "account_id": "oc:pk:14699ee340994e43103490585a96671ec66a3280bc0f90518f29cd1866f0fa7d",
      "liquid_balance": 9999000000,
      "vested_balance": 0,
      "restricted_starter_claim_balance": 0,
      "last_transfer_nonce": 1,
      "next_nonce_hint": 2
    },
    {
      "account_id": "oc:pk:3333333333333333333333333333333333333333333333333333333333333333",
      "liquid_balance": 1000000,
      "vested_balance": 0,
      "restricted_starter_claim_balance": 0,
      "next_nonce_hint": 1
    }
  ]
}
```

## Claim Boundary
Allowed:
- `faucet_guard_ready=pass`
- `guarded/resettable public_testnet faucet lane passed`
- `faucet amount is 1000000 with process-memory cooldown of 3600 seconds`

Still denied:
- `ready_for_live_candidate`
- `mainnet-grade`
- `mainnet_live`
- `production OC settlement`
- `public validator admission is open`
- durable anti-abuse, WAF, TLS, or persistent cooldown guarantees

Residual risk:
- Cooldown state is in process memory and resets on service restart.
- The service is plain HTTP and does not by itself provide TLS, WAF, or durable
  public abuse-control storage.
