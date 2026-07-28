# Node identity and replication contract project record

- PRD: `doc/p2p/node/node-identity-replication-contract.prd.md`
- Design: `doc/p2p/node/node-identity-replication-contract.design.md`

## 任务拆解

- [x] node-identity-replication-signer-binding (PRD-P2P-MIG-088) [test_tier_required]: absorbed validator signer binding, apply-before-observe replication ingest, and explicit PoS recovery failure. Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)
- [x] node-identity-replication-distfs (PRD-P2P-MIG-092) [test_tier_required]: absorbed signed DistFS replication, guard persistence, stale or duplicate rejection, and recovery boundary. Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)
- [x] node-identity-replication-keypair-bootstrap (PRD-P2P-MIG-095) [test_tier_required]: absorbed local config keypair bootstrap and the explicit invalid or unwritable-config failure boundary. Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)
- [x] node-identity-replication-network-injection (PRD-P2P-MIG-099) [test_tier_required]: absorbed node replication network injection and world/topic isolation without making UDP fallback or `aw.*` names current truth. Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)
- [x] node-identity-replication-contract-consolidation (PRD-P2P-MIG-088/092/095/099) [test_tier_required]: established the stable authority and retired historical source triplets after shared index/route repair. Trace: #2684 (task_466ecbb2e1ab4e79915c58de7e95dd78)

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
