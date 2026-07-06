# Oasis7 Current Mainstream Public-Chain Gap Benchmark

- Task: GitHub issue #1842 / `task_16ff5f37bfb94220a6349fb4c9304bcc`
- Date: 2026-07-03
- Owner: `tpm` integration
- Professional slices: `runtime_engineer`, `blockchain_ops_engineer`, `producer_system_designer`, `qa_engineer`, `liveops_community`
- Status: current-state research / no release promotion

## Executive Verdict

After the 2026-07-06 proof-window continuity slice, Oasis7 should be assessed as:

`crypto-hardened preview with externally verifiable sampled and continuous world-head evidence`

After the 2026-07-06 public-testnet 11-lane verification refresh, Oasis7 also has:

`controlled formal public_testnet live-candidate readiness evidence`

After the 2026-07-06 validator-finality proof semantics slice, Oasis7 also has:

`bounded validator-set finality and fork-misbehavior evidence semantics anchored to committed world heads`

It should not be assessed as:

`live public testnet already online`, `mainstream public-chain-ready`, `full light client`, `mainnet-grade`, or `multi-client consensus equivalent`.

The main changes since the previous benchmark are that Oasis7 now has concrete external verifier lanes:

- `WorldHeadProofV1` proof artifacts exist for committed world-head evidence.
- `oasis7_world_head_proof_verify` can validate sampled proof artifacts outside the node process.
- `WorldStateReceiptProofV1` now defines a bounded resource/query/receipt proof contract anchored to a verified `WorldHeadProofV1`, and `oasis7_world_head_proof_verify --state-receipt-proof` validates its head-proof hash, state/receipt root, leaf hash, ordered proof path, and claim boundary outside the node process.
- `WorldFinalityProofV1` now defines a bounded validator-set finality, stake-threshold, committed-head binding, and fork/reorg/misbehavior evidence contract anchored to verified `WorldHeadProofV1` windows.
- `oasis7_world_head_proof_verify --finality-proof` validates the bounded finality proof contract outside the node process.
- `network-tier-external-verifier-light-client-lite.sh` emits operator evidence and binds it to manifest/genesis/bootstrap/RPC/sample-window context.
- `external_verifier_light_client_lite_ready` is validated by readiness tooling as an optional non-promotional lane.
- `oasis7_world_head_proof_verify --proof-window` validates contiguous `WorldHeadProofV1` windows with trusted-anchor, height, prev-hash, world-id, observed-head, timestamp, and basic quorum fail-closed checks.
- `network-tier-light-client-continuity-window.sh` emits optional `light_client_continuity_window_ready` evidence and readiness tooling validates it as non-promotional `ignored_lanes` evidence.
- `state_resource_receipt_proof_ready` is validated by readiness tooling as an optional non-promotional `ignored_lanes` evidence lane when present.
- `validator_finality_proof_ready` is validated by readiness tooling as an optional non-promotional `ignored_lanes` evidence lane when present.
- `public-testnet-current-required-lanes-2026-07-03.tsv` now covers all 11 active required lanes with pass evidence, and `network-tier-public-testnet-readiness.sh` returns `gate_result=pass`, `readiness_verdict=ready_for_live_candidate`, and `live_candidate_allowed=true` for controlled public-testnet live-candidate claims.

This closes a bounded continuity/auditability gap, the current controlled public-testnet live-candidate readiness gap, the minimal state/resource/receipt proof contract/verifier gap, bounded validator-set finality/fork-misbehavior evidence semantics, live Ed25519 finality vote verification, and bounded validator-set transition execution semantics. It does not close public launch, mainnet-grade release, full light-client, trust-minimized validator governance/transition security, live arbitrary runtime proof emission, full state/receipt index completeness, or multi-client-equivalence gaps.

Residual risk remains explicit: the governed-bootstrap manifest still records `status="rehearsal"`, the bundle provenance records `git_worktree_dirty=true`, and faucet evidence is guarded/cooldown/plain-HTTP testnet evidence rather than durable anti-abuse, WAF/TLS, or production faucet operations.

## Current Oasis7 Posture

| Area | Current posture | Evidence | Verdict |
| --- | --- | --- | --- |
| World-state substrate framing | P2P is no longer treated as the whole milestone; current target is the chain-backed large-world state substrate. | `doc/p2p/project.md`, `testing-manual.md` S9A | `stronger_than_transport_only` |
| Head proof artifact | `WorldHeadProofV1` binds committed head / execution context / optional checkpoint closure. | `crates/oasis7_proto`, chain proof template | `present` |
| External verifier | Independent binary + operator wrappers validate sampled proof contract/hash/world/height/observed head, contiguous proof-window continuity, bounded state/resource/receipt proof contract evidence, and bounded validator-set finality/fork-misbehavior evidence. | `crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs`, `scripts/network-tier-external-verifier-light-client-lite.sh`, `scripts/network-tier-light-client-continuity-window.sh`, `scripts/network-tier-validator-finality-proof.sh` | `present_continuous_light_client_lite_plus_bounded_state_receipt_finality` |
| Readiness gate integration | Proof lanes are validated but optional and non-promotional. | `scripts/network-tier-public-testnet-readiness.sh`, readiness lane templates | `present_non_promotional` |
| Formal public testnet | Current 11-lane TSV is complete and all-pass; readiness script allows controlled `ready_for_live_candidate` claims. | formal network-tier runbook/project, `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv` | `ready_for_live_candidate_controlled` |
| Release train / live network closure | 11-lane live-candidate evidence is present, but public launch / validator admission / mainnet release train remain separately gated. | formal network-tier runbook/project, lane evidence history | `gap_medium_high` |
| Full light client | Continuous proof-window, bounded state/receipt proof evidence, bounded validator-set finality/fork-misbehavior evidence, live Ed25519 finality vote verification, and bounded validator-set transition execution semantics exist, but trust-minimized validator governance/transition security, live arbitrary proof emission, and independent client parity remain missing. | verifier/window/state-receipt/finality lanes + role slices | `gap_medium_high` |
| State/resource/receipt proof | Minimal proof contract and verifier coverage exist for sampled resource/query/receipt inclusion or absence evidence anchored to `WorldHeadProofV1`; live runtime emission and full state/receipt index completeness remain open. | `WorldStateReceiptProofV1`, verifier tests, optional readiness lane | `bounded_contract_present_live_emission_gap` |
| Multi-client equivalence | External process is not an independent implementation/client diversity strategy. | runtime/producer slices | `gap_medium_high` |

## Mainstream Benchmark Frame

The current external benchmark is based on official documentation for public-chain operations and trust surfaces:

- Ethereum official docs describe nodes/clients, light clients, client diversity, networks/testnets, and execution spec tests.
- Solana official docs distinguish clusters and RPC commitment/API behavior.
- Cosmos/IBC docs frame light clients as a core verification surface for cross-chain trust.
- Polkadot docs frame light clients as a trust-minimized way to verify chain state.
- Avalanche docs frame validator/node operation and staking uptime as operator readiness concerns.

These chains differ architecturally, but their mature infrastructure posture generally includes:

1. Public network surfaces: stable public RPC, explorer, faucet/testnet policy, reset/mainnet value boundaries.
2. Node/operator readiness: documented validator/full-node operation, monitoring, upgrades, recovery, uptime expectations.
3. Trust-minimized verification: light clients, header/finality verification, state/proof validation, or equivalent checkpoint/finality paths.
4. Client/spec confidence: independent clients or spec test suites/differential validation that reduce single-implementation risk.
5. Release-train maturity: rehearsal networks, testnets, drills, chaos/negative tests, and claim boundaries.

## Gap Matrix

| Rank | Gap | Why it matters | Current Oasis7 state | Recommended next action |
| --- | --- | --- | --- | --- |
| P0 | Formal `public_testnet` 11-lane all-pass readiness | Mainstream public chains present a reachable, current, operator-safe public surface before stronger claims. | Closed for controlled live-candidate readiness: the current TSV has 11/11 required lanes pass and readiness returns `ready_for_live_candidate`. | Keep evidence fresh during release train and do not convert it into public launch/mainnet claims. |
| P0 | Same-world hosted entry + API/viewer projection | Player-facing and API-facing views must read the same formal world state, not a local smoke world or copied checkpoint. | Closed for the current readiness packet by same-window JSON evidence for API/viewer and same-world hosted entry. | Re-sample before any external public launch or release train decision. |
| P0 | Runtime bootstrap and freshness re-validation | Historical endpoint/faucet evidence was later constrained by freshness/runtime drift findings. | Closed for the current readiness packet by run142 runtime/world-resource closure, public surface freshness, and faucet guard evidence. | Keep this as a freshness-sensitive release-train gate, not a permanent proof. |
| P1 | Continuous light-client-lite to trust-minimized light client | Mainstream light-client posture is not a sampled proof hash; it needs continuity/finality/transition semantics. | Continuous proof-window verifier now checks anchor, height continuity, prev-hash linkage, world identity, observed head, timestamps, and basic quorum fail-closed cases; `WorldFinalityProofV1` adds bounded validator-set, stake-threshold, committed-head binding, live Ed25519 finality vote verification, bounded validator-set transition execution, and fork/reorg/misbehavior evidence semantics; both remain optional/non-promotional evidence. | Add trust-minimized validator governance/transition security and independent spec/replay parity before claiming trust-minimized light-client equivalence. |
| P1 | State/resource/receipt proof contract | Head proof alone cannot prove a concrete account/resource/query/receipt to an external consumer. | Closed for bounded contract/verifier scope: `WorldStateReceiptProofV1` validates resource/query/receipt subjects, leaf hash, ordered proof path, and state/receipt root binding against a verified `WorldHeadProofV1`; optional `state_resource_receipt_proof_ready` evidence remains non-promotional. | Add live runtime emission, operator sampling on real artifacts, full state/receipt index completeness, and property/fuzz pressure before claiming broad arbitrary state-proof availability. |
| P1 | Fault/negative/release-train drills | Mature public chains prove operational resilience via rehearsals, faults, and recovery evidence. | Existing runbooks and readiness scripts are strong, but current live rehearsal evidence is incomplete. | Run network rehearsal/release-train drill with clean bootstrap, rollback/restore, fork/freshness negative cases, and evidence writeback. |
| P2 | Fuzz/property gate | Mainstream-grade testing expects invariant/property pressure beyond deterministic examples. | Prior benchmark already marked fuzz/property gate as missing. | Define first property targets around proof contract validation, state/replay determinism, and readiness fail-closed behavior. |
| P2 | Multi-client / independent implementation parity | Mature ecosystems reduce correlated implementation bugs through client diversity or spec tests. | External verifier is separate process but same implementation family. | Start with independent replay/verifier spec tests before considering a second client. |

## Claim Envelope

Allowed:

- `limited playable technical preview`
- `crypto-hardened preview`
- `formal public_testnet mechanism is documented`
- `current required-lane packet is complete`
- `all 11 formal public_testnet required lanes have pass evidence`
- `controlled public_testnet live-candidate claim is allowed by the script-generated readiness review`
- `the network remains resettable, non-mainnet, and guarded-faucet bounded`
- `WorldHeadProofV1 can be externally verified as light-client-lite sampled evidence`
- `contiguous WorldHeadProofV1 windows can be externally verified as bounded continuity evidence`
- `WorldStateReceiptProofV1 can be externally verified as bounded sampled state/resource/receipt inclusion evidence anchored to a committed world head`
- `WorldFinalityProofV1 can be externally verified as bounded validator-set finality and fork-misbehavior evidence anchored to committed world heads`
- `WorldFinalityProofV1 verifies live Ed25519 finality vote signatures and bounded validator-set transition execution semantics when those transitions are present`
- `legacy shared_devnet is legacy/rehearsal evidence, not the target public_testnet`

Forbidden:

- `live public testnet is already online`
- `unrestricted public faucet is open`
- `public validator admission is open`
- `mainnet-grade`
- `mainnet_live`
- `production OC settlement`
- `mainstream public-chain-grade security/testing`
- `full light client security`
- `trust-minimized validator-set transition execution`
- `live runtime arbitrary state proof availability`
- `full state/receipt index completeness`
- `multi-client consensus equivalence`

## Integrated Professional Findings

`runtime_engineer`:

- The proof posture improved materially after Gap 3.
- The verifier path now covers sampled proof, contiguous proof-window evidence, bounded state/resource/receipt proof contract evidence, and bounded validator-set finality/fork-misbehavior evidence semantics.
- It still lacks trust-minimized validator governance/transition security, live arbitrary proof emission, DA sampling, and multi-client equivalence.
- The next technical steps are trust-minimized validator governance/transition security, live state/resource/receipt proof emission and sampling, and independent spec/replay parity.

`blockchain_ops_engineer`:

- Formal network-tier 11-lane readiness is currently all-pass for controlled live-candidate evidence.
- Existing governed-bootstrap/live evidence satisfies the current 11 required lanes, as verified by `network-tier-public-testnet-readiness.sh`.
- Ops residual risks remain: manifest `status="rehearsal"`, bundle `git_worktree_dirty=true`, guarded faucet cooldown/plain-HTTP limits, and no public launch/mainnet/validator-admission claim.
- `light_client_continuity_window_ready`, `state_resource_receipt_proof_ready`, and `validator_finality_proof_ready` are valid optional evidence lanes but must remain non-promotional until the required lanes and stronger light-client/runtime emission semantics are complete.
- The next ops step is to keep live-candidate evidence fresh through release-train drills and avoid treating optional proof lanes as promotion gates.

`producer_system_designer`:

- Product positioning can be raised to auditability-enhanced preview.
- It cannot be raised to mainstream public-chain readiness.
- Public network product closure has higher priority than expanding claims around the sampled proof lane.

`qa_engineer`:

- Verification maturity is stronger and the current required-lane packet is all-pass.
- Sampled proof, proof-window verifier, bounded state/resource/receipt proof, and bounded validator/finality proof lanes are optional ignored lanes and cannot promote readiness.
- API/viewer same-window projection and same-world hosted entry are closed for the current packet, but must be re-sampled before public launch/release-train promotion.

`liveops_community`:

- Operator/community risk is misreading reachable endpoint/faucet evidence as an opened public testnet.
- Any external wording must keep `controlled live-candidate`, `resettable`, `guarded faucet`, `non-mainnet`, `not validator-open`, and `not live public launch` visible.

## Recommended Execution Order

1. Keep the current all-pass formal `public_testnet` 11-lane packet fresh during any release-train or public launch decision.
2. Re-sample public RPC / explorer / guarded faucet / runtime status / hosted/API/viewer evidence immediately before external claims.
3. Run the optional external verifier lanes on real sampled, contiguous-window, state/resource/receipt, and validator/finality `WorldHeadProofV1`-anchored evidence; keep them non-promotional.
4. Continue the light-client-lite upgrade: trust-minimized validator governance/transition security, live state/resource/receipt proof emission and sampling, and independent spec/replay parity.
5. Add fault/negative/release-train drill and fuzz/property targets now that the 11-lane public-testnet blocker is closed for controlled live-candidate readiness.

## External References

- Ethereum nodes and clients: https://ethereum.org/en/developers/docs/nodes-and-clients/
- Ethereum light clients: https://ethereum.org/en/developers/docs/nodes-and-clients/light-clients/
- Ethereum client diversity: https://ethereum.org/en/developers/docs/nodes-and-clients/client-diversity/
- Ethereum networks and testnets: https://ethereum.org/en/developers/docs/networks/
- Ethereum execution spec tests: https://eest.ethereum.org/main/
- Solana clusters: https://solana.com/docs/references/clusters
- Solana RPC: https://solana.com/docs/rpc
- Cosmos IBC light clients: https://docs.cosmos.network/ibc/latest/light-clients/developer-guide/overview
- Polkadot light clients: https://docs.polkadot.com/reference/tools/light-clients/
- Avalanche Primary Network validation: https://docs.avax.network/docs/tooling/avalanche-cli/create-avalanche-nodes/validate-primary-network
