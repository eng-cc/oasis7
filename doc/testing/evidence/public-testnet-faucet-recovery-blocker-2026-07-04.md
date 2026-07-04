# Public Testnet Faucet Recovery Blocker (2026-07-04)

## Meta
- task: GitHub issue #1868 / `task_17e9c8cd02014f938b29689a44cbfb89`
- lane: `faucet_guard_ready`
- owner_role: `liveops_community`
- integration_role: `tpm`
- endpoint: `http://39.104.204.172:6681/`
- verdict: `block`

## Fresh Recheck
Command:

```bash
curl --connect-timeout 8 --max-time 20 -sS -w '%{http_code}' \
  -o .tmp/public-testnet-faucet-recheck-20260704/faucet.body \
  http://39.104.204.172:6681/
```

Observed result:

```text
curl: (7) Failed to connect to 39.104.204.172 port 6681 after 173 ms: Couldn't connect to server
http_code=000
started_at=2026-07-04T05:17:04Z
ended_at=2026-07-04T05:17:04Z
```

The public faucet endpoint is still unreachable from the current execution environment. This evidence keeps `faucet_guard_ready=block`.

Second fresh sample:

```bash
curl --connect-timeout 8 --max-time 20 -sS -w '%{http_code}' \
  -o .tmp/public-testnet-faucet-recheck-20260704-2/root.body \
  http://39.104.204.172:6681/

curl --connect-timeout 8 --max-time 20 -sS -w '%{http_code}' \
  -o .tmp/public-testnet-faucet-recheck-20260704-2/healthz.body \
  http://39.104.204.172:6681/healthz
```

Observed result:

```text
root:
  started_at=2026-07-04T05:31:33Z
  ended_at=2026-07-04T05:31:33Z
  http_code=000
  curl_status=7
  stderr=curl: (7) Failed to connect to 39.104.204.172 port 6681 after 19 ms: Couldn't connect to server

healthz:
  started_at=2026-07-04T05:31:33Z
  ended_at=2026-07-04T05:31:33Z
  http_code=000
  curl_status=7
  stderr=curl: (7) Failed to connect to 39.104.204.172 port 6681 after 22 ms: Couldn't connect to server
```

## Repo-Owned Recovery Added
This task adds a reusable operator recovery surface:

- `scripts/public-testnet-faucet/start-public-testnet-faucet.sh`
- `scripts/public-testnet-faucet/package-public-testnet-faucet.sh`
- `scripts/public-testnet-faucet/oasis7-public-testnet-faucet.service`
- `scripts/public-testnet-faucet/public-testnet-faucet.env.example`
- `doc/p2p/blockchain/p2p-public-testnet-faucet-operator-runbook-2026-07-04.md`

The package/runbook uses the existing `oasis7_testnet_faucet serve` contract and preserves the prior guarded faucet boundary:

- public port: `6681`
- expected upstream env value: `http://127.0.0.1:6631`
- amount default: `1000000`
- cooldown default: `3600` seconds
- key material read from files outside the release directory
- startup guard validates key files are readable and non-empty

Known guard boundary:

- `/healthz` proves the faucet process is alive, not that upstream claim submission is viable.
- cooldown/account/IP tracking is in process memory, so a restart clears cooldown state.
- stronger public-surface controls such as TLS, proxy rate limiting, WAF, and durable audit logs must be supplied outside the current faucet binary if required for a later public readiness standard.

## Lane Impact
Status remains:

```text
faucet_guard_ready	block
```

This task closes the missing repo-owned recovery path, not the live readiness lane.

## Pass Criteria For A Later Round
`faucet_guard_ready` may move to `pass` only after fresh evidence proves:

1. `GET http://39.104.204.172:6681/` returns the expected faucet metadata.
2. `GET http://39.104.204.172:6681/healthz` succeeds.
3. `POST http://39.104.204.172:6681/claim` succeeds for a fresh `oc:pk:<64-hex>` test account.
4. Repeated claim or documented service response proves cooldown/account/IP guard behavior.
5. Upstream/explorer/chain-state evidence shows the claim was committed or the faucet account state changed as expected.
6. Evidence records amount, cooldown, upstream boundary, and service identity without exposing private key material.

## Claim Boundary
Allowed:

- `repo-owned faucet recovery package and runbook now exist`
- `fresh faucet endpoint check remains blocked`
- `public_testnet aggregate readiness remains block`

Denied:

- `public faucet is open`
- `faucet_guard_ready is pass`
- `ready_for_live_candidate`
