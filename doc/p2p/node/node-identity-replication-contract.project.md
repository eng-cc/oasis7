# Node identity and replication contract project record

- PRD: `doc/p2p/node/node-identity-replication-contract.prd.md`
- Design: `doc/p2p/node/node-identity-replication-contract.design.md`

## 任务拆解

- [x] `PRD-P2P-MIG-088`: validator signer binding, apply-before-observe
  replication ingest, and explicit PoS recovery failure absorbed.
- [x] `PRD-P2P-MIG-092`: signed DistFS replication, guard persistence, stale or
  duplicate rejection, and recovery boundary absorbed.
- [x] `PRD-P2P-MIG-095`: local config keypair bootstrap and explicit invalid or
  unwritable-config failure boundary absorbed.
- [x] `PRD-P2P-MIG-099`: node replication network injection and world/topic
  isolation absorbed without making UDP fallback or `aw.*` names current truth.
- [x] `node-identity-replication-contract-consolidation`
  (`task_466ecbb2e1ab4e79915c58de7e95dd78`): established the stable authority;
  historical source triplets are retired after shared index/route repair.

## 依赖

- Runtime semantics and tests: `crates/oasis7_node`,
  `crates/oasis7_distfs`, and `oasis7_chain_runtime`.
- Network topology/reachability: `doc/p2p/network/mainnet-private-reachability-architecture.*`.
- Mainnet custody/governance/readiness: `doc/p2p/blockchain/p2p-mainnet-security-governance-readiness.*`.
- Real triad operator samples: `doc/p2p/node/node-triad-operations-observability.*`.

## Current verification entry

- `./scripts/doc-governance-check.sh`
- `./scripts/readme-link-check.sh`
- Targeted runtime tests selected by runtime and QA for signer binding,
  replication ingest ordering, persisted guard recovery, and corrupt-state
  startup failure.

## 状态

This project record does not claim deployed libp2p end-to-end operation,
public-network health, automatic recovery, production custody, or a release
verdict. Those claims require their own current evidence and owners.
