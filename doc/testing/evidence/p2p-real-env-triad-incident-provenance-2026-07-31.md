# P2P Real-Environment Triad Incident Provenance (2026-07-31)

## Status and authority boundary

This document consolidates the 2026-04-07--08 real-environment triad incident
and rollout chain. It is historical provenance only: it records what was
observed and changed in that period, and is not current readiness, operator,
deployment, or protocol authority. Its terminal historical disposition remains
`blocked`; it does not establish a shared-network pass, full mixed-topology
truth, or a currently healthy triad.

For current readiness and operator requirements, use the formal network-tier
documents and [testing manual](../../../testing-manual.md), including its S9
P2P gates. The current authority links are
[`node-triad-operations-observability.prd.md`](../../p2p/node/node-triad-operations-observability.prd.md),
[`public-testnet-governed-bootstrap.runbook.md`](../../p2p/blockchain/public-testnet-governed-bootstrap.runbook.md),
[`environment-lanes-and-inventory-2026-05-29.md`](../../engineering/governance/environment-lanes-and-inventory-2026-05-29.md),
and the July [required lanes](public-testnet-current-required-lanes-2026-07-03.md)
and [claims-boundary evidence](public-testnet-claims-boundary-review-2026-07-06.md).
Runtime behaviour remains authoritative in the implementation and its current
tests; this record preserves the incident evidence that motivated the
historical changes.

## Absorbed records and immutable locators

The absorbed source directory is `doc/testing/evidence` (each name below is
relative to that directory and is deliberately no longer a live Markdown link).

| Ordered record | Absorbed source path | Terminal commit locator | Historical conclusion retained |
| --- | --- | --- | --- |
| 1 | `p2p-real-env-triad-snapshot-2026-04-07.md` | `b70e4b03f8f7a23e892ef24cf3845ad86e2c511d` (`qa: record real env triad baseline`) | First audited `partial_with_observer_blocker` baseline. |
| 2 | `p2p-real-env-observer-gap-sync-followup-2026-04-08.md` | `83b3759addc568f0c4d47586d25643f088d61e19` (`Fix P2PARCH-6 gap sync blob provider routing`) | Observer had crossed the peer-head-zero condition, but was blocked on gap-sync blob availability. |
| 3 | `p2p-real-env-triad-reconfirm-2026-04-08.md` | `f8b1baf97316697027392194637df42b2b668d9b` (`Reconfirm P2PARCH-6 real-env triad blocker`) | Credentialed same-window reconfirmation isolated stale execution state at the sequencer. |
| 4 | `p2p-real-env-triad-stale-height-rollout-2026-04-08.md` | `95ae1e3ee60462f35181cc64a45918328f3f4dd2` (`Document triad stale-height rollout follow-up`) | The stale-height signature disappeared after the cloud rollout, exposing the storage challenge/blob residual. |
| 5 | `p2p-real-env-triad-blob-availability-root-cause-2026-04-08.md` | `2e5415cdb99d2658adb643db73c02e9960ab39ce` (`Clarify fetch-commit retry cooldown scope`) | Root cause, corrective rollouts, and the terminal `fetch-commit`/convergence residual. |

The original names and terminal commit locators above are retained so archived
SIG-PM-0020 provenance remains interpretable; the immutable signal archive is
not rewritten by this consolidation.

## Environment and evidence limits

- Topology: one local `triad-observer-local` observer plus two Alibaba Cloud
  ECS nodes, `triad-sequencer-a` and `triad-storage-b`; the observer was not in
  the validator set. This was neither a dedicated sentry/NAT lab nor complete
  NAT/CGNAT coverage.
- A local-only sample without the remote ECS credentials was not a same-window
  triad conclusion. Its `cloud_pair_service_unhealthy` /
  `cloud_pair_chain_not_visible` output meant remote state was not collected,
  not that the cloud pair was proven unhealthy.
- The credentialed snapshots used
  `.tmp/p2p_real_env_triad/20260408-120134/summary.json` and
  `.tmp/p2p_real_env_triad/20260408-132008/summary.json`; these are historical
  run references, not live endpoint evidence.

## Ordered incident transition

1. The April 7 baseline observed a live cloud side but an attached observer
   with `observer_known_peer_heads_zero`,
   `observer_network_committed_height_zero`, and
   `observer_committed_height_not_advancing`. It established an audited
   `partial`, not a pass.
2. The local observer follow-up showed `known_peer_heads=1` and
   `network_committed_height>0`, replacing the old observer-peering signature
   with `gap sync ... blob not found`. Historical gap-sync routing then preferred
   DHT blob providers and fell back to the ordinary lane-aware request when the
   provider route was unavailable or returned `NetworkProtocolUnavailable`.
3. Credentialed reconfirmation showed the observer and storage progressing,
   while the sequencer remained at zero heights with
   `sequencer_committed_height_zero` and
   `execution driver received stale height: context=57536 state=57560`.
   The execution bridge recovery from an exact execution record was added and
   historically covered by
   `node_runtime_execution_driver_reconciles_stale_state_from_exact_record`.
4. The `f8b1baf97316` rollout deployed a fresh
   `oasis7_chain_runtime` with sha256
   `72a6008f24b85e3b8e223db2e141688c2d10cd58cff578c1550e2028796d7aa7`
   to the two ECS nodes. The stale-height signature did not recur, but the
   sequencer exposed `storage challenge gate network threshold unmet` and the
   observer still reported gap-sync `blob not found`; the triad remained
   `blocked`.
5. The root-cause investigation found that consensus `committed_height` could
   advance while the storage replication root remained empty. Reusing that
   height as the replication apply/gap-sync cursor prevented historical
   commit/blob backfill; the sequencer's challenge gate then stopped publishing,
   creating a deadlock. The historical correction introduced a separate
   `replication_persisted_height`, provider-route fallback, and a cold-start
   challenge-gate fallback to older available samples.
6. Subsequent historical rollouts recorded
   `95ae1e3ee604-blob-route-fallback-20260408`
   (`0179d52afb91355821dcfbeb94c83c7bb10eb174fe1d81d41fbf16d27b26329a`),
   `95ae1e3ee604-challenge-gate-fallback-20260408`
   (`a2cb5191cdb58cfa0b430369e0220666b5d18e22f4cf58b5b0d1a220f1370fea`),
   `95ae1e3ee604-inbound-endpoint-route-sanitize-20260408`
   (`b84b551e087a1e2b47dde4d8d62a71fc0100cc3980d94379e633d1c53657a6e6`),
   and `95ae1e3ee604-unsupported-peer-no-fallback-20260408`
   (`98b1b99878ba271af63ba4d5e72be1d6a42073e84a383d6d2012a30fb2e3c2de`).
   They recorded observer recovery and storage backfill, while progressively
   narrowing the remaining failure from challenge-gate/blob errors to
   `libp2p-replication no connected peers for protocol /aw/node/replication/fetch-commit/1.0.0`.

## Historical regression evidence

The following tests were reported as passed in the absorbed records, as
historical evidence rather than a fresh validation claim:

- `node_runtime_execution_driver_reconciles_stale_state_from_exact_record`
- `runtime_network_replication_gap_sync_prefers_dht_blob_providers`
- `runtime_network_replication_gap_sync_falls_back_after_provider_route_unavailable`
- `replication_gap_sync_backfills_when_consensus_height_already_advanced`
- `runtime_network_replication_gap_sync_falls_back_after_provider_route_not_found`
- `runtime_replication_storage_challenge_gate_falls_back_after_provider_route_not_found`
- `runtime_replication_storage_challenge_gate_falls_back_after_provider_route_unavailable`
- `runtime_replication_storage_challenge_gate_allows_when_network_matches_reach_threshold`
- `runtime_replication_storage_challenge_gate_falls_back_to_older_samples_during_catchup`

## Terminal historical disposition

The final April record remained `blocked`. Observer progress and the removal of
the stale-height/challenge-gate signatures did not prove complete convergence:
storage could still encounter `fetch-commit NetworkProtocolUnavailable` and,
after unsupported observer fallback was removed, the more accurate remaining
signature was no connected healthy `fetch-commit` source during startup/retry
windows. The unresolved boundary was fetch-commit source readiness, peer-head
reconvergence, and retry ordering after storage restart. Any present-day
assessment must collect new same-window evidence under the current gate rather
than inherit this historical result.
