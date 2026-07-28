# Node identity and replication contract

- Design: `doc/p2p/node/node-identity-replication-contract.design.md`
- Project record: `doc/p2p/node/node-identity-replication-contract.project.md`

## 目标与权威边界

This is the current `node/` authority for local node-identity bootstrap,
validator-to-signer binding, and signed replication ingestion. It absorbs the
completed `PRD-P2P-MIG-088`, `PRD-P2P-MIG-092`, `PRD-P2P-MIG-095`, and
`PRD-P2P-MIG-099` triplets. Historical implementation detail remains traceable
through Git history and GitHub task evidence, not through a second live
authority.

The contract covers the node-runtime configuration and replication boundary. It
does not define consensus or governance admission, reward settlement, production
custody, public-network deployment, topology health, or QA release readiness.

## 范围

This scope is limited to the node-side identity and replication contracts
listed below. It does not widen the authority to consensus, governance,
settlement, deployment, or release decisions.

## Current contract

### 接口 / 数据：identity and signer binding

- A local development node may ensure a missing node keypair in its current
  config path before it starts. Disabled-node flows do not bootstrap identity.
- Missing, malformed, conflicting, or unwritable configuration fails with an
  explicit diagnostic. It must not silently replace an existing invalid identity.
- When `validator_signer_public_keys` is configured, it binds every configured
  validator ID to one normalized 32-byte ed25519 public key. The map must cover
  the validator set, contain no unknown validator, and reject invalid keys.
- For proposal, attestation, and commit messages, normal signature verification
  precedes the enabled validator-to-signer binding check. A missing or mismatched
  public key is rejected.

The local config identity is a development/bootstrap convenience only. It is not
a production keystore, KMS/HSM custody mechanism, rotation or revocation
workflow, validator admission, governance signer, mainnet claim, or readiness
verdict. Private keys, seeds, mnemonics, and complete secret environments must
not enter documentation, inventory, monitor artifacts, or evidence.

### 接口 / 数据：replication and recovery

- `NodeRuntime` can receive a `NodeReplicationNetworkHandle`; its replication
  path supports network injection and world/topic isolation.
- Signed `FileReplicationRecord` ingestion validates the applicable identity and
  source boundary, applies/persists successfully, and only then advances
  replication observation such as peer heads or committed progress. Errors remain
  observable; a failed apply must not be represented as a progressed peer.
- Local and remote single-writer guards, record ordering, and persisted guard
  state protect against stale or duplicate remote application. Invalid records
  do not enter local state.
- Corrupt or unreadable PoS recovery state blocks node startup rather than
  selecting a default state. A restart is diagnostic or temporary recovery, not
  a substitute for correcting the state, configuration, or deployment root cause.

## Transport and operational boundary

The runtime injection abstraction is not evidence that a libp2p deployment,
NAT traversal, public reachability, peer inventory, or mainnet-grade network
health exists. Transport labels do not establish topology truth: health and role
must be read from the current runtime status/evidence window. Current real-triad
sampling and bounded operator evidence remain under
`node-triad-operations-observability.*`.

The historical UDP fallback and `aw.<world_id>.replication` topic spelling are
not current transport authority. They may be inspected in history only unless a
future code-backed contract explicitly restores them. This document does not add
PKI distribution, multi-writer CRDT, DHT/Kad indexing, automatic deployment,
state sync, restore, rollback, or release automation.

## 里程碑

- M1: Four completed historical triplets are absorbed into this stable node
  contract without carrying their obsolete transport wording forward.
- M2: Shared routes and the file index are repaired by the integration owner,
  then the retired basenames pass the frozen-head stale-reference gate.
- M3: Runtime and QA owners retain targeted verification of signer binding,
  ingest ordering, recovery, and duplicate/stale replication handling.

## 风险

- A local bootstrap key can be mistaken for a custody or governance signer; the
  non-custody boundary must remain explicit.
- Transport labels, UDP fallback, or historical `aw.*` topics can be mistaken
  for a current deployed topology; they are not current authority.
- Restarting after corruption can hide the real config/state/deployment defect;
  startup failure must remain observable and fail closed.

## Verification and failure posture

Targeted node verification must cover signer-map rejection, missing/mismatched
signer rejection, apply-before-observe ingest behavior, corrupt-state startup
failure, duplicate/stale record rejection, and persisted replication recovery.
The specific command set is selected by the runtime and QA owners; documentation
migration itself does not claim runtime, integration, or release success.

For this authority migration, run:

```sh
./scripts/doc-governance-check.sh
./scripts/readme-link-check.sh
git diff --check
```

## Explicit non-claims

- A valid local keypair or signer map is not custody, governance truth, or a
  production/mainnet readiness signal.
- Signed replication is not a public-chain, topology-health, settlement, or QA
  release verdict.
- This document does not supersede runtime consensus semantics, the formal
  network-tier contract, or the mainnet security/governance authority.
