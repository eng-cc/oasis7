# Oasis7 Current Mainstream Public-Chain Gap Benchmark

Lifecycle: historical 2026-07-03/06 benchmark snapshot. Every present-tense posture, lane count, `pass`, and `ready_for_live_candidate` statement below is window-bound research evidence, not current network health, public availability, release, or launch authority; re-sample through the formal network-tier runbook before reuse.

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
- `public-testnet-release-train-drill.sh` now composes the positive 11-lane readiness packet, freshness/fork-like/readiness/unsupported-promotion-gate fail-closed negative packets, and the `public_testnet_rehearsal` aggregate gate into one reproducible drill summary with `public_launch_allowed=false`.

This closes a bounded continuity/auditability gap, the current controlled public-testnet live-candidate readiness gap, the release-train drill/gate behavior gap, the minimal state/resource/receipt proof contract/verifier gap, bounded validator-set finality/fork-misbehavior evidence semantics, live Ed25519 finality vote verification, bounded validator-set transition execution semantics, and bounded governance-authenticated validator-set transition evidence against a trusted governance-set anchor. It does not close public launch, real rollback incident evidence, mainnet-grade release, full light-client, live arbitrary runtime proof emission, full state/receipt index completeness, or multi-client-equivalence gaps.

Residual risk remains explicit: the governed-bootstrap manifest still records `status="rehearsal"`, the bundle provenance records `git_worktree_dirty=true`, and faucet evidence is guarded/cooldown/plain-HTTP testnet evidence rather than durable anti-abuse, WAF/TLS, or production faucet operations.

## Current Oasis7 Posture

| Area | Current posture | Evidence | Verdict |
| --- | --- | --- | --- |
| World-state substrate framing | P2P is no longer treated as the whole milestone; current target is the chain-backed large-world state substrate. | GitHub Issue / GitHub Project, `testing-manual.md` S9A | `stronger_than_transport_only` |
| Head proof artifact | `WorldHeadProofV1` binds committed head / execution context / optional checkpoint closure. | `crates/oasis7_proto`, chain proof template | `present` |
| External verifier | Independent binary + operator wrappers validate sampled proof contract/hash/world/height/observed head, contiguous proof-window continuity, bounded state/resource/receipt proof contract evidence, and bounded validator-set finality/fork-misbehavior evidence. | `crates/oasis7_proto/src/bin/oasis7_world_head_proof_verify.rs`, `scripts/network-tier-external-verifier-light-client-lite.sh`, `scripts/network-tier-light-client-continuity-window.sh`, `scripts/network-tier-validator-finality-proof.sh` | `present_continuous_light_client_lite_plus_bounded_state_receipt_finality` |
| Readiness gate integration | Proof lanes are validated but optional and non-promotional. | `scripts/network-tier-public-testnet-readiness.sh`, readiness lane templates | `present_non_promotional` |
| Formal public testnet | Current 11-lane TSV is complete and all-pass; readiness script allows controlled `ready_for_live_candidate` claims. | formal network-tier runbook/project, `doc/testing/evidence/public-testnet-current-required-lanes-2026-07-03.tsv` | `ready_for_live_candidate_controlled` |
| Release train / live network closure | 11-lane live-candidate evidence is present, and the release-train drill wrapper now proves positive readiness plus fail-closed freshness/fork-like/readiness/promotion-gate negatives in a reproducible local gate smoke. Public launch, live rollback incident evidence, validator admission, and mainnet release train remain separately gated. | `scripts/public-testnet-release-train-drill.sh`, formal network-tier runbook/project, lane evidence history | `drill_behavior_present_live_launch_gap` |
| Full light client | Continuous proof-window, bounded state/receipt proof evidence, bounded validator-set finality/fork-misbehavior evidence, live Ed25519 finality vote verification, bounded validator-set transition execution semantics, and trusted governance-set anchored transition evidence exist, but live arbitrary proof emission and independent client parity remain missing. | verifier/window/state-receipt/finality lanes + role slices | `gap_medium` |
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
| P1 | Continuous light-client-lite to trust-minimized light client | Mainstream light-client posture is not a sampled proof hash; it needs continuity/finality/transition semantics. | Continuous proof-window verifier now checks anchor, height continuity, prev-hash linkage, world identity, observed head, timestamps, and basic quorum fail-closed cases; `WorldFinalityProofV1` adds bounded validator-set, stake-threshold, committed-head binding, live Ed25519 finality vote verification, bounded validator-set transition execution, trusted governance-set anchored transition evidence, and fork/reorg/misbehavior evidence semantics; both remain optional/non-promotional evidence. | Add independent spec/replay parity and live arbitrary proof emission before claiming full trust-minimized light-client equivalence. |
| P1 | State/resource/receipt proof contract | Head proof alone cannot prove a concrete account/resource/query/receipt to an external consumer. | Closed for bounded contract/verifier scope: `WorldStateReceiptProofV1` validates resource/query/receipt subjects, leaf hash, ordered proof path, and state/receipt root binding against a verified `WorldHeadProofV1`; optional `state_resource_receipt_proof_ready` evidence remains non-promotional. | Add live runtime emission, operator sampling on real artifacts, full state/receipt index completeness, and property/fuzz pressure before claiming broad arbitrary state-proof availability. |
| P1 | Fault/negative/release-train drills | Mature public chains prove operational resilience via rehearsals, faults, and recovery evidence. | Reproducible local release-train drill exists: it asserts positive 11-lane readiness, consumes freshness/fork-like/readiness/unsupported-promotion-gate negative packets through the readiness gate, writes aggregate `public_testnet_rehearsal` gate refs in smoke or from operator-supplied rehearsal lane evidence, and preserves `public_launch_allowed=false`. Live rehearsal, clean bootstrap, and real rollback/restore evidence are still incomplete. | Run the drill with operator-supplied immutable runtime/world/governance artifacts plus explicit `public_testnet_rehearsal` lane evidence during the next live candidate window, then add real rollback/restore evidence writeback. |
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
- `WorldFinalityProofV1 verifies bounded governance-authenticated validator-set transition evidence against a trusted governance-set anchor when transitions are present; this transition-governance semantic change uses proof schema/hash domain v2 and rejects legacy schema-1 transition artifacts`
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
- `live runtime arbitrary state proof availability`
- `full state/receipt index completeness`
- `multi-client consensus equivalence`

## 历史角色回报与执行建议来源

当次角色回报与执行顺序见 [Task #1842 历史记录](https://github.com/eng-cc/oasis7/issues/1842#issuecomment-6162414656)。本文研究方法、固定窗口结论和 gap 矩阵继续保留；不将旧 all-pass 改写为当前 readiness。实际对外判断须重取同候选公开入口、runtime、hosted/API/Viewer 与 readiness evidence，optional proof lanes 不作为 promotion gates。

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

<!-- Historical section locators; execution details are in the linked original Task/PR. -->
<a id="integrated-professional-findings"></a>
<a id="recommended-execution-order"></a>
