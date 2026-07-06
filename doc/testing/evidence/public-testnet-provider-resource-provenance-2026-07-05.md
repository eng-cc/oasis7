# Public Testnet Provider Resource Provenance Evidence (2026-07-05)

## Meta
- Task: GitHub issue #1964 / `task_643302fe9bf7404fb9bd9dcf7f8985ed`
- Lane: `provider_resource_provenance_ready`
- Owner role: `agent_engineer`
- Integration role: `tpm`
- Verdict: `pass`

This packet verifies the provider resource schema/provenance contract against
the current governed `public_testnet` live world-resource surface. It does not
claim real LetAI upstream billing, quota availability, or production provider
capacity.

## Live World Resource Sample
Source:
- `GET http://39.104.204.172:6631/v1/chain/status`

Observed fields:

```json
{
  "node_id": "triad-testnet-sequencer",
  "height": 71,
  "last_error": null,
  "world_resource": {
    "schema_version": "oasis7.world_resource_manifest.v1",
    "delta_schema_version": "oasis7.world_resource_delta.v1",
    "world_id": "oasis7-public-testnet-governed-20260606",
    "chain_id": "oasis7-public-testnet-governed-20260606",
    "latest_resource_commit_height": 71,
    "latest_resource_commit_hash_present": true,
    "readiness_status": "ready",
    "failed_gates": []
  }
}
```

## Provider Sample
Source:
- `target/debug/oasis7_provider_local_bridge --bind 127.0.0.1:5842 --mock`
- `GET http://127.0.0.1:5842/v1/provider/info`
- `GET http://127.0.0.1:5842/v1/provider/health`

Provider info:

```json
{
  "provider_id": "provider_local_mock",
  "name": "Local Mock Provider Bridge",
  "version": "0.1.0",
  "protocol_version": "world-simulator-provider-loopback-http-v1",
  "capabilities": [
    "decision",
    "feedback",
    "agent_chat",
    "loopback_only",
    "mock",
    "deterministic",
    "no_billing",
    "no_quota"
  ],
  "supported_action_sets": [
    "wait",
    "wait_ticks",
    "move_agent",
    "speak_to_nearby",
    "inspect_target",
    "simple_interact"
  ],
  "chain_resource_manifest_schema_version": "oasis7.world_resource_manifest.v1",
  "chain_resource_delta_schema_version": "oasis7.world_resource_delta.v1"
}
```

Provider health:

```json
{
  "ok": true,
  "status": "ready",
  "queue_depth": 0
}
```

## Compatibility Result
Local compatibility report:

```json
{
  "pass": true,
  "provider_id": "provider_local_mock",
  "provider_health_ok": true,
  "provider_health_status": "ready",
  "manifest_schema_matches": true,
  "delta_schema_matches": true,
  "missing_capabilities": [],
  "missing_supported_actions": [],
  "live_chain_id": "oasis7-public-testnet-governed-20260606",
  "live_world_id": "oasis7-public-testnet-governed-20260606",
  "live_resource_commit_height": 71,
  "live_resource_commit_hash_present": true,
  "world_resource_ready": "ready",
  "world_resource_failed_gates": []
}
```

The provider reports the same world-resource manifest and delta schema versions
as the live governed public-testnet world-resource status. It also exposes the
required provider capabilities and supported action set for phase-1 provider
compatibility.

## Boundary
Allowed:
- `provider_resource_provenance_ready=pass`
- provider resource manifest/delta schema compatibility is evidenced for the
  current governed `public_testnet` world-resource contract

Still denied:
- real LetAI upstream readiness
- provider billing/quota availability
- production provider capacity
- `ready_for_live_candidate`
- `mainnet-grade`

Residual risk:
- This is a deterministic local provider bridge probe, not a remote HTTPS
  provider availability or quota test.
- Provider action execution against a hosted same-world entry remains covered by
  the separate `same_world_hosted_entry_ready` and API/viewer lanes.
