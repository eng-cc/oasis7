# Oasis7 Current Mainstream Public-Chain Gap Benchmark

- Task: GitHub issue #1842 / `task_16ff5f37bfb94220a6349fb4c9304bcc`
- Date: 2026-07-03
- Owner: `tpm` integration
- Professional slices: `runtime_engineer`, `blockchain_ops_engineer`, `producer_system_designer`, `qa_engineer`, `liveops_community`
- Status: current-state research / no release promotion

## Executive Verdict

After the 2026-07-06 proof-window continuity slice, Oasis7 should be assessed as:

`crypto-hardened preview with externally verifiable sampled and continuous world-head evidence`

It should not be assessed as:

`mainstream public-chain-ready`, `ready_for_live_candidate`, `full light client`, `mainnet-grade`, or `multi-client consensus equivalent`.

The main changes since the previous benchmark are that Oasis7 now has concrete external verifier lanes:

- `WorldHeadProofV1` proof artifacts exist for committed world-head evidence.
- `oasis7_world_head_proof_verify` can validate sampled proof artifacts outside the node process.
- `network-tier-external-verifier-light-client-lite.sh` emits operator evidence and binds it to manifest/genesis/bootstrap/RPC/sample-window context.
- `external_verifier_light_client_lite_ready` is validated by readiness tooling as an optional non-promotional lane.
- `oasis7_world_head_proof_verify --proof-window` validates contiguous `WorldHeadProofV1` windows with trusted-anchor, height, prev-hash, world-id, observed-head, timestamp, and basic quorum fail-closed checks.
- `network-tier-light-client-continuity-window.sh` emits optional `light_client_continuity_window_ready` evidence and readiness tooling validates it as non-promotional `ignored_lanes` evidence.

This closes a bounded continuity/auditability gap. It does not close the public network readiness, full light-client, validator-set transition/finality proof, state/receipt proof, or multi-client-equivalence gaps.

## Current Oasis7 Posture

| Area | Current posture | Evidence | Verdict |
| --- | --- | --- | --- |
| World-state substrate framing | P2P is no longer treated as the whole milestone; current target is the chain-backed large-world state substrate. | `doc/p2p/project.md`, `testing-manual.md` S9A | `stronger_than_transport_only` |
| Head proof artifact | `WorldHeadProofV1` binds committed head / execution context / optional checkpoint closure. | `crates/oasis7_proto`, chain proof template | `present` |
| External verifier | Independent binary + operator wrappers validate sampled proof contract/hash/world/height/observed head and contiguous proof-window continuity. | `crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs`, `scripts/network-tier-external-verifier-light-client-lite.sh`, `scripts/network-tier-light-client-continuity-window.sh` | `present_continuous_light_client_lite` |
| Readiness gate integration | Proof lanes are validated but optional and non-promotional. | `scripts/network-tier-public-testnet-readiness.sh`, readiness lane template | `present_non_promotional` |
| Formal public testnet | Current truth remains `block` / governed-bootstrap rehearsal / not `ready_for_live_candidate`. | formal network-tier runbook/project | `blocked` |
| Release train / live network closure | Missing current all-pass 11-lane TSV and same-window live evidence. | formal network-tier runbook/project, lane evidence history | `gap_high` |
| Full light client | Continuous proof-window evidence exists, but finality signatures, validator-set transition, state/receipt inclusion, and independent client parity remain missing. | verifier/window lane + role slices | `gap_medium_high` |
| State/resource/receipt proof | Current proof reaches head/state-root binding, not arbitrary query/resource/receipt verification. | runtime slice | `gap_high` |
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
| P0 | Formal `public_testnet` 11-lane all-pass readiness | Mainstream public chains present a reachable, current, operator-safe public surface before stronger claims. | Current formal verdict remains `block`; governed-bootstrap is rehearsal; current TSV/evidence are not all-pass. | Build a non-template TSV for all 11 required lanes and run `network-tier-public-testnet-readiness.sh` against current governed-bootstrap truth. |
| P0 | Same-world hosted entry + API/viewer projection | Player-facing and API-facing views must read the same formal world state, not a local smoke world or copied checkpoint. | Required lanes exist, but current all-pass evidence is missing. | Produce same-window JSON evidence for hosted-login / launcher / viewer / pure API against the same formal `public_testnet` world state. |
| P0 | Runtime bootstrap and freshness re-validation | Historical endpoint/faucet evidence was later constrained by freshness/runtime drift findings. | Historical positive evidence exists, but 2026-05-22 evidence made public RPC/explorer/faucet partial and runtime bootstrap block. | Re-run clean governed bootstrap and same-window public RPC/explorer/faucet/status samples. |
| P1 | Continuous light-client-lite to trust-minimized light client | Mainstream light-client posture is not a sampled proof hash; it needs continuity/finality/transition semantics. | Continuous proof-window verifier now checks anchor, height continuity, prev-hash linkage, world identity, observed head, timestamps, and basic quorum fail-closed cases; it remains optional/non-promotional evidence. | Add validator-set transition, finality signature/stake semantics, fork/reorg/misbehavior evidence, and independent spec/replay parity before claiming trust-minimized light-client equivalence. |
| P1 | State/resource/receipt proof contract | Head proof alone cannot prove a concrete account/resource/query/receipt to an external consumer. | Current artifact binds head/state root but does not prove arbitrary state/query inclusion. | Add minimal resource/state/receipt proof contract and verifier coverage. |
| P1 | Fault/negative/release-train drills | Mature public chains prove operational resilience via rehearsals, faults, and recovery evidence. | Existing runbooks and readiness scripts are strong, but current live rehearsal evidence is incomplete. | Run network rehearsal/release-train drill with clean bootstrap, rollback/restore, fork/freshness negative cases, and evidence writeback. |
| P2 | Fuzz/property gate | Mainstream-grade testing expects invariant/property pressure beyond deterministic examples. | Prior benchmark already marked fuzz/property gate as missing. | Define first property targets around proof contract validation, state/replay determinism, and readiness fail-closed behavior. |
| P2 | Multi-client / independent implementation parity | Mature ecosystems reduce correlated implementation bugs through client diversity or spec tests. | External verifier is separate process but same implementation family. | Start with independent replay/verifier spec tests before considering a second client. |

## Claim Envelope

Allowed:

- `limited playable technical preview`
- `crypto-hardened preview`
- `formal public_testnet mechanism is documented`
- `governed-bootstrap evidence is rehearsal / not ready_for_live_candidate`
- `WorldHeadProofV1 can be externally verified as light-client-lite sampled evidence`
- `contiguous WorldHeadProofV1 windows can be externally verified as bounded continuity evidence`
- `legacy shared_devnet is legacy/rehearsal evidence, not the target public_testnet`

Forbidden:

- `live public testnet is already online`
- `public faucet is open`
- `public validator admission is open`
- `ready_for_live_candidate`
- `mainnet-grade`
- `mainnet_live`
- `production OC settlement`
- `mainstream public-chain-grade security/testing`
- `full light client security`
- `multi-client consensus equivalence`

## Integrated Professional Findings

`runtime_engineer`:

- The proof posture improved materially after Gap 3.
- The verifier path now covers sampled proof and contiguous proof-window evidence.
- It still lacks validator-set transition, finality signature/stake semantics, state/receipt proof, DA sampling, and multi-client equivalence.
- The next technical step is validator/finality transition semantics plus state/resource/receipt proof contracts.

`blockchain_ops_engineer`:

- Formal network-tier remains `block`.
- Existing governed-bootstrap/live evidence does not satisfy the current 11 required lanes.
- `light_client_continuity_window_ready` is valid optional evidence but must remain non-promotional until the required lanes and stronger light-client semantics are complete.
- The next ops step is to target governed-bootstrap truth, complete the 11-lane TSV, and re-run same-window public endpoint/world-status sampling.

`producer_system_designer`:

- Product positioning can be raised to auditability-enhanced preview.
- It cannot be raised to mainstream public-chain readiness.
- Public network product closure has higher priority than expanding claims around the sampled proof lane.

`qa_engineer`:

- Verification maturity is stronger but still partial.
- Sampled proof and proof-window verifier lanes are optional ignored lanes and cannot promote readiness.
- API/viewer same-window projection and same-world hosted entry are key release-grade blockers.

`liveops_community`:

- Operator/community risk is misreading reachable endpoint/faucet evidence as an opened public testnet.
- Any external wording must keep `rehearsal`, `resettable`, `guarded faucet`, `non-mainnet`, `not validator-open`, and `not ready_for_live_candidate` visible.

## Recommended Execution Order

1. Generate a current formal `public_testnet` lanes TSV covering all 11 required lanes.
2. Re-run readiness against current governed-bootstrap manifest/bundle/genesis/bootstrap truth.
3. Fill `same_world_hosted_entry_ready` and `api_viewer_projection_ready` with same-window JSON evidence.
4. Re-sample public RPC / explorer / guarded faucet / status endpoints after clean governed bootstrap.
5. Run the optional external verifier lanes on real sampled and contiguous-window `WorldHeadProofV1` evidence; keep them non-promotional.
6. Continue the light-client-lite upgrade: validator-set transition, finality/fork/misbehavior boundaries, state/resource/receipt proof, and independent spec/replay parity.
7. Add fault/negative/release-train drill and fuzz/property targets after the current 11-lane public-testnet blocker is made explicit.

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
